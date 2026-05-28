#!/usr/bin/env python3
"""Build the MD Optimisateur pre-extracted filament database.

This script FETCHES the open filament catalog published as JSON by the
TigerTag project (GitHub repo ``TigerTag-Project/TigerTag-RFID-Guide``,
``database/`` folder) and populates a read-only SQLite database with the
factual printing parameters it contains.

Legal note (see tools/sds-importer/data/FILAMENT_DB_DESIGN.md for detail):
  * We CONSUME the public JSON *data* only. Raw facts (a brand name, a
    recommended nozzle temperature) are not copyrightable.
  * We do NOT copy or embed any TigerTag *code/SDK* (that is GPLv3).
  * Nothing in this product is named "TigerTag"; the upstream id is kept
    only as an internal cross-reference column (``tigertag_*_id``).
  * No vendor TDS/MSDS/RoHS PDFs are stored or redistributed. The
    ``document_refs`` table holds only extracted factual parameters plus a
    SOURCE URL that deep-links to the vendor's own hosted document. It is
    created here but populated by a later enrichment pass.

The script is ADDITIVE and standalone: it uses only the Python standard
library (urllib, json, sqlite3) so it can run in CI without extra deps.

Outputs (under tools/sds-importer/data/):
  * filaments.sqlite  -- the read-only database
  * filaments.json    -- a JSON mirror of the same content (for the app
                         snapshot / diffing / inspection without sqlite)

Usage:
    python scripts/build_filament_db.py [--offline DIR] [--db PATH]

    --offline DIR   Read id_*.json from a local directory instead of the
                    network (useful for CI and reproducible builds).
    --db PATH       Override the output sqlite path.
"""

from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import re
import sqlite3
import sys
import urllib.request
from typing import Any, Dict, List, Optional, Tuple

# --------------------------------------------------------------------------
# Source configuration
# --------------------------------------------------------------------------

# Verified raw paths (branch ``main``). Confirmed reachable on 2026-05-28.
RAW_BASE = (
    "https://raw.githubusercontent.com/"
    "TigerTag-Project/TigerTag-RFID-Guide/main/database"
)

# Files we consume. ``id_aspect.json`` / ``id_diameter.json`` exist upstream
# but are not needed to populate the schema below; diameter defaults to the
# de-facto 1.75 mm standard used by the U1 ecosystem.
SOURCE_FILES = {
    "brand": "id_brand.json",
    "material": "id_material.json",
    "type": "id_type.json",
    "diameter": "id_diameter.json",
}

DEFAULT_DIAMETER_MM = 1.75

# Repo-relative output locations.
_THIS_DIR = os.path.dirname(os.path.abspath(__file__))
_REPO_ROOT = os.path.dirname(_THIS_DIR)
DATA_DIR = os.path.join(_REPO_ROOT, "tools", "sds-importer", "data")
DEFAULT_DB_PATH = os.path.join(DATA_DIR, "filaments.sqlite")
DEFAULT_JSON_PATH = os.path.join(DATA_DIR, "filaments.json")

USER_AGENT = "MD-Optimisateur-FilamentDB-Builder/1.0 (+factual-data-only)"


# --------------------------------------------------------------------------
# Fetching
# --------------------------------------------------------------------------

def _fetch_json(url: str) -> Any:
    """GET a URL and parse JSON, with an explicit UA and a sane timeout."""
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    with urllib.request.urlopen(req, timeout=30) as resp:  # noqa: S310 (https only)
        raw = resp.read().decode("utf-8")
    return json.loads(raw)


def load_sources(offline_dir: Optional[str]) -> Dict[str, Any]:
    """Load every source file either from disk (offline) or the network."""
    out: Dict[str, Any] = {}
    for key, fname in SOURCE_FILES.items():
        if offline_dir:
            path = os.path.join(offline_dir, fname)
            print(f"  reading {path}")
            with open(path, "r", encoding="utf-8") as fh:
                out[key] = json.load(fh)
        else:
            url = f"{RAW_BASE}/{fname}"
            print(f"  fetching {url}")
            out[key] = _fetch_json(url)
    return out


# --------------------------------------------------------------------------
# Derivation helpers
# --------------------------------------------------------------------------

# Recognised base polymer families, longest-token-first so "PETG" wins over a
# bare "PET" and "PLA" is matched as a whole token. Mirrors the families the
# existing extractor's polymer.rs knows about, plus a few catalog-only ones.
_BASE_TYPES = [
    "PETG", "PET", "PLA", "ABS", "ASA", "TPU", "TPE", "PCTG", "PC",
    "PVA", "PVB", "HIPS", "PP", "PE", "PA12", "PA6", "PA", "PPS",
    "PEEK", "PEKK", "PEI", "PSU", "POM", "PHA",
]

# Tokens in a label that indicate a fibre/particle fill.
_FILL_PATTERNS = [
    ("CF", re.compile(r"\bCF\b|carbon", re.I)),
    ("GF", re.compile(r"\bGF\b|glass", re.I)),
    ("KF", re.compile(r"\bKF\b|kevlar|aramid", re.I)),
    ("WD", re.compile(r"\bwood\b", re.I)),
    ("MTL", re.compile(r"\bmetal\b|\bsteel\b|\bcopper\b|\bbronze\b", re.I)),
]


def derive_base_type(label: str, material_type: str) -> Optional[str]:
    """Derive the base polymer family (PLA/PETG/ABS/...).

    Prefer the explicit ``material_type`` field when the catalog provides a
    non-empty one; otherwise fall back to scanning the label, which is needed
    because some entries (e.g. "PETG-GF") ship an empty ``material_type``.
    """
    mt = (material_type or "").strip().upper()
    # TigerTag carries a sentinel "None" recipe (the blank/unprogrammed tag).
    # Treat such placeholders as no family rather than a literal "NONE".
    if mt in {"NONE", "NULL", "N/A"}:
        mt = ""
    if mt:
        # material_type may itself be a filled token (e.g. "PE" for "PE-CF").
        for base in _BASE_TYPES:
            if mt == base:
                return base
        # Otherwise treat the first label token below as authoritative.
    if (label or "").strip().upper() in {"NONE", "NULL", "N/A"}:
        return None
    upper = (label or "").upper()
    for base in _BASE_TYPES:
        # Word-ish boundary: base followed by end / dash / digit / non-alpha.
        if re.search(rf"(?<![A-Z]){re.escape(base)}(?![A-Z])", upper):
            return base
    # Last resort: a non-empty material_type we didn't recognise as a family.
    return mt or None


def derive_filled_type(label: str, filled_type: Any, filled: Any) -> Optional[str]:
    """Derive the fill kind (CF/GF/...) or None.

    The catalog's own ``filled_type`` is used when present; otherwise the
    label is scanned. ``filled`` (bool) only tells us *that* it is filled,
    not with what, so it is used solely as a tie-breaker.
    """
    ft = (filled_type or "")
    if isinstance(ft, str) and ft.strip():
        return ft.strip().upper()
    for token, rx in _FILL_PATTERNS:
        if rx.search(label or ""):
            return token
    return None


def _f(value: Any) -> Optional[float]:
    """Coerce a JSON value to float or None (treats null / '' as None)."""
    if value is None or value == "":
        return None
    try:
        return float(value)
    except (TypeError, ValueError):
        return None


# --------------------------------------------------------------------------
# Schema
# --------------------------------------------------------------------------

SCHEMA = """
CREATE TABLE brands (
    id                INTEGER PRIMARY KEY,
    name              TEXT NOT NULL,
    website           TEXT,
    tigertag_brand_id INTEGER UNIQUE
);

CREATE TABLE materials (
    id                   INTEGER PRIMARY KEY,
    brand_id             INTEGER REFERENCES brands(id),
    label                TEXT NOT NULL,
    base_type            TEXT,
    filled_type          TEXT,
    density              REAL,
    diameter             REAL,
    tigertag_material_id INTEGER UNIQUE,
    bambu_id             TEXT,
    creality_id          TEXT
);

CREATE TABLE color_variants (
    id          INTEGER PRIMARY KEY,
    material_id INTEGER REFERENCES materials(id),
    color_name  TEXT,
    hex         TEXT,
    finish      TEXT
);

CREATE TABLE printing_params (
    id          INTEGER PRIMARY KEY,
    material_id INTEGER REFERENCES materials(id),
    nozzle_min  REAL,
    nozzle_max  REAL,
    bed_min     REAL,
    bed_max     REAL,
    dry_temp    REAL,
    dry_time    REAL,
    source      TEXT
);

CREATE TABLE document_refs (
    id                    INTEGER PRIMARY KEY,
    material_id           INTEGER REFERENCES materials(id),
    doc_type              TEXT,            -- 'TDS' | 'MSDS' | 'RoHS'
    url                   TEXT,            -- deep link to vendor-hosted PDF
    retrieved_at          TEXT,
    extraction_confidence REAL,
    rohs_compliant        INTEGER          -- 0 / 1 / NULL (unknown)
);

CREATE INDEX idx_materials_brand   ON materials(brand_id);
CREATE INDEX idx_params_material   ON printing_params(material_id);
CREATE INDEX idx_colors_material   ON color_variants(material_id);
CREATE INDEX idx_docs_material     ON document_refs(material_id);
"""


def create_schema(conn: sqlite3.Connection) -> None:
    conn.executescript(SCHEMA)


# --------------------------------------------------------------------------
# Population
# --------------------------------------------------------------------------

def populate_brands(conn: sqlite3.Connection, brands: List[Dict[str, Any]]) -> int:
    rows = []
    for b in brands:
        bid = b.get("id")
        name = (b.get("name") or "").strip()
        if bid is None or not name:
            continue
        # website is unknown from the catalog; left NULL, filled by enrichment.
        rows.append((bid, name, None, bid))
    conn.executemany(
        "INSERT OR REPLACE INTO brands (id, name, website, tigertag_brand_id) "
        "VALUES (?, ?, ?, ?)",
        rows,
    )
    return len(rows)


def populate_materials_and_params(
    conn: sqlite3.Connection,
    materials: List[Dict[str, Any]],
) -> Tuple[int, int]:
    """Insert materials + their printing_params.

    The TigerTag material catalog is a flat list of (family x fill) recipes;
    it carries no brand linkage (brand association lives on the physical NFC
    tag, not the recipe). We therefore leave ``brand_id`` NULL here and let
    the enrichment pass attach brands. ``tigertag_material_id`` is the stable
    cross-reference.
    """
    mat_rows = []
    param_rows = []
    seen_ids = set()

    for m in materials:
        mid = m.get("id")
        if mid is None or mid in seen_ids:
            continue
        seen_ids.add(mid)

        label = (m.get("label") or "").strip()
        base_type = derive_base_type(label, m.get("material_type", ""))
        filled_type = derive_filled_type(
            label, m.get("filled_type"), m.get("filled")
        )
        density = _f(m.get("density"))

        meta = m.get("metadata") or {}
        bambu_id = meta.get("bambuID")
        creality_id = meta.get("crealityID")

        mat_rows.append(
            (
                mid,            # id (use the upstream id as PK; stable + unique)
                None,           # brand_id (enrichment)
                label,
                base_type,
                filled_type,
                density,
                DEFAULT_DIAMETER_MM,
                mid,            # tigertag_material_id
                bambu_id,
                creality_id,
            )
        )

        rec = m.get("recommended") or {}
        # Only emit a params row if the catalog gave us at least one value.
        vals = (
            _f(rec.get("nozzleTempMin")),
            _f(rec.get("nozzleTempMax")),
            _f(rec.get("bedTempMin")),
            _f(rec.get("bedTempMax")),
            _f(rec.get("dryTemp")),
            _f(rec.get("dryTime")),
        )
        if any(v is not None for v in vals):
            param_rows.append((mid, *vals, "tigertag"))

    conn.executemany(
        "INSERT OR REPLACE INTO materials "
        "(id, brand_id, label, base_type, filled_type, density, diameter, "
        " tigertag_material_id, bambu_id, creality_id) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        mat_rows,
    )
    conn.executemany(
        "INSERT INTO printing_params "
        "(material_id, nozzle_min, nozzle_max, bed_min, bed_max, dry_temp, "
        " dry_time, source) "
        "VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        param_rows,
    )
    return len(mat_rows), len(param_rows)


# --------------------------------------------------------------------------
# Validation + JSON mirror
# --------------------------------------------------------------------------

def validate(db_path: str) -> Dict[str, int]:
    """Re-open the sqlite read-only and count rows in every table."""
    uri = f"file:{db_path}?mode=ro"
    conn = sqlite3.connect(uri, uri=True)
    try:
        counts = {}
        for tbl in ("brands", "materials", "color_variants",
                    "printing_params", "document_refs"):
            counts[tbl] = conn.execute(
                f"SELECT COUNT(*) FROM {tbl}"  # noqa: S608 (fixed table names)
            ).fetchone()[0]
        # Spot-check a join works (materials <-> params).
        sample = conn.execute(
            "SELECT m.label, m.base_type, m.filled_type, p.nozzle_min, "
            "       p.nozzle_max, p.bed_min, p.bed_max, p.dry_temp, p.dry_time "
            "FROM materials m JOIN printing_params p ON p.material_id = m.id "
            "ORDER BY m.id LIMIT 5"
        ).fetchall()
        print("\n  Validation: re-opened sqlite read-only. Sample join rows:")
        for row in sample:
            print(f"    {row}")
        return counts
    finally:
        conn.close()


def emit_json_mirror(db_path: str, json_path: str) -> None:
    """Write a JSON mirror of the DB content for the app snapshot."""
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    try:
        def dump(table: str) -> List[Dict[str, Any]]:
            return [dict(r) for r in conn.execute(
                f"SELECT * FROM {table}"  # noqa: S608 (fixed table names)
            ).fetchall()]

        mirror = {
            "schema_version": 1,
            "generated_at": _dt.datetime.now(_dt.timezone.utc).isoformat(),
            "source": "tigertag-rfid-guide/database (factual data only)",
            "brands": dump("brands"),
            "materials": dump("materials"),
            "color_variants": dump("color_variants"),
            "printing_params": dump("printing_params"),
            "document_refs": dump("document_refs"),
        }
    finally:
        conn.close()
    with open(json_path, "w", encoding="utf-8") as fh:
        json.dump(mirror, fh, ensure_ascii=False, indent=2)
    print(f"  wrote JSON mirror -> {json_path}")


# --------------------------------------------------------------------------
# Main
# --------------------------------------------------------------------------

def build(db_path: str, json_path: str, offline_dir: Optional[str]) -> None:
    os.makedirs(os.path.dirname(db_path), exist_ok=True)

    print("Loading TigerTag catalog sources ...")
    sources = load_sources(offline_dir)
    brands = sources["brand"]
    materials = sources["material"]
    print(f"  loaded {len(brands)} brand records, "
          f"{len(materials)} material records")

    # Fresh build: remove any stale DB so PRIMARY KEYs are deterministic.
    if os.path.exists(db_path):
        os.remove(db_path)

    conn = sqlite3.connect(db_path)
    try:
        create_schema(conn)
        n_brands = populate_brands(conn, brands)
        n_mats, n_params = populate_materials_and_params(conn, materials)
        conn.commit()
    finally:
        conn.close()

    counts = validate(db_path)
    emit_json_mirror(db_path, json_path)

    print("\n=== Build complete ===")
    print(f"  brands ............ {counts['brands']:>4}")
    print(f"  materials ......... {counts['materials']:>4}")
    print(f"  printing_params ... {counts['printing_params']:>4}")
    print(f"  color_variants .... {counts['color_variants']:>4} (empty; enrichment)")
    print(f"  document_refs ..... {counts['document_refs']:>4} (empty; enrichment)")
    print(f"\n  sqlite -> {db_path}")

    # Sanity: inserted counts should match validated counts.
    assert counts["brands"] == n_brands, "brand count mismatch"
    assert counts["materials"] == n_mats, "material count mismatch"
    assert counts["printing_params"] == n_params, "param count mismatch"


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--offline", metavar="DIR", default=None,
        help="read id_*.json from a local directory instead of the network",
    )
    parser.add_argument(
        "--db", metavar="PATH", default=DEFAULT_DB_PATH,
        help="output sqlite path (default: tools/sds-importer/data/filaments.sqlite)",
    )
    parser.add_argument(
        "--json", metavar="PATH", default=DEFAULT_JSON_PATH,
        help="output JSON mirror path",
    )
    args = parser.parse_args(argv)

    try:
        build(args.db, args.json, args.offline)
    except Exception as exc:  # noqa: BLE001 - top-level reporting
        print(f"ERROR: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
