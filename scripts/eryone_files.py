#!/usr/bin/env python3
"""
Build the full Eryone filament record set from the manufacturer's consolidated
files page (https://eryone3d.com/pages/filament-files). The page lists every
Eryone TDS as a Shopify-CDN PDF; we capture each as a verified manufacturer doc
link (params are extractable from each TDS later by the app's tds.rs). Output:
tools/sds-importer/data/scrape_out/cluster_eryone.json (merged by merge_scrape_out.py).
"""
from __future__ import annotations
import json, re, urllib.request
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
OUT = REPO / "tools" / "sds-importer" / "data" / "scrape_out" / "cluster_eryone.json"
PAGE = "https://eryone3d.com/pages/filament-files"

def material_of(url: str):
    fn = url.rsplit("/", 1)[-1]
    lbl = re.sub(r"^ERYO\w*[-_]+", "", fn[:-4])          # drop ERYONE-/ERYOE_-- prefix + .pdf
    lbl = re.sub(r"[-_]+TDS(_EN)?$", "", lbl, flags=re.I) # drop -TDS / _TDS / -TDS_EN
    lbl = re.sub(r"[\s_-]+", " ", lbl).strip()
    lbl = re.sub(r"\bplus\b", "+", lbl, flags=re.I).replace("PLA +", "PLA+").replace(" +", "+")
    up = lbl.upper()
    base = next((b for b in ["PETG", "PLA", "ABS", "ASA", "TPU", "PA12", "PA6", "PA", "PC", "PP"] if b in up), None)
    if base in ("PA6", "PA12"):
        base = "PA"
    filled = "CF" if re.search(r"\bCF\b", up) else ("GF" if re.search(r"\bGF\b", up) else None)
    return lbl, base, filled

def main():
    req = urllib.request.Request(PAGE, headers={"User-Agent": "Mozilla/5.0"})
    html = urllib.request.urlopen(req, timeout=60).read().decode("utf-8", "replace")
    urls = sorted({u.split("?")[0] for u in
                   re.findall(r"https://cdn\.shopify\.com/s/files/[^\"' ]*ERYO[^\"' ]*\.pdf", html)})
    recs, seen = [], set()
    for u in urls:
        lbl, base, filled = material_of(u)
        if not lbl or lbl in seen:
            continue
        seen.add(lbl)
        recs.append({
            "brand": "Eryone", "material": lbl, "base_type": base, "filled_type": filled,
            "density": None,
            "params": {"nozzle_min": None, "nozzle_max": None, "bed_min": None,
                       "bed_max": None, "dry_temp": None, "dry_time": None},
            "docs": [{"doc_type": "TDS", "url": u, "rohs_compliant": None}],
            "colors": [], "source": "manufacturer",
        })
    OUT.write_text(json.dumps({
        "cluster": "eryone",
        "_note": "Full Eryone TDS library from eryone3d.com/pages/filament-files (Shopify CDN). "
                 "Verified-pattern manufacturer links; per-material params extractable from each TDS via the app's tds.rs.",
        "records": recs}, indent=2, ensure_ascii=False), encoding="utf-8")
    print(f"wrote {len(recs)} Eryone records")
    for r in recs:
        print(f"  {r['material']:<24} -> base={r['base_type']} filled={r['filled_type']}")

if __name__ == "__main__":
    main()
