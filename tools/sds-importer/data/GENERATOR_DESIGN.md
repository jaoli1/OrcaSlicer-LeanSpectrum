# Generalizing the profile generator (all printers / multi-format)

Status: design note. ADDITIVE — no existing code/profile is changed by this
document or by `machine_catalog.*` next to it.

## 0. Where we are today

`tools/sds-importer/src-tauri/src/profile.rs` generates **two** OrcaSlicer JSON
presets from an `ExtractedFilament` (defined in `lib.rs`):

1. a **filament** preset (`type:"filament"`), and
2. a companion **process** preset (`type:"process"`) carrying the fork's
   process-domain features (scarf seams, print speed, LeanSpectrum filament
   economy, color-mixing readiness).

It is hard-wired to the Snapmaker U1, 0.4 nozzle:

| Constant (`profile.rs`)        | Value                                      |
| ------------------------------ | ------------------------------------------ |
| `U1_PRINTER`                   | `Snapmaker U1 (0.4 nozzle)`                |
| `BASE_PROCESS_U1`              | `0.20 Standard @Snapmaker U1 (0.4 nozzle)` |
| `inherit_stub_for(polymer)`    | a fixed map PLA→`Snapmaker PLA SnapSpeed @U1`, PETG→`Snapmaker PETG @U1`, … |
| `PRESET_VERSION`               | `01.10.01.70` (must be a parseable 4-part Semver or the loader silently drops the preset) |

`ExtractedFilament` is format-neutral physical data (polymer, density, glass
transition, nozzle/bed temp min/max/recommended, print-speed min/max/recommended,
`max_flow_mm3_s`, `fan_enabled`). It carries **no** printer or slicer-format
assumptions — that is exactly the seam we widen below.

Two hard-won invariants from the current code must survive generalization:

- **Never emit an empty value.** `build_profile_json` inserts a key only when a
  finite value exists; emitting `[""]` would overwrite the inherited parent
  (the v0.1.10 blanking bug).
- **Domain matters.** `seam_*`/`scarf_*`/`filament_economy_*`/`*_speed` are
  PROCESS-domain keys and are silently ignored inside a *filament* preset; temps
  / flow / density are FILAMENT-domain. Each key must go in the right preset.

The generalization keeps both the two-preset split and these invariants; it only
replaces the four hard-wired constants with **catalog lookups** and adds a
**format backend**.

---

## (a) Picking the inherit / base process per vendor + nozzle

The catalog (`machine_catalog.sqlite`, built by
`scripts/build_machine_catalog.py`) is the lookup table. Relevant tables:

```
vendors(id, name)
machines(id, vendor_id, model_name, setting_id)
machine_variants(id, machine_id, nozzle_diameter, bed_size,
                 max_layer_height, default_process_name, machine_profile_path)
base_processes(id, vendor_id, name, sub_path, layer_height)
```

### Step 1 — resolve the target printer variant

The UI gives a vendor + model + nozzle (today it is implicitly "Snapmaker / U1 /
0.4"). Resolve the concrete variant row:

```sql
SELECT mv.*
FROM machine_variants mv
JOIN machines m ON m.id = mv.machine_id
JOIN vendors  v ON v.id = m.vendor_id
WHERE v.name = :vendor AND m.model_name = :model
  AND mv.nozzle_diameter = :nozzle;
```

This yields the **compatible-printer name** to put in `compatible_printers`
(it is the variant's preset name, recoverable from `machine_profile_path` /
the machine JSON `name`), the `bed_size`, the `max_layer_height`, and the
variant's `default_process_name`.

> Why a catalog and not name-munging: variant naming is **not** uniform.
> Snapmaker uses `Snapmaker U1 (0.4 nozzle)` (parenthesized); BBL/Prusa/Voron use
> `Bambu Lab X1 Carbon 0.4 nozzle` (no parens). The catalog resolves the real
> name from each machine JSON's `printer_model`/`printer_variant`, so the
> generator never has to guess the string.

### Step 2 — choose the **process** to inherit (`BASE_PROCESS` generalized)

Order of preference:

1. **The variant's `default_process_name`** — already nozzle-correct (e.g. U1
   0.4 → `0.20 Standard @Snapmaker U1 (0.4 nozzle)`, U1 0.8 → `0.40 Standard …
   (0.8 nozzle)`). This is the single best default and covers 99% of variants.
2. If null (e.g. "Generic Marlin Printer"), pick from `base_processes` for that
   vendor the row whose `layer_height` is closest to a sane fraction of the
   nozzle (≈ 0.5 × nozzle) **and** whose name/`compatible_printers` matches the
   variant — i.e. a "Standard/Optimal" balanced profile.
3. Last resort: any vendor `base_processes` row with `layer_height` ≈ 0.5×nozzle.

The chosen name goes verbatim into the process preset's `inherits`, replacing
the hard-wired `BASE_PROCESS_U1`.

### Step 3 — choose the **filament** parent to inherit (`inherit_stub_for` generalized)

`inherit_stub_for` is currently a static polymer→U1-leaf map. Generalize it to a
catalog-style lookup of the vendor's filament presets that are compatible with
the target variant, ranked:

1. A vendor-tuned leaf for this polymer compatible with the variant
   (e.g. `Snapmaker PETG @U1`, `Bambu PLA Basic @BBL X1C`). The machine_model
   JSON's `default_materials` list (seen in BBL/Snapmaker models) is a strong
   hint for the preferred leaf per printer.
2. Else the bundle's `Generic <POLYMER>` compatible with the variant
   (the current PC/PA fallback strategy, now generalized to all vendors).
3. Else the nearest thermal sibling that *is* compatible (current HIPS≈ABS,
   PP≈PETG rule). Since the real temps come from the data sheet and override the
   parent, only the hardware tuning (cooling / retraction / pressure advance) is
   inherited — so a thermal-sibling parent is safe.

> A future, additive extension to the catalog: a `base_filaments(vendor_id,
> name, sub_path, polymer, compatible_variant)` table populated from each
> `<Vendor>.json` `filament_list` + the filament JSON `filament_type` /
> `compatible_printers`. Then filament-parent selection becomes the same kind of
> SQL lookup as the process. Not built yet — out of scope for this pass.

Whatever name is chosen **must exist and be compatible**, exactly as the
`profile.rs` doc-comment warns: a dangling `inherits` resolves to nothing and the
preset falls back to bare defaults. The catalog guarantees existence because
every name it stores came from an on-disk profile.

---

## (b) How nozzle-dependent values scale

The data sheet gives nozzle-*independent* material physics (temperatures,
density, glass transition). Geometry-coupled values scale with **nozzle
diameter** `N` and the chosen **layer height** `h`. Anchors below are taken from
the shipped 0.4-nozzle Snapmaker/BBL processes (`line_width` 0.42, initial
0.5) and the per-nozzle `max_layer_height` recovered into the catalog
(0.4→0.32, 0.6→0.4–0.42, 0.8→0.56–0.6, 0.2→0.14).

| Quantity | Scaling rule | Domain / key |
| --- | --- | --- |
| **Line width** (general `line_width`) | ≈ `1.05 × N` (0.4→0.42). Clamp to `[N, 1.2×N]`. | process |
| Initial-layer line width | ≈ `1.25 × N` (0.4→0.5) for first-layer adhesion | process |
| Outer-wall line width | ≈ `1.0–1.05 × N` (slightly tighter for surface quality) | process |
| Inner-wall / sparse-infill width | ≈ `1.1 × N` (0.4→0.45) | process |
| **Layer-height range** | `min ≈ 0.2×N` (floor at the machine's `min_layer_height`, typ. 0.08), `max = catalog max_layer_height` (≈ `0.65–0.75 × N`). Default `h ≈ 0.5×N`. | process / machine |
| **Max volumetric flow** `filament_max_volumetric_speed` | Material-capped first (data sheet `max_flow_mm3_s` or polymer default). The *achievable* flow also rises with the melt zone of bigger nozzles, but the filament cap is the safe ceiling, so keep the material value and let speed×width×height stay under it. | filament |
| **Print speed** (`*_wall_speed`, `*_infill_speed`) | From the data sheet (process domain, as today). Effective flow = `speed × line_width × h` must stay ≤ max volumetric flow → when `N`/`h` grow, the generator should cap speed at `max_flow / (line_width × h)`. | process |
| Nozzle/bed **temperatures** | Do **not** scale with nozzle. Larger nozzles often want **+5–10 °C** (more mass/s), a small optional bump, never a multiplier. | filament |

Rule of thumb the generator can encode once: derive `line_width`, `h`-range and
the speed cap from `N` + catalog `max_layer_height`; take every temperature and
the flow ceiling straight from `ExtractedFilament` (nozzle-independent).

---

## (c) Multi-format export mapping (OrcaSlicer JSON vs PrusaSlicer INI)

Two structural differences dominate:

- **OrcaSlicer JSON**: per-key values are **arrays of strings** in *filament*
  presets (one entry per extruder, e.g. `"nozzle_temperature": ["220"]`),
  **plain string scalars** in *process*/*machine* presets
  (`"layer_height": "0.2"`, `"outer_wall_speed": "70"`). Bools are `"0"`/`"1"`.
  Multi-value machine keys (one per extruder) are string arrays.
- **PrusaSlicer / SuperSlicer INI**: flat `key = value`, **bare scalars**, no
  arrays for single-extruder data (multi-extruder uses comma/semicolon lists).
  Bools are `0`/`1`. Filament + print + printer settings live in separate
  preset *types* but the same INI key namespace.

Mapping table (✔ universal concept; value-format column notes the conversion):

| Concept | OrcaSlicer JSON key | OrcaSlicer value format | PrusaSlicer INI key | PrusaSlicer value format | Notes |
| --- | --- | --- | --- | --- | --- |
| Nozzle temp (other layers) | `nozzle_temperature` | `["220"]` array | `temperature` | `220` scalar | ✔ universal. Orca array→Prusa scalar. |
| Nozzle temp (first layer) | `nozzle_temperature_initial_layer` | `["220"]` | `first_layer_temperature` | `220` | ✔ universal |
| Min nozzle temp | `nozzle_temperature_range_low` | `["190"]` | *(no direct key)* | — | Orca-only (UI guard). Drop on INI export. |
| Max nozzle temp | `nozzle_temperature_range_high` | `["240"]` | *(no direct key)* | — | Orca-only. |
| Bed temp (other layers) | `hot_plate_temp` (+ `cool_/eng_/textured_plate_temp`) | `["60"]` × 4 plate types | `bed_temperature` | `60` scalar | ✔ concept universal. Orca has **per-plate** keys (set all four, as `profile.rs` does); Prusa has **one** `bed_temperature`. Collapse on export. |
| Bed temp (first layer) | `*_plate_temp_initial_layer` | `["60"]` × 4 | `first_layer_bed_temperature` | `60` | Same collapse. |
| Flow / extrusion mult. | `filament_flow_ratio` | `["0.98"]` (ratio ~1.0) | `extrusion_multiplier` | `0.98` | ✔ universal, **same semantics** (both ~1.0 multiplier). Array→scalar. |
| Max volumetric flow | `filament_max_volumetric_speed` | `["14"]` | `filament_max_volumetric_speed` | `14` | ✔ universal, **same key name**. Array→scalar. |
| Filament density | `filament_density` | `["1.24"]` | `filament_density` | `1.24` | ✔ universal, same name. |
| Filament diameter | `filament_diameter` | `["1.75"]` | `filament_diameter` | `1.75` | ✔ universal. |
| Glass-transition temp | `temperature_vitrification` | `["54"]` | *(no standard key)* | — | Orca-only (cooling logic). Drop on INI. |
| Filament type | `filament_type` | `["PLA"]` | `filament_type` | `PLA` | ✔ universal. |
| Filament vendor | `filament_vendor` | `["Acme"]` | `filament_vendor` | `Acme` | ✔ universal. |
| Cooling on/off | `fan_always_on` / `reduce_fan_stop_start_freq` | `["1"]` | `fan_always_on` | `1` | ✔ concept; per-slicer key spelling differs. |
| Min/max fan speed | `fan_min_speed` / `fan_max_speed` | `["100"]` | `min_fan_speed` / `max_fan_speed` | `100` | ✔ concept; **key names differ** (Orca `fan_*_speed` vs Prusa `*_fan_speed`). |
| **Layer height** | `layer_height` | `"0.2"` scalar (process) | `layer_height` | `0.2` | ✔ universal, same name. PROCESS domain. |
| First-layer height | `initial_layer_print_height` | `"0.25"` | `first_layer_height` | `0.25` | ✔ concept; **key names differ**. |
| General line width | `line_width` | `"0.42"` | `extrusion_width` | `0.42` | ✔ concept; **key names differ**. |
| Outer-wall line width | `outer_wall_line_width` | `"0.42"` | `external_perimeter_extrusion_width` | `0.42` | ✔ concept; names differ. |
| Inner-wall line width | `inner_wall_line_width` | `"0.45"` | `perimeter_extrusion_width` | `0.45` | ✔ concept; names differ. |
| Outer-wall speed | `outer_wall_speed` | `"70"` | `external_perimeter_speed` | `70` | ✔ concept; names differ. |
| Inner-wall speed | `inner_wall_speed` | `"80"` | `perimeter_speed` | `80` | ✔ concept; names differ. |
| Sparse-infill speed | `sparse_infill_speed` | `"85"` | `infill_speed` | `85` | ✔ concept; names differ. |
| Solid-infill speed | `internal_solid_infill_speed` | `"100"` | `solid_infill_speed` | `100` | ✔ concept; names differ. |
| First-layer speed | `initial_layer_speed` | `"50"` | `first_layer_speed` | `50` | ✔ concept; names differ. |
| Compatible printer | `compatible_printers` | `["Snapmaker U1 (0.4 nozzle)"]` array | `compatible_printers_condition` | boolean expr string | **Structurally different**: Orca = explicit name list; Prusa = a condition string (e.g. `printer_model=="MK4"`). Not a 1:1 value copy. |
| Preset name | `name` | `"…"` | *(INI section header / filename)* | — | Orca stores `name` in-doc; Prusa keys it by file/section. |
| Preset version gate | `version` | `"01.10.01.70"` | *(none required)* | — | **Orca-only and mandatory** (loader drops a preset with no parseable Semver). No INI analogue. |
| Inherit parent | `inherits` | `"<preset name>"` | `inherits` | `<preset name>` | ✔ both support inheritance; Prusa INI also uses `inherits`. |
| **Scarf / seam** group | `seam_slope_type`, `scarf_angle_threshold`, `scarf_joint_speed`, … | scalars (process) | *(no equivalent)* | — | **Orca-only feature.** No PrusaSlicer mapping → drop on INI export. |
| **Filament economy** group | `filament_economy_*` | `"0"`/`"1"`, scalars | *(no equivalent)* | — | **Fork-only.** Orca/this-fork only. |
| **Color mixing** | `mixed_filament_region_collapse` | `"1"` | *(no equivalent)* | — | **Fork-only.** |

### Format-backend shape (suggested, additive)

Keep `ExtractedFilament` + the scaling logic format-neutral; add a thin
`trait ProfileFormat` with two implementations:

- `OrcaJson` — emits the current JSON (filament arrays-of-strings + process
  scalars), unchanged behaviour for the U1 path.
- `PrusaIni` — emits flat `key = value` INI, **scalars**, using the right-hand
  key column above; **skips** the "Orca-only / fork-only" rows
  (`version`, `temperature_vitrification`, `nozzle_temperature_range_*`, scarf,
  economy, mixing) and **collapses** the four plate-temp keys to the single
  `bed_temperature` / `first_layer_bed_temperature`.

Universal keys (same name + same semantics, only array↔scalar differs):
`filament_max_volumetric_speed`, `filament_density`, `filament_diameter`,
`filament_type`, `filament_vendor`, `layer_height`, `inherits`. These are the
safe core; everything else needs the key-rename and/or domain handling above.
