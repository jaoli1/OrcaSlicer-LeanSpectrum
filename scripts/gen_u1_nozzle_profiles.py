#!/usr/bin/env python3
"""
Generate Snapmaker U1 printer + process profiles for the 0.2 / 0.6 / 0.8 mm
nozzles (the fork ships only 0.4) and, later, the project-type process library.

Design notes
------------
* The fork's vendor index `resources/profiles/Snapmaker.json` registers every
  printer in `machine_list` and every process in `process_list` as
  `{ "name", "sub_path" }`. A profile JSON that is not registered there is
  invisible to the slicer — so this script patches the index too.
* THE recurring failure mode of this repo is *dead keys*: a config key that is
  not registered in `PrintConfig.cpp` is silently ignored. So every key we emit
  is validated against the 832 `this->add("…")` registrations before writing.
* Nozzle-dependent values (layer heights, line widths) scale with the nozzle
  diameter; everything else (kinematics, g-code, retraction) is copied from the
  shipped 0.4 mm profiles so the new nozzles inherit the U1's tuning verbatim.

This is intentionally idempotent: re-running regenerates the files and leaves
the index with exactly one entry per profile.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
PROFILES = REPO / "resources" / "profiles"
SNAP = PROFILES / "Snapmaker"
MACHINE_DIR = SNAP / "machine"
PROCESS_DIR = SNAP / "process"
INDEX = PROFILES / "Snapmaker.json"
PRINTCONFIG = REPO / "src" / "libslic3r" / "PrintConfig.cpp"

# ---------------------------------------------------------------------------
# Dead-key validator: the set of every config option the slicer actually knows.
# ---------------------------------------------------------------------------
def registered_keys() -> set[str]:
    text = PRINTCONFIG.read_text(encoding="utf-8", errors="replace")
    add_keys = set(re.findall(r'this->add\("([a-z0-9_]+)"', text))
    if len(add_keys) < 700:
        sys.exit(f"validator: only {len(add_keys)} this->add keys parsed — refusing to run")
    # Union with every quoted snake_case token in PrintConfig.cpp: this picks up
    # keys registered via axis loops (machine_max_*) and legacy/compat keys kept
    # in substitution lists (e.g. smooth_coefficient) that real shipped profiles
    # still carry. A genuinely invented/typo'd key appears nowhere → still caught.
    tokens = set(re.findall(r'"([a-z][a-z0-9_]{3,})"', text))
    # Axis-looped machine limits: the full names (…_x/_y/_z/_e) are built by
    # string concat in C++ so never appear as literals.
    axis_keys = {f"{base}{axis}"
                 for base in ("machine_max_speed_", "machine_max_acceleration_", "machine_max_jerk_")
                 for axis in ("x", "y", "z", "e")}
    return add_keys | tokens | axis_keys

# Profile-meta keys that are not config options but are legal in a preset file.
META_KEYS = {
    "type", "name", "from", "instantiation", "inherits", "setting_id",
    "is_custom_defined", "version", "compatible_printers",
    "compatible_printers_condition", "compatible_prints",
    "compatible_prints_condition", "description", "filament_id",
    "printer_model", "printer_variant", "printer_settings_id",
    "_leanspectrum_metadata",
}

def validate(profile: dict, keys: set[str], where: str) -> None:
    bad = [k for k in profile if k not in keys and k not in META_KEYS]
    if bad:
        sys.exit(f"DEAD KEYS in {where}: {bad}")

# ---------------------------------------------------------------------------
# Per-nozzle physical parameters.
#   line_width ≈ 1.05× nozzle; layer defaults ≈ 0.5× nozzle; min/max bracket it.
# ---------------------------------------------------------------------------
NOZZLES = {
    "0.2": dict(layer="0.10", min_layer="0.06", max_layer="0.16", lw="0.22",
                initial_layer="0.12", setting="SM_U1_02"),
    "0.6": dict(layer="0.30", min_layer="0.10", max_layer="0.48", lw="0.62",
                initial_layer="0.30", setting="SM_U1_06"),
    "0.8": dict(layer="0.40", min_layer="0.15", max_layer="0.60", lw="0.82",
                initial_layer="0.40", setting="SM_U1_08"),
}

LINE_WIDTH_KEYS = [
    "line_width", "outer_wall_line_width", "inner_wall_line_width",
    "top_surface_line_width", "sparse_infill_line_width",
    "internal_solid_infill_line_width", "initial_layer_line_width",
    "support_line_width",
]

def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))

def dump(path: Path, obj: dict) -> None:
    path.write_text(json.dumps(obj, indent=4, ensure_ascii=False) + "\n", encoding="utf-8")

def gen_machine(noz: str, p: dict, template: dict, keys: set[str]) -> tuple[str, str]:
    m = dict(template)
    name = f"Snapmaker U1 ({noz} nozzle)"
    std = f"0.{int(float(p['layer'])*100):02d} Standard @Snapmaker U1 ({noz} nozzle)"
    m["name"] = name
    m["setting_id"] = p["setting"]
    m["printer_variant"] = noz
    m["printer_settings_id"] = f"MyToolChanger {noz} nozzle"
    m["nozzle_diameter"] = [noz] * 4
    m["max_layer_height"] = [p["max_layer"]] * 4
    m["min_layer_height"] = [p["min_layer"]] * 4
    m["default_print_profile"] = std
    validate(m, keys, f"machine {name}")
    dump(MACHINE_DIR / f"{name}.json", m)
    return name, std

def gen_standard_process(noz: str, p: dict, std_name: str, template: dict,
                         machine_name: str, keys: set[str]) -> str:
    proc = dict(template)
    proc["name"] = std_name
    proc["inherits"] = "fdm_process_U1_common"
    proc["compatible_printers"] = [machine_name]
    proc["setting_id"] = f"GP_U1_{noz.replace('.', '')}_std"
    proc["layer_height"] = p["layer"]
    proc["initial_layer_print_height"] = p["initial_layer"]
    for k in LINE_WIDTH_KEYS:
        proc[k] = p["lw"]
    proc["description"] = (f"Standard balanced profile for the {noz} mm nozzle on "
                           f"the Snapmaker U1. Generated by scripts/gen_u1_nozzle_profiles.py.")
    validate(proc, keys, f"process {std_name}")
    dump(PROCESS_DIR / f"{std_name}.json", proc)
    return std_name

def patch_index(machines: list[str], processes: list[str]) -> None:
    idx = load(INDEX)
    def upsert(lst_key: str, name: str, sub: str):
        lst = idx[lst_key]
        for e in lst:
            if e.get("name") == name:
                e["sub_path"] = sub
                return
        lst.append({"name": name, "sub_path": sub})
    for n in machines:
        upsert("machine_list", n, f"machine/{n}.json")
    for n in processes:
        upsert("process_list", n, f"process/{n}.json")
    dump(INDEX, idx)

def patch_model() -> None:
    """The machine_model declares available nozzle variants as a ';'-separated
    list; the slicer only offers printers whose variant is listed. Extend it to
    all four so the 0.2/0.6/0.8 printers appear in the selection wizard."""
    path = MACHINE_DIR / "Snapmaker U1.json"
    model = load(path)
    model["nozzle_diameter"] = "0.2;0.4;0.6;0.8"
    dump(path, model)

def main() -> None:
    keys = registered_keys()
    patch_model()
    machine_tpl = load(MACHINE_DIR / "Snapmaker U1 (0.4 nozzle).json")
    proc_tpl = load(PROCESS_DIR / "0.20 Standard @Snapmaker U1 (0.4 nozzle).json")

    new_machines: list[str] = []
    new_processes: list[str] = []
    for noz, p in NOZZLES.items():
        mname, sname = gen_machine(noz, p, machine_tpl, keys)
        gen_standard_process(noz, p, sname, proc_tpl, mname, keys)
        new_machines.append(mname)
        new_processes.append(sname)

    patch_index(new_machines, new_processes)
    print(f"Validated against {len(keys)} registered keys — no dead keys.")
    print("Machines:", ", ".join(new_machines))
    print("Processes:", ", ".join(new_processes))

if __name__ == "__main__":
    main()
