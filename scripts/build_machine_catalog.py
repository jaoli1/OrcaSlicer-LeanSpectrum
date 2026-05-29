#!/usr/bin/env python3
"""Build a compact machine/process catalog from OrcaSlicer's bundled profiles.

ADDITIVE TOOL — reads only, writes only new artifacts under
``tools/sds-importer/data/``. It does NOT modify any profile, app/Rust code,
workflow, or build file.

What it does
------------
Walks every ``resources/profiles/<Vendor>.json`` index and the machine /
process JSONs it references, resolves each printer *variant* (a concrete,
instantiable ``type:"machine"`` preset such as "Snapmaker U1 (0.4 nozzle)")
through its ``inherits`` chain to recover nozzle diameter, bed size, max layer
height and the default/compatible base process, and emits:

  * ``tools/sds-importer/data/machine_catalog.sqlite``  (primary)
  * ``tools/sds-importer/data/machine_catalog.json``    (mirror)

Schema (sqlite)
---------------
  vendors(id, name)
  machines(id, vendor_id, model_name, setting_id)
  machine_variants(id, machine_id, nozzle_diameter, bed_size,
                   max_layer_height, default_process_name, machine_profile_path)
  base_processes(id, vendor_id, name, sub_path, layer_height)

Why this shape
--------------
The generalized profile generator (see GENERATOR_DESIGN.md) needs, for any
target printer + nozzle: (a) which base process to inherit, and (b) the
nozzle-dependent envelope (max layer height, bed size) to scale line widths /
flow. ``machine_variants`` answers both; ``base_processes`` lets the generator
resolve the base layer height for a chosen process.

Robustness
----------
Profiles lean heavily on ``inherits``: of ~935 machine entries only ~63%
carry ``printable_area`` directly and ~81% ``printer_variant`` — the rest
inherit them. Every field is therefore resolved by walking the inherits chain
*within the same vendor's* machine/ (resp. process/) directory, guarding
against missing files, parse errors and inherits cycles. Anything unresolved
is left NULL rather than guessed.
"""

from __future__ import annotations

import json
import os
import re
import sqlite3
import sys
from typing import Any, Dict, List, Optional, Tuple

# ----------------------------------------------------------------------------
# Paths
# ----------------------------------------------------------------------------
SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO_ROOT = os.path.dirname(SCRIPT_DIR)
PROFILES_DIR = os.path.join(REPO_ROOT, "resources", "profiles")
OUT_DIR = os.path.join(REPO_ROOT, "tools", "sds-importer", "data")
SQLITE_PATH = os.path.join(OUT_DIR, "machine_catalog.sqlite")
JSON_PATH = os.path.join(OUT_DIR, "machine_catalog.json")


# ----------------------------------------------------------------------------
# Low-level JSON loading with caching + inherits resolution
# ----------------------------------------------------------------------------
class ProfileStore:
    """Loads and caches profile JSONs for one vendor, indexed by ``name``.

    OrcaSlicer resolves ``inherits`` by preset *name* (not file name) within a
    config bundle, so we index every JSON in a vendor's ``machine/`` (or
    ``process/``) folder by its ``name`` field and follow the chain by name.
    """

    def __init__(self, vendor_dir: str, kind: str):
        # kind is "machine" or "process"
        self.kind = kind
        self.dir = os.path.join(vendor_dir, kind)
        self.by_name: Dict[str, Dict[str, Any]] = {}
        self.path_by_name: Dict[str, str] = {}
        self.parse_errors: List[str] = []
        self._load_all()

    def _load_all(self) -> None:
        if not os.path.isdir(self.dir):
            return
        # Recurse: several vendors (Elegoo, FlyingBear, InfiMech, ...) keep
        # per-model leaves in subfolders (machine/EC/, machine/Ghost7/, ...)
        # while the shared inherits base sits elsewhere. Indexing by name
        # across the whole subtree lets the inherits chain resolve regardless
        # of which folder each link lives in.
        for root, _dirs, files in os.walk(self.dir):
            for fname in files:
                if not fname.lower().endswith(".json"):
                    continue
                fpath = os.path.join(root, fname)
                try:
                    with open(fpath, encoding="utf-8") as fh:
                        data = json.load(fh)
                except (OSError, ValueError) as exc:
                    self.parse_errors.append(f"{fpath}: {exc}")
                    continue
                if not isinstance(data, dict):
                    continue
                name = data.get("name")
                if not name:
                    # Fall back to the file stem so it is still resolvable.
                    name = os.path.splitext(fname)[0]
                # First writer wins; profile names are unique per bundle.
                self.by_name.setdefault(name, data)
                self.path_by_name.setdefault(name, fpath)

    def get(self, name: str) -> Optional[Dict[str, Any]]:
        return self.by_name.get(name)

    def resolve(self, name: str, key: str) -> Optional[Any]:
        """Return the first non-empty value for ``key`` walking inherits."""
        node = self.by_name.get(name)
        seen = set()
        while node is not None:
            nid = id(node)
            if nid in seen:  # inherits cycle guard
                break
            seen.add(nid)
            if key in node and _non_empty(node[key]):
                return node[key]
            parent = node.get("inherits")
            if not parent or not isinstance(parent, str):
                break
            node = self.by_name.get(parent)
        return None


def _non_empty(value: Any) -> bool:
    if value is None:
        return False
    if isinstance(value, str):
        return value.strip() != ""
    if isinstance(value, (list, dict)):
        return len(value) > 0
    return True


# ----------------------------------------------------------------------------
# Field extractors
# ----------------------------------------------------------------------------
def first_scalar(value: Any) -> Optional[str]:
    """Profiles store many machine values as 1+ element string arrays (one per
    extruder). Collapse to the first element; pass scalars through."""
    if value is None:
        return None
    if isinstance(value, list):
        for item in value:
            if _non_empty(item):
                return str(item).strip()
        return None
    s = str(value).strip()
    return s or None


def parse_float(value: Any) -> Optional[float]:
    s = first_scalar(value)
    if s is None:
        return None
    try:
        return float(s)
    except ValueError:
        m = re.search(r"[-+]?\d*\.?\d+", s)
        return float(m.group(0)) if m else None


def bed_size_from_printable_area(area: Any) -> Optional[str]:
    """``printable_area`` is a list of "XxY" corner points. Derive a compact
    "WxD" (mm) string from the bounding box. Returns None if unparseable."""
    if not isinstance(area, list) or not area:
        return None
    xs: List[float] = []
    ys: List[float] = []
    for pt in area:
        if not isinstance(pt, str):
            continue
        m = re.match(r"\s*([-+]?\d*\.?\d+)\s*[xX]\s*([-+]?\d*\.?\d+)\s*$", pt)
        if not m:
            continue
        xs.append(float(m.group(1)))
        ys.append(float(m.group(2)))
    if not xs or not ys:
        return None
    w = max(xs) - min(xs)
    d = max(ys) - min(ys)
    # Round to the nearest mm; drop the ".0" for whole numbers.
    return f"{_fmt_mm(w)}x{_fmt_mm(d)}"


def _fmt_mm(x: float) -> str:
    r = round(x)
    if abs(x - r) < 0.5:
        return str(int(r))
    return f"{x:.1f}"


# Matches a leading layer height in a process name, e.g.
# "0.20 Standard @Snapmaker U1 (0.4 nozzle)" -> 0.20
_LH_NAME_RE = re.compile(r"(?<!\d)(\d\.\d{1,3})(?=\s)")


def layer_height_for_process(store: ProfileStore, name: str) -> Optional[float]:
    """Resolve a process's layer height: explicit ``layer_height`` up the
    inherits chain, else the leading number in the (leaf) name."""
    lh = parse_float(store.resolve(name, "layer_height"))
    if lh is not None:
        return lh
    m = _LH_NAME_RE.match(name.strip())
    if m:
        return float(m.group(1))
    return None


# ----------------------------------------------------------------------------
# Catalog assembly
# ----------------------------------------------------------------------------
class Catalog:
    def __init__(self) -> None:
        self.vendors: List[Dict[str, Any]] = []
        self.machines: List[Dict[str, Any]] = []
        self.variants: List[Dict[str, Any]] = []
        self.processes: List[Dict[str, Any]] = []
        self._vendor_id = 0
        self._machine_id = 0
        self._variant_id = 0
        self._process_id = 0
        self.warnings: List[str] = []

    def add_vendor(self, name: str) -> int:
        self._vendor_id += 1
        self.vendors.append({"id": self._vendor_id, "name": name})
        return self._vendor_id

    def add_machine(self, vendor_id: int, model_name: str,
                    setting_id: Optional[str]) -> int:
        self._machine_id += 1
        self.machines.append({
            "id": self._machine_id,
            "vendor_id": vendor_id,
            "model_name": model_name,
            "setting_id": setting_id,
        })
        return self._machine_id

    def add_variant(self, machine_id: int, **kw: Any) -> int:
        self._variant_id += 1
        row = {"id": self._variant_id, "machine_id": machine_id}
        row.update(kw)
        self.variants.append(row)
        return self._variant_id

    def add_process(self, vendor_id: int, name: str, sub_path: Optional[str],
                    layer_height: Optional[float]) -> int:
        self._process_id += 1
        self.processes.append({
            "id": self._process_id,
            "vendor_id": vendor_id,
            "name": name,
            "sub_path": sub_path,
            "layer_height": layer_height,
        })
        return self._process_id


def discover_vendor_indexes() -> List[Tuple[str, str]]:
    """Return (vendor_name, index_path) for every ``<Vendor>.json`` that has a
    matching ``<Vendor>/`` directory beside it (the bundle layout)."""
    out: List[Tuple[str, str]] = []
    for fname in sorted(os.listdir(PROFILES_DIR)):
        if not fname.endswith(".json"):
            continue
        vendor = fname[:-5]
        idx_path = os.path.join(PROFILES_DIR, fname)
        vendor_dir = os.path.join(PROFILES_DIR, vendor)
        if os.path.isdir(vendor_dir):
            out.append((vendor, idx_path))
    return out


def build_catalog() -> Catalog:
    cat = Catalog()

    for vendor_name, idx_path in discover_vendor_indexes():
        try:
            with open(idx_path, encoding="utf-8") as fh:
                index = json.load(fh)
        except (OSError, ValueError) as exc:
            cat.warnings.append(f"index {idx_path}: {exc}")
            continue
        if not isinstance(index, dict):
            continue

        vendor_dir = os.path.join(PROFILES_DIR, vendor_name)
        machine_store = ProfileStore(vendor_dir, "machine")
        process_store = ProfileStore(vendor_dir, "process")
        cat.warnings.extend(machine_store.parse_errors)
        cat.warnings.extend(process_store.parse_errors)

        vendor_id = cat.add_vendor(vendor_name)

        # ---- base processes (process_list) --------------------------------
        # name -> process row id, so variants can resolve a default later.
        for entry in index.get("process_list", []) or []:
            if not isinstance(entry, dict):
                continue
            pname = entry.get("name")
            if not pname:
                continue
            sub = entry.get("sub_path")
            lh = layer_height_for_process(process_store, pname)
            cat.add_process(vendor_id, pname, sub, lh)

        # ---- machine MODELS (machine_model_list) --------------------------
        # Build model identity + remember setting_id/model_id for grouping.
        # We key models by name; a variant's printer_model points back here.
        model_id_by_name: Dict[str, int] = {}
        model_setting_by_name: Dict[str, Optional[str]] = {}
        for entry in index.get("machine_model_list", []) or []:
            if not isinstance(entry, dict):
                continue
            mname = entry.get("name")
            if not mname:
                continue
            sub = entry.get("sub_path")
            setting_id = None
            if sub:
                mp_path = os.path.join(vendor_dir, sub)
                mp = _safe_load(mp_path)
                if mp:
                    # machine_model files carry model_id; the instantiable
                    # variant carries setting_id. Prefer model_id here.
                    setting_id = mp.get("model_id") or mp.get("setting_id")
            mid = cat.add_machine(vendor_id, mname, setting_id)
            model_id_by_name[mname] = mid
            model_setting_by_name[mname] = setting_id

        # ---- machine VARIANTS (machine_list) ------------------------------
        # Only instantiable type:"machine" presets are real, selectable
        # printers. Group each under its printer_model; synthesize a model row
        # if the index's machine_model_list did not declare it.
        for entry in index.get("machine_list", []) or []:
            if not isinstance(entry, dict):
                continue
            sub = entry.get("sub_path")
            if not sub:
                continue
            mp_path = os.path.join(vendor_dir, sub)
            mp = _safe_load(mp_path)
            if mp is None:
                cat.warnings.append(f"missing/invalid machine file: {mp_path}")
                continue
            if mp.get("type") != "machine":
                continue  # skip fdm_* common/abstract bases
            if str(mp.get("instantiation", "")).lower() != "true":
                continue  # skip non-instantiable intermediates

            vname = mp.get("name") or os.path.splitext(os.path.basename(sub))[0]

            # Resolve fields through the inherits chain (within this vendor).
            nozzle = parse_float(machine_store.resolve(vname, "nozzle_diameter"))
            max_lh = parse_float(machine_store.resolve(vname, "max_layer_height"))
            area = machine_store.resolve(vname, "printable_area")
            bed = bed_size_from_printable_area(area)
            default_proc = first_scalar(
                machine_store.resolve(vname, "default_print_profile"))
            printer_model = first_scalar(
                machine_store.resolve(vname, "printer_model"))

            # Cornering jerk ceiling. OrcaSlicer warns ("jerk exceeds machine
            # maximum") and silently auto-caps when a process jerk exceeds the
            # machine's machine_max_jerk. Resolve BOTH axes through the inherits
            # chain and keep the binding (smaller) one, so a generated process
            # can be clamped to never trip that warning on this printer. A value
            # of 0 means a junction-deviation machine (classic jerk disabled) —
            # left as 0, which the app reads as "no classic-jerk ceiling".
            jerk_x = parse_float(machine_store.resolve(vname, "machine_max_jerk_x"))
            jerk_y = parse_float(machine_store.resolve(vname, "machine_max_jerk_y"))
            jerks = [j for j in (jerk_x, jerk_y) if j is not None]
            max_jerk = min(jerks) if jerks else None

            # Link the variant to its model row (create one if the model list
            # omitted it, which happens for a few klipper/community bundles).
            model_key = printer_model or vname
            if model_key in model_id_by_name:
                machine_id = model_id_by_name[model_key]
            else:
                setting_id = first_scalar(
                    machine_store.resolve(vname, "setting_id"))
                machine_id = cat.add_machine(vendor_id, model_key, setting_id)
                model_id_by_name[model_key] = machine_id

            # If a default process is named but absent from process_list, still
            # record the name (the generator can resolve the file directly).
            if default_proc and not any(
                p["vendor_id"] == vendor_id and p["name"] == default_proc
                for p in cat.processes
            ):
                # Best-effort: capture its layer height too.
                lh = layer_height_for_process(process_store, default_proc)
                sub_guess = None
                pp = process_store.path_by_name.get(default_proc)
                if pp:
                    # Path relative to the vendor dir (handles process/SUBDIR/*).
                    sub_guess = os.path.relpath(
                        pp, vendor_dir).replace("\\", "/")
                cat.add_process(vendor_id, default_proc, sub_guess, lh)

            rel_path = os.path.relpath(mp_path, REPO_ROOT).replace("\\", "/")
            cat.add_variant(
                machine_id,
                nozzle_diameter=nozzle,
                bed_size=bed,
                max_layer_height=max_lh,
                default_process_name=default_proc,
                machine_profile_path=rel_path,
                max_jerk=max_jerk,
            )

    return cat


def _safe_load(path: str) -> Optional[Dict[str, Any]]:
    if not os.path.exists(path):
        return None
    try:
        with open(path, encoding="utf-8") as fh:
            data = json.load(fh)
        return data if isinstance(data, dict) else None
    except (OSError, ValueError):
        return None


# ----------------------------------------------------------------------------
# Persistence
# ----------------------------------------------------------------------------
SCHEMA = """
CREATE TABLE vendors (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL
);
CREATE TABLE machines (
    id         INTEGER PRIMARY KEY,
    vendor_id  INTEGER NOT NULL REFERENCES vendors(id),
    model_name TEXT NOT NULL,
    setting_id TEXT
);
CREATE TABLE machine_variants (
    id                   INTEGER PRIMARY KEY,
    machine_id           INTEGER NOT NULL REFERENCES machines(id),
    nozzle_diameter      REAL,
    bed_size             TEXT,
    max_layer_height     REAL,
    default_process_name TEXT,
    machine_profile_path TEXT,
    max_jerk             REAL
);
CREATE TABLE base_processes (
    id           INTEGER PRIMARY KEY,
    vendor_id    INTEGER NOT NULL REFERENCES vendors(id),
    name         TEXT NOT NULL,
    sub_path     TEXT,
    layer_height REAL
);
CREATE INDEX idx_machines_vendor    ON machines(vendor_id);
CREATE INDEX idx_variants_machine   ON machine_variants(machine_id);
CREATE INDEX idx_processes_vendor   ON base_processes(vendor_id);
"""


def write_sqlite(cat: Catalog, path: str) -> None:
    if os.path.exists(path):
        os.remove(path)
    conn = sqlite3.connect(path)
    try:
        conn.executescript(SCHEMA)
        conn.executemany(
            "INSERT INTO vendors(id, name) VALUES (:id, :name)", cat.vendors)
        conn.executemany(
            "INSERT INTO machines(id, vendor_id, model_name, setting_id) "
            "VALUES (:id, :vendor_id, :model_name, :setting_id)", cat.machines)
        conn.executemany(
            "INSERT INTO machine_variants(id, machine_id, nozzle_diameter, "
            "bed_size, max_layer_height, default_process_name, "
            "machine_profile_path, max_jerk) VALUES (:id, :machine_id, "
            ":nozzle_diameter, :bed_size, :max_layer_height, "
            ":default_process_name, :machine_profile_path, :max_jerk)",
            cat.variants)
        conn.executemany(
            "INSERT INTO base_processes(id, vendor_id, name, sub_path, "
            "layer_height) VALUES (:id, :vendor_id, :name, :sub_path, "
            ":layer_height)", cat.processes)
        conn.commit()
    finally:
        conn.close()


def write_json(cat: Catalog, path: str) -> None:
    doc = {
        "_meta": {
            "generator": "scripts/build_machine_catalog.py",
            "source": "resources/profiles",
            "counts": {
                "vendors": len(cat.vendors),
                "machines": len(cat.machines),
                "machine_variants": len(cat.variants),
                "base_processes": len(cat.processes),
            },
        },
        "vendors": cat.vendors,
        "machines": cat.machines,
        "machine_variants": cat.variants,
        "base_processes": cat.processes,
    }
    with open(path, "w", encoding="utf-8") as fh:
        json.dump(doc, fh, indent=2, ensure_ascii=False)


# ----------------------------------------------------------------------------
# Reporting + validation
# ----------------------------------------------------------------------------
def print_summary(cat: Catalog) -> None:
    by_vendor_id = {v["id"]: v["name"] for v in cat.vendors}
    machines_per_vendor: Dict[int, int] = {}
    for m in cat.machines:
        machines_per_vendor[m["vendor_id"]] = \
            machines_per_vendor.get(m["vendor_id"], 0) + 1
    machine_to_vendor = {m["id"]: m["vendor_id"] for m in cat.machines}
    variants_per_vendor: Dict[int, int] = {}
    for var in cat.variants:
        vid = machine_to_vendor.get(var["machine_id"])
        if vid is not None:
            variants_per_vendor[vid] = variants_per_vendor.get(vid, 0) + 1
    processes_per_vendor: Dict[int, int] = {}
    for p in cat.processes:
        processes_per_vendor[p["vendor_id"]] = \
            processes_per_vendor.get(p["vendor_id"], 0) + 1

    print("\n=== Machine catalog summary (per vendor) ===")
    header = f"{'Vendor':<22}{'Models':>8}{'Variants':>10}{'Processes':>11}"
    print(header)
    print("-" * len(header))
    for vid in sorted(by_vendor_id, key=lambda i: by_vendor_id[i].lower()):
        print(f"{by_vendor_id[vid][:22]:<22}"
              f"{machines_per_vendor.get(vid, 0):>8}"
              f"{variants_per_vendor.get(vid, 0):>10}"
              f"{processes_per_vendor.get(vid, 0):>11}")
    print("-" * len(header))
    print(f"{'TOTAL':<22}{len(cat.machines):>8}"
          f"{len(cat.variants):>10}{len(cat.processes):>11}")

    # Coverage diagnostics — how many variants resolved each key.
    n = len(cat.variants) or 1
    res_nozzle = sum(1 for v in cat.variants if v["nozzle_diameter"] is not None)
    res_bed = sum(1 for v in cat.variants if v["bed_size"])
    res_lh = sum(1 for v in cat.variants if v["max_layer_height"] is not None)
    res_proc = sum(1 for v in cat.variants if v["default_process_name"])
    res_jerk = sum(1 for v in cat.variants if v.get("max_jerk") is not None)
    print("\n=== Variant field resolution ===")
    print(f"  nozzle_diameter      : {res_nozzle}/{n} ({100*res_nozzle//n}%)")
    print(f"  bed_size             : {res_bed}/{n} ({100*res_bed//n}%)")
    print(f"  max_layer_height     : {res_lh}/{n} ({100*res_lh//n}%)")
    print(f"  default_process_name : {res_proc}/{n} ({100*res_proc//n}%)")
    print(f"  max_jerk             : {res_jerk}/{n} ({100*res_jerk//n}%)")
    proc_lh = sum(1 for p in cat.processes if p["layer_height"] is not None)
    pn = len(cat.processes) or 1
    print(f"  process layer_height : {proc_lh}/{pn} ({100*proc_lh//pn}%)")

    if cat.warnings:
        shown = cat.warnings[:10]
        print(f"\n=== Warnings ({len(cat.warnings)} total, first {len(shown)}) ===")
        for w in shown:
            print(f"  - {w}")


def validate_sqlite(path: str) -> None:
    print("\n=== Re-opening sqlite to validate row counts ===")
    conn = sqlite3.connect(path)
    try:
        for table in ("vendors", "machines", "machine_variants",
                      "base_processes"):
            (count,) = conn.execute(
                f"SELECT COUNT(*) FROM {table}").fetchone()
            print(f"  {table:<18}: {count} rows")
        # A spot-check join proving variant->machine->vendor wiring.
        sample = conn.execute(
            "SELECT v.name, m.model_name, mv.nozzle_diameter, mv.bed_size, "
            "       mv.default_process_name "
            "FROM machine_variants mv "
            "JOIN machines m ON m.id = mv.machine_id "
            "JOIN vendors  v ON v.id = m.vendor_id "
            "WHERE m.model_name LIKE '%U1%' "
            "ORDER BY mv.nozzle_diameter LIMIT 4").fetchall()
        if sample:
            print("\n  spot-check (Snapmaker U1 variants):")
            for row in sample:
                print(f"    {row[0]} | {row[1]} | nozzle={row[2]} | "
                      f"bed={row[3]} | proc={row[4]}")
    finally:
        conn.close()


def main() -> int:
    if not os.path.isdir(PROFILES_DIR):
        print(f"ERROR: profiles dir not found: {PROFILES_DIR}", file=sys.stderr)
        return 2
    os.makedirs(OUT_DIR, exist_ok=True)

    cat = build_catalog()
    write_sqlite(cat, SQLITE_PATH)
    write_json(cat, JSON_PATH)

    print_summary(cat)
    print(f"\nWrote {os.path.relpath(SQLITE_PATH, REPO_ROOT)}")
    print(f"Wrote {os.path.relpath(JSON_PATH, REPO_ROOT)}")
    validate_sqlite(SQLITE_PATH)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
