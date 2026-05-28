#!/usr/bin/env python3
"""
Fill missing manufacturer printing params by extracting the TEXT layer of each
TDS PDF already linked in filaments.sqlite. Most manufacturer TDS are real text
PDFs (Eryone/BASF/3DXTech/...), so direct extraction is instant and more
accurate than vision OCR, and it even captures the authoritative test-specimen
("following conditions") note. PDFs with NO usable text are reported as
image-only -> genuine Ollama vision-OCR candidates.

Parser ported faithfully from the app's tds.rs regexes so the DB matches what
the companion app would extract from the same sheet. Output:
tools/sds-importer/data/scrape_out/cluster_tds_extract.json  (merged by
merge_scrape_out.py; params attach to existing materials, idempotent).

Usage:
  python scripts/tds_text_extract.py            # full batch over null-param TDS
  python scripts/tds_text_extract.py --url URL  # one-off test on a single PDF
"""
from __future__ import annotations
import argparse, hashlib, json, re, sqlite3, sys, tempfile, urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DB = REPO / "tools" / "sds-importer" / "data" / "filaments.sqlite"
OUT = REPO / "tools" / "sds-importer" / "data" / "scrape_out" / "cluster_tds_extract.json"
CACHE = Path(tempfile.gettempdir()) / "tds_cache"
CACHE.mkdir(exist_ok=True)

# ---- regexes ported from tds.rs -------------------------------------------
RANGE_RX = re.compile(r"(?i)(\d{2,3}(?:\.\d+)?)\s*(?:°\s*[cf]|℃|℉|°)?\s*(?:-|to|–|~|à|au)\s*(\d{2,3}(?:\.\d+)?)")
SPEED_UNIT_RX = re.compile(r"(?i)mm\s*/\s*s")
# density MUST be unit-anchored to g/cm3 (a bare number near "density" too often
# catches the 1.75 mm diameter, an ASTM id, or a test temperature). Filled/metal
# filaments can exceed 1.7, so allow up to 5.0 but reject the common diameters.
# accept g/cm3, g/cc, g·cm-3 (3DXTech writes the unit glued to the value: "g/cc1.05").
# The cube marker is matched as an explicit optional (³ or a lone 3) so it never
# swallows the leading digit of the density value.
_UNIT = r"g\s*[·/]?\s*c[cm]\s*(?:³|3)?"
DENS_AFTER = re.compile(r"(?i)(\d+(?:\.\d{1,3})?)\s*" + _UNIT)
DENS_BEFORE = re.compile(r"(?i)" + _UNIT + r"\s*[^\d\n]{0,40}?(\d+(?:\.\d{1,3})?)")
_DIAMETERS = {1.75, 2.85, 3.0}
# tolerant of "Temp" abbreviation and the PDF "T emp" space artifact (3DXTech),
# and of "Extrusion Temp" wording.
SPEC_TEMP = re.compile(r"(?i)(?:print\w*|nozzle|extrud\w*|extrus\w*)\s*t\s*emp(?:erature)?\s*[=:]?\s*(\d{2,3})")
SPEC_SPEED = re.compile(r"(?i)(?:printing\s*)?speed\s*[=:]?\s*(\d{1,3})")
SPEC_BED = re.compile(r"(?i)(?:base\s*plate|build\s*plate|heated\s*bed|bed)(?:\s*t\s*emp\w*)?\s*[=:]?\s*(\d{2,3})")
MM_YYYY = re.compile(r"\b(?:0[1-9]|1[0-2])/(?:20\d{2})\b")
# drying: "Drying conditions 80℃, 12h" / "Dry: 65 °C / 8h" (app doesn't scan this; we add it)
DRY_RX = re.compile(r"(?i)dry\w*[^\n]{0,30}?(\d{2,3})\s*(?:°\s*c|℃|°)[^\n]{0,8}?(\d{1,2})\s*h")
DRY_TEMP_ONLY = re.compile(r"(?i)dry\w*[^\n]{0,30}?(\d{2,3})\s*(?:°\s*c|℃)")

# polymer family floors (subset of polymer.rs) for range disambiguation
FAMILY = {  # base_type -> (nozzle_lo, nozzle_hi, bed_lo, bed_hi)
    "PLA": (190, 220, 0, 60), "PETG": (230, 250, 60, 85), "ABS": (240, 270, 90, 110),
    "ASA": (240, 265, 90, 110), "TPU": (210, 240, 30, 60), "PC": (260, 300, 100, 120),
    "PA": (250, 300, 40, 110), "PP": (220, 260, 80, 110), "HIPS": (230, 250, 90, 110),
}

def fnum(s):
    try: return float(s)
    except: return None

def scan_range_after(text, labels, bed_max=None):
    low = text.lower()
    for lab in labels:
        start = 0
        while True:  # try EVERY occurrence — the first is often a distractor
            i = low.find(lab, start)             # ("higher nozzle temperatures" advice
            if i < 0:
                break                            #  before the real "Nozzle Temp 220-250")
            start = i + len(lab)
            win = text[i + len(lab): i + len(lab) + 80]
            m = RANGE_RX.search(win)
            if m:
                lo, hi = fnum(m.group(1)), fnum(m.group(2))
                if lo and hi and lo <= hi and not (bed_max and (lo > bed_max + 10 or hi > bed_max + 10)):
                    return lo, hi
            # single value fallback: "Nozzle 240 °C" / "215⁰C Extruder"
            m2 = re.search(r"(\d{2,3}(?:\.\d+)?)\s*(?:°\s*c|℃|⁰|º|°)", win)
            if m2:
                v = fnum(m2.group(1))
                if v and (not bed_max or v <= bed_max + 10):
                    return v, v
    return None, None

def scan_print_speed(text):
    low = text.lower()
    for lab in ("printing speed", "print speed", "speed", "vitesse"):
        i = low.find(lab)
        if i < 0: continue
        win = text[i: i + 90]
        if SPEED_UNIT_RX.search(win):
            m = RANGE_RX.search(win)
            if m:
                return fnum(m.group(1)), fnum(m.group(2))
            m2 = re.search(r"(\d{2,3})\s*mm\s*/\s*s", win)
            if m2:
                return None, fnum(m2.group(1))
    return None, None

def scan_density(text):
    low = text.lower(); i = low.find("density")
    win = text[max(0, i - 20): i + 140] if i >= 0 else text[:600]
    for rx in (DENS_AFTER, DENS_BEFORE):
        for m in rx.finditer(win):
            v = fnum(m.group(1))
            if v and 0.7 <= v <= 5.0 and v not in _DIAMETERS:
                return v
    return None

def scan_specimen(text):
    low = text.lower()
    for anchor in ("following conditions", "splines are printed", "specimens are printed",
                   "samples are printed", "printed specimen conditions", "specimen conditions",
                   "test specimen", "test conditions", "are printed under", "print test condition"):
        i = low.find(anchor)
        if i < 0: continue
        win = text[i: i + 260]
        n = SPEC_TEMP.search(win); s = SPEC_SPEED.search(win); b = SPEC_BED.search(win)
        nv = fnum(n.group(1)) if n else None
        sv = fnum(s.group(1)) if s else None
        bv = fnum(b.group(1)) if b else None
        nv = nv if (nv and 120 <= nv <= 500) else None   # up to TPI/PEEK/PEI (~445 C)
        sv = sv if (sv and 1 <= sv <= 1000) else None
        bv = bv if (bv and 15 <= bv <= 200) else None     # up to PPSU/PEI chamber-bed
        if nv or sv or bv:
            return nv, sv, bv
    return None, None, None

def scan_drying(text):
    # drying temps are 35-180 °C and times 1-48 h; anything outside is a misread
    # (e.g. a Tg of 257 °C, or a 27 that is really something else).
    m = DRY_RX.search(text)
    if m:
        t, h = fnum(m.group(1)), fnum(m.group(2))
        t = t if (t and 35 <= t <= 180) else None
        h = h if (h and 1 <= h <= 48) else None
        if t or h:
            return t, h
    m = DRY_TEMP_ONLY.search(text)
    if m:
        t = fnum(m.group(1))
        if t and 35 <= t <= 180:
            return t, None
    return None, None

def parse_tds(text, base_type=None):
    fam = FAMILY.get((base_type or "").upper())
    bed_max = fam[3] if fam else None
    n_lo, n_hi = scan_range_after(text, [
        "nozzle temperature", "extruder temperature", "print temperature",
        "bottom printing temperature", "température buse", "température d'impression",
        "printing temperature", "3d printing temperature",
        "nozzle temp", "extruder temp", "print temp", "printing temp",
        # last-resort bare labels (Extrudr "Nozzle 200-230°C"); specific ones win first
        "hotend", "hot end", "nozzle", "buse"])
    b_lo, b_hi = scan_range_after(text, [
        "bed temperature", "heated bed", "platform temperature", "température plateau",
        "plateau chauffant", "base plate", "plate temp", "bed temp", "platform temp",
        "heatbed", "heat bed", "hot bed"], bed_max=bed_max)
    s_lo, s_hi = scan_print_speed(text)
    dens = scan_density(text)
    spec_n, spec_s, spec_b = scan_specimen(text)
    dry_t, dry_h = scan_drying(text)
    # a bare "Nozzle ..." label can grab a diameter (0.4mm) or an orientation
    # angle ("+/- 45°"); no filament prints below 120 C, so drop such readings and
    # let the authoritative specimen "Extrusion Temp" value take over.
    if n_lo is not None and n_lo < 120:
        n_lo = n_hi = None
    # store table ranges; if missing fall back to authoritative specimen single value
    if n_lo is None and spec_n: n_lo = n_hi = spec_n
    if b_lo is None and spec_b: b_lo = b_hi = spec_b
    params = {"nozzle_min": n_lo, "nozzle_max": n_hi, "bed_min": b_lo, "bed_max": b_hi,
              "dry_temp": dry_t, "dry_time": dry_h}
    return {"params": params, "density": dens,
            "specimen": {"nozzle": spec_n, "speed": spec_s, "bed": spec_b},
            "speed": {"min": s_lo, "max": s_hi},
            "revision": (MM_YYYY.search(text[:600]) or [None]) and (MM_YYYY.search(text[:600]).group(0) if MM_YYYY.search(text[:600]) else None)}

def fetch_text(url):
    h = hashlib.sha1(url.encode()).hexdigest()[:16]
    pdf = CACHE / f"{h}.pdf"
    if not pdf.exists() or pdf.stat().st_size == 0:
        req = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0"})
        data = urllib.request.urlopen(req, timeout=90).read()
        pdf.write_bytes(data)
    if pdf.read_bytes()[:5] != b"%PDF-":
        return None, "not-pdf"
    from pypdf import PdfReader
    try:
        r = PdfReader(str(pdf))
        text = "\n".join((p.extract_text() or "") for p in r.pages)
        return text, None
    except Exception as e:
        return None, f"pypdf-error:{e}"

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--url")
    ap.add_argument("--limit", type=int, default=0)
    args = ap.parse_args()
    sys.stdout.reconfigure(encoding="utf-8")

    if args.url:
        text, err = fetch_text(args.url)
        if err: print("ERR", err); return
        print(f"text chars: {len(text)}")
        print(json.dumps(parse_tds(text), ensure_ascii=False, indent=2))
        return

    c = sqlite3.connect(DB)
    q = """SELECT b.name, m.label, m.base_type, d.url
           FROM materials m JOIN brands b ON b.id=m.brand_id
           JOIN document_refs d ON d.material_id=m.id AND upper(d.doc_type)='TDS'
           WHERE NOT EXISTS (SELECT 1 FROM printing_params p
                             WHERE p.material_id=m.id AND p.source='manufacturer')
           ORDER BY b.name, m.label"""
    rows = c.execute(q).fetchall()
    if args.limit: rows = rows[:args.limit]
    recs, image_only, failed, got = [], [], [], 0
    seen = set()
    for brand, label, base, url in rows:
        key = (brand, label)
        if key in seen:  # one material may have several TDS docs
            continue
        text, err = fetch_text(url)
        if err or not text or len(text.strip()) < 40:
            (image_only if (err is None or "not-pdf" in str(err)) else failed).append((brand, label, url, err or "empty-text"))
            continue
        seen.add(key)
        r = parse_tds(text, base)
        p = r["params"]
        if any(p.get(k) is not None for k in p):
            got += 1
            recs.append({"brand": brand, "material": label, "base_type": base,
                         "filled_type": None, "density": r["density"],
                         "params": p, "docs": [{"doc_type": "TDS", "url": url, "rohs_compliant": None}],
                         "colors": [], "source": "manufacturer"})
            sn, ss, sb = r["specimen"]["nozzle"], r["specimen"]["speed"], r["specimen"]["bed"]
            extra = f" spec(n={sn},s={ss},b={sb})" if (sn or ss or sb) else ""
            print(f"OK  {brand:<14} {label:<26} N={p['nozzle_min']}-{p['nozzle_max']} "
                  f"B={p['bed_min']}-{p['bed_max']} dry={p['dry_temp']}/{p['dry_time']} dens={r['density']}{extra}")
        else:
            image_only.append((brand, label, url, "no-params-in-text"))

    OUT.write_text(json.dumps({"cluster": "tds_extract",
        "_note": "Printing params extracted from the TEXT layer of manufacturer TDS PDFs "
                 "already linked in the DB (pypdf + tds.rs-ported regexes). Manufacturer-published facts.",
        "records": recs}, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"\n=== {got} materials got params -> {OUT.name} ===")
    print(f"image-only / no-text-params (Ollama-OCR candidates): {len(image_only)}")
    for b, l, u, e in image_only:
        print(f"  OCR? {b:<14} {l:<26} {e}")
    if failed:
        print(f"download/parse failures: {len(failed)}")
        for b, l, u, e in failed:
            print(f"  FAIL {b:<14} {l:<26} {e}")

if __name__ == "__main__":
    main()
