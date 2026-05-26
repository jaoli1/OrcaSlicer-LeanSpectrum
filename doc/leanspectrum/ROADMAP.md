# LeanSpectrum roadmap

Updated 2026-05-27.

## Done (shipped on `feature/filament-economy`)

### Post-slicing G-code economy module (5 passes + safety net)
- Pass 1 — no-op tool swap removal with orphan wipe-tower block strip
- Pass 2 — wipe-tower purge shrink based on per-extruder idle time
- Pass 3 — back-to-back retract collapse (conservative)
- Pass 4 — curvature-aware E scaling, first-layer guard, mass-conservation rollback
- Pass 5 — verification gate: M82→M83, volumetric flow cap, post-pass I2/I3 + revert
- Catch2 suite covering all 5 passes + integration
- Pipeline hook in BackgroundSlicingProcess + stats logging

### BambuConvert — Bambu .3mf → U1 palette mapper
- sRGB ↔ Lab (D65), CIEDE2000 (Sharma 2005 reference pairs)
- 3 strategies: Usage / Chromatic / Balanced
- 19-ratio discrete mixing grid (vs the 5-cadence default)
- bbs_3mf adapter (`apply_bambu_to_u1_conversion`)
- MixedFilamentManager hydration from recipe string
- GUI: **File → Convert Bambu palette to Snapmaker U1...**
- Python probe tool (`tools/bambu-3mf-probe/probe.py`) for dry-run

### FullSpectrum F1 + F2 dither
- 1D Floyd-Steinberg error diffusion (replaces rotated-Bresenham)
- Per-layer curvature gain ∈ [-1, +1]
- MixedFilamentManager opt-in (Ordered remains default)

### Auto-Profile — one-click intent profiles
- 5 intents × 9 polymer families = 45 sensible bundles
- Snapmaker U1 wiki-validated values (max_vol, retract, purge)
- DIP flush-into-infill / support flipped on by default
- GUI: **File → Auto-generate profile...**

### Wave Overhangs (partial — Phases 1, 2, 3a, 6 landed)
- Algorithm module (`src/libslic3r/WaveOverhangs/`, 1117 lines)
- 37 config keys + 3 enum maps
- Polygons output fields on Layer + PerimeterGenerator
- ExtrusionPath flag propagation through copy/move

### SDS / TDS Importer (Tauri 2 companion app)
- Validated against 1600+ vendor PDFs (50+ brands)
- 3 input modes (single PDF / catalog crawler / local corpus)
- Bilingual FR / EN, optional Tesseract OCR
- Cross-platform CI release bundles

## Next session — Wave Overhangs Phases 3b → 5

### Phase 3b — PerimeterGenerator hook (HARDEST remaining task)
Estimated 750-line integrated change across 5 files. Cannot be split
further without breaking the build mid-commit. Sequence:

1. `PerimeterGenerator.hpp` — `apply_extra_perimeters` signature gets
   a new `const ExPolygon& island_region` parameter.
2. `PerimeterGenerator.cpp` — `apply_extra_perimeters` callsite +
   the +379-line block that invokes `WaveOverhangs::generate()` and
   populates `out_wave_overhang_floor_polygons` /
   `out_wave_overhang_covered_polygons`.
3. `LayerRegion.cpp` — +11 lines moving the per-region outputs into
   the parent Layer's wave fields.
4. `Print.hpp` — 3 method forward declarations
   (`apply_wave_overhang_floor_layer_authority`,
   `apply_wave_overhang_bridge_suppression`,
   `tag_wave_overhang_perimeters`).
5. `PrintObject.cpp` — +343 lines for the seed-layer detection +
   the three new methods declared in step 4.

Verification: build_all green, then a synthetic 45° overhang STL
sliced with `wave_overhangs = true` should produce wave-pattern
perimeters in the G-code (visible via the slicer's preview).

### Phase 4 — G-code emission + support exclusions
Smaller integrated scope than 3b. ~444 lines across 5 files. Each
file is mostly independent:

- `GCode.cpp` (+267 lines) — emit `;WAVE_OVERHANG_START/END`
  markers, apply wave-overhang fan/speed overrides per ExtrusionPath
  flag
- `GCodeWriter.cpp` (+24 lines) — end-of-line retraction in wave
  segments
- `CoolingBuffer.cpp` (+84 lines) — aux fan override during wave
  layers
- `Support/SupportMaterial.cpp` (+21 lines) — skip ordinary supports
  inside `wave_overhang_covered_polygons`
- `Support/TreeSupport.cpp` (+36 lines) — same for tree supports
- **New**: `src/libslic3r/GCode/FilamentEconomy.cpp` — add
  `FeatureType::WaveOverhang` with `feature_cap = 0.0` so Pass 4
  curvature scaling doesn't thin cantilevered wave traces

### Phase 5 — GUI tab + i18n
Mostly mechanical wxWidgets:

- `src/slic3r/GUI/Tab.cpp` — new "Wave overhangs" group with the
  37 settings organised under the 8 sub-groups (Detection, Pattern,
  Motion, Cooling, Floor, Corner, Debug, Master)
- `src/slic3r/GUI/ConfigManipulation.cpp` — validation logic
  (e.g. "corner_taper_distance must be > 0 when corner_taper_enable
  is true")
- `src/slic3r/GUI/AboutDialog.cpp` — credit Andersons / McCulloch /
  dennisklappe in the attribution panel (no rebrand)
- Translation strings (FR PO file at
  `localization/i18n/fr/Snapmaker_Orca_fr.po`)

## Future / unscheduled

### #21 — FullSpectrum 3-physical mixing (deferred)
Schema change on `bambu_convert_recipe` to encode 3-component
recipes. Lets brown / purple targets land closer than the current
2-physical mix can reach. Diminishing returns; only useful with
palettes that have a chromatic gap the 2-physical search can't
bridge.

### #22 — Snapmaker U1 reliability (specs needed)
Vague task. Concrete asks would be:
- Start / end g-code review against U1 firmware quirks
- Default filament profiles tuned to U1 published spec
- First-layer adhesion settings per polymer (some matters for
  ASA / PA / TPU on textured PEI)

### Bias Auto-Pick wizard (LeanSpectrum-original, not yet started)
UI to let users click a target color on a CIE-Lab grid and have
the slicer back-solve which existing physical pair + bias produces
the closest match. Extends FullSpectrum's existing bias mechanism.

### Helio thermal simulation (deferred, niche)
Voxel-based thermal solver for FullSpectrum prints. High complexity,
narrow audience. Skip until a user asks.

### IDEX copy/mirror (deferred, conflicts with FullSpectrum)
Print two copies of a part simultaneously on the U1's two heads.
Mutually exclusive with FullSpectrum mixing per layer — would need
to be a UI radio choice (Copy/Mirror vs Multi-color 4 extruders vs
FullSpectrum mixed).

## Decision log

- **G2/G3 arc-fitting** (PrusaSlicer #4352) — REJECTED for Klipper
  targets. OrcaSlicer has it but recommends keeping it off because
  Klipper re-splits arcs into segments, hurting surface quality.
  U1 runs Klipper. No port.
- **Dynamic Infill Purging** — REJECTED as port. OrcaSlicer
  mainline already has `flush_into_infill`, `flush_into_support`,
  `flush_into_objects`. AutoProfile flips the first two to true by
  default; rest is up to the user.
