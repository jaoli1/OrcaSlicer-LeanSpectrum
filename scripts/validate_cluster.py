#!/usr/bin/env python3
"""
Validate a scrape_out/cluster_*.json before merging it into filaments.sqlite.
Enforces the project's hard rule: facts must come from the manufacturer's own
site/docs (never TigerTag, never resellers, never third-party DBs), and the
numbers must be physically plausible. Prints a per-record report and a verdict.

Usage: python scripts/validate_cluster.py tools/.../scrape_out/cluster_web_a.json
"""
from __future__ import annotations
import json, re, sys
from pathlib import Path
from urllib.parse import urlparse

# domains that are NEVER acceptable as a manufacturer source
DENY = re.compile(r"(?i)\b(tigertag|amazon|aliexpress|ebay|etsy|wish|temu|reddit|"
                  r"facebook|youtube|3dprima|3djake|matterhackers\.com/r/|filament2print|"
                  r"reichelt|conrad|banggood|alibaba)\b")
# value plausibility windows
RANGES = {"nozzle_min": (150, 500), "nozzle_max": (150, 500), "bed_min": (0, 200),
          "bed_max": (0, 200), "dry_temp": (35, 180), "dry_time": (1, 48)}

def brand_tokens(brand: str):
    b = re.sub(r"[^a-z0-9]", "", brand.lower())
    toks = {b}
    for t in re.split(r"[\s_]+", brand.lower()):
        t = re.sub(r"[^a-z0-9]", "", t)
        if len(t) >= 3:
            toks.add(t)
    return toks

def host_of(url: str):
    try:
        return (urlparse(url).hostname or "").lower()
    except Exception:
        return ""

def main():
    if len(sys.argv) < 2:
        print("usage: validate_cluster.py <cluster.json>"); return 2
    data = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
    recs = data.get("records", [])
    print(f"cluster={data.get('cluster')} records={len(recs)}")
    warn = err = ok = 0
    for r in recs:
        brand = r.get("brand", "?"); label = r.get("material", "?")
        src = r.get("_source_note") or ""
        doc = " ".join(d.get("url", "") for d in (r.get("docs") or []))
        # _source_note is often prose with an embedded URL -> pull every URL out
        urls = re.findall(r"https?://[^\s;,)\"']+", src + " " + doc)
        hosts = [host_of(u) for u in urls]
        host = hosts[0] if hosts else ""
        p = r.get("params") or {}
        msgs = []
        # provenance
        if not urls:
            msgs.append("ERR no source URL")
        else:
            bad = [h for u, h in zip(urls, hosts) if DENY.search(u)]
            if bad:
                msgs.append(f"ERR non-manufacturer source: {bad}")
            elif not any(brand_tokens(brand) & set(re.findall(r"[a-z0-9]+", h)) for h in hosts):
                msgs.append(f"WARN no brand token in {hosts} (verify maker; ok for house-brand/CDN hosts)")
        # plausibility
        for k, (lo, hi) in RANGES.items():
            v = p.get(k)
            if v is not None and not (lo <= v <= hi):
                msgs.append(f"ERR {k}={v} out of [{lo},{hi}]")
        if p.get("nozzle_min") and p.get("nozzle_max") and p["nozzle_min"] > p["nozzle_max"]:
            msgs.append("ERR nozzle_min>nozzle_max")
        if p.get("bed_min") is not None and p.get("bed_max") is not None and p["bed_min"] > p["bed_max"]:
            msgs.append("ERR bed_min>bed_max")
        d = r.get("density")
        if d is not None and not (0.7 <= d <= 2.5):
            msgs.append(f"ERR density={d} implausible")
        if not any(p.get(k) is not None for k in p) and d is None:
            msgs.append("WARN no params and no density (empty record)")
        tag = "OK " if not msgs else ("ERR" if any(m.startswith("ERR") for m in msgs) else "warn")
        if tag == "OK ": ok += 1
        elif tag == "ERR": err += 1
        else: warn += 1
        extra = ("  " + " | ".join(msgs)) if msgs else ""
        print(f"  [{tag}] {brand:<15} {label:<24} {host}{extra}")
    print(f"\nverdict: ok={ok} warn={warn} err={err}  -> "
          + ("SAFE TO MERGE" if err == 0 else "FIX ERRORS BEFORE MERGE"))
    return 1 if err else 0

if __name__ == "__main__":
    sys.exit(main())
