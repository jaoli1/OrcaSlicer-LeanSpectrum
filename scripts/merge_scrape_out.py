#!/usr/bin/env python3
"""
Merge per-cluster manufacturer-scrape JSON (tools/sds-importer/data/scrape_out/
cluster_*.json) into the master filaments.sqlite. Single-threaded, idempotent
(safe to re-run). Each cluster file: {"cluster": N, "records": [record, ...]}.
record = {brand, material, base_type, density, params{nozzle_min,nozzle_max,
bed_min,bed_max,dry_temp,dry_time}, docs[{doc_type,url,rohs_compliant}],
colors[{name,hex,finish}], source}. Only manufacturer-sourced facts; no PDFs.
"""
from __future__ import annotations
import glob, json, os, sqlite3, datetime
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
DB = REPO / "tools" / "sds-importer" / "data" / "filaments.sqlite"
OUT = REPO / "tools" / "sds-importer" / "data" / "scrape_out"

def next_id(c, table):
    r = c.execute(f"SELECT COALESCE(MAX(id),0)+1 FROM {table}").fetchone()
    return r[0]

def main():
    files = sorted(glob.glob(str(OUT / "cluster_*.json")))
    if not files:
        print("no cluster_*.json found in", OUT); return
    c = sqlite3.connect(DB)
    c.execute("PRAGMA foreign_keys=ON")
    now = datetime.date.today().isoformat()
    brand_by_name = {n.lower(): i for i, n in c.execute("SELECT id,name FROM brands")}
    stats = dict(records=0, materials_new=0, docs=0, colors=0, params=0, skipped_brand=0, files=0)

    for f in files:
        stats["files"] += 1
        data = json.load(open(f, encoding="utf-8"))
        for rec in data.get("records", []):
            stats["records"] += 1
            bid = brand_by_name.get(str(rec.get("brand", "")).lower())
            if not bid:
                stats["skipped_brand"] += 1
                continue
            label = (rec.get("material") or "").strip() or "Filament"
            row = c.execute("SELECT id FROM materials WHERE brand_id=? AND lower(label)=lower(?)",
                            (bid, label)).fetchone()
            if row:
                mid = row[0]
            else:
                mid = next_id(c, "materials")
                c.execute("INSERT INTO materials(id,brand_id,label,base_type,filled_type,density,diameter,"
                          "tigertag_material_id,bambu_id,creality_id) VALUES(?,?,?,?,?,?,?,?,?,?)",
                          (mid, bid, label, rec.get("base_type"), rec.get("filled_type"),
                           rec.get("density"), rec.get("diameter", 1.75), None, None, None))
                stats["materials_new"] += 1
            # docs (dedup on material+url)
            for d in rec.get("docs", []):
                url = (d.get("url") or "").strip()
                if not url:
                    continue
                if c.execute("SELECT 1 FROM document_refs WHERE material_id=? AND url=?", (mid, url)).fetchone():
                    continue
                c.execute("INSERT INTO document_refs(id,material_id,doc_type,url,retrieved_at,"
                          "extraction_confidence,rohs_compliant) VALUES(?,?,?,?,?,?,?)",
                          (next_id(c, "document_refs"), mid, d.get("doc_type"), url, now,
                           0.9, d.get("rohs_compliant")))
                stats["docs"] += 1
            # colors (dedup on material+hex, or material+name when hex null)
            for col in rec.get("colors", []):
                hexv = (col.get("hex") or None)
                name = (col.get("name") or None)
                dup = c.execute("SELECT 1 FROM color_variants WHERE material_id=? AND "
                                "IFNULL(hex,'')=IFNULL(?,'') AND IFNULL(color_name,'')=IFNULL(?,'')",
                                (mid, hexv, name)).fetchone()
                if dup:
                    continue
                c.execute("INSERT INTO color_variants(id,material_id,color_name,hex,finish) VALUES(?,?,?,?,?)",
                          (next_id(c, "color_variants"), mid, name, hexv, col.get("finish")))
                stats["colors"] += 1
            # params (one manufacturer row per material)
            p = rec.get("params") or {}
            if any(p.get(k) is not None for k in ("nozzle_min","nozzle_max","bed_min","bed_max","dry_temp","dry_time")):
                if not c.execute("SELECT 1 FROM printing_params WHERE material_id=? AND source='manufacturer'",
                                 (mid,)).fetchone():
                    c.execute("INSERT INTO printing_params(id,material_id,nozzle_min,nozzle_max,bed_min,bed_max,"
                              "dry_temp,dry_time,source) VALUES(?,?,?,?,?,?,?,?,'manufacturer')",
                              (next_id(c, "printing_params"), mid, p.get("nozzle_min"), p.get("nozzle_max"),
                               p.get("bed_min"), p.get("bed_max"), p.get("dry_temp"), p.get("dry_time")))
                    stats["params"] += 1
    c.commit()
    print("MERGE DONE:", stats)
    print("totals -> materials:", c.execute("SELECT COUNT(*) FROM materials").fetchone()[0],
          "| doc_refs:", c.execute("SELECT COUNT(*) FROM document_refs").fetchone()[0],
          "| colors(hex):", c.execute("SELECT COUNT(*) FROM color_variants WHERE hex IS NOT NULL").fetchone()[0],
          "| mfr params:", c.execute("SELECT COUNT(*) FROM printing_params WHERE source='manufacturer'").fetchone()[0])
    c.close()

if __name__ == "__main__":
    main()
