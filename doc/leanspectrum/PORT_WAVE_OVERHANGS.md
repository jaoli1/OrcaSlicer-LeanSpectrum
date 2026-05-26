# Porting plan — Wave Overhangs

Status: **planning / not started**.
Source: [dennisklappe/OrcaSlicer-WaveOverhangs](https://github.com/dennisklappe/OrcaSlicer-WaveOverhangs)
(itself a port of stmcculloch/PrusaSlicer-WaveOverhangs).
Research basis: [Wavefront support-free overhang algorithm pre-print](https://doi.org/10.2139/ssrn.6640458).

## What it does

Generates curved internal "wave" perimeters that propagate outward
from a starting support-overhang interface, letting the slicer print
overhangs up to ~90 deg without external support material. The
trade-off is extra non-load-bearing material *inside* the part, but
overall filament use typically drops 15–40 % on parts with significant
overhangs because external support trees are eliminated.

Compounds nicely with our existing filament economy passes — Pass 1
(no-op tool swap removal) and Pass 2 (purge shrink) both operate on
the final G-code and are independent of which generator emitted the
perimeters.

## Algorithm summary

1. Identify the start contour (typically the slice where overhang
   begins — the layer above the last fully-supported layer).
2. Seed a wavefront on that contour.
3. Iteratively expand the wavefront inward by one line-spacing
   per iteration, applying:
   - Pattern mode: Smart / Monotonic / ZigZag (configurable).
   - Narrow-region split when the wavefront pinches.
   - Corner reinforcement at angle features.
4. Emit each iteration's wave as a perimeter ring with adjusted flow
   (so the curved geometry deposits the right cross-section).

The dennisklappe README documents `docs/ALGORITHMS.md` for the full
iteration flowchart.

## Settings (~20 in upstream WaveOverhangs)

Grouped under a `wave_overhangs_enable` master toggle (off by default):

| Group | Keys |
|---|---|
| Detection | start_angle_threshold, min_overhang_width |
| Pattern | pattern_mode (Smart / Monotonic / ZigZag), line_spacing, perimeter_overlap |
| Motion | max_iterations, narrow_region_min_width, smoothing_iterations |
| Flow | flow_mm3_per_mm, flow_compensation |
| Corner reinforcement | corner_angle_threshold, corner_reinforcement_count |
| Cooling | fan_boost_during_wave |
| Floor layers | floor_layer_count, floor_layer_flow |
| Debug | emit_wave_markers, dump_iteration_polygons |

The exact key names should mirror dennisklappe's verbatim so future
upstream sync is mechanical.

## Verified file scope (clone + diff vs upstream OrcaSlicer)

Cloned dennisklappe/OrcaSlicer-WaveOverhangs and diffed against the
merge base with OrcaSlicer/OrcaSlicer main. Numbers below are precise
as of dennisklappe HEAD f6853e5a8d (May 2026):

**Total**: 3008 lines added, 32 removed, across 26 files in
`src/libslic3r/`.

| File | Δ lines | Role |
|---|---:|---|
| `WaveOverhangs/WaveOverhangs.cpp` | +916 | Main algorithm — wavefront propagation, pattern modes (Smart / Monotonic / ZigZag), narrow-region split, corner reinforcement |
| `WaveOverhangs/WaveOverhangs.hpp` | +50 | Public API |
| `WaveOverhangs/AndersonsGenerator.cpp` | +55 | Andersons reference impl |
| `WaveOverhangs/AndersonsGenerator.hpp` | +25 | |
| `WaveOverhangs/IGenerator.hpp` | +71 | Generator interface |
| `PrintConfig.cpp` | +489 | ~20 setting definitions (Detection, Pattern, Motion, Cooling, Floor, Debug) |
| `PrintConfig.hpp` | +61 | Setting struct fields + accessor declarations |
| `PerimeterGenerator.cpp` | +379 | Hook — carve wall_loops out of wave-covered regions, emit waves |
| `PerimeterGenerator.hpp` | +15 | API extension |
| `PrintObject.cpp` | +343 | Pipeline integration — detect overhang seed layers |
| `GCode.cpp` | +267 | Emit wave markers + fan boost |
| `Fill/Fill.cpp` | +78 | Hilbert-curve floor over wave regions; preserve solid infill above stacked waves |
| `GCode/CoolingBuffer.cpp` | +84 | Aux fan override for wave regions |
| `Support/TreeSupport.cpp` | +36 | Skip tree supports inside wave-covered regions |
| `Support/SupportMaterial.cpp` | +21 | Same for ordinary supports |
| `GCodeWriter.cpp` | +24 | End-of-line retraction in wave segments |
| `Layer.hpp` | +24 | Layer-level wave region annotations |
| `Preset.cpp` | +18 | Round-trip the new keys |
| `LayerRegion.cpp` | +11 | Invoke wave generator from region |
| `Print.hpp` | +13 | Forward decls |
| `GCode.hpp` | +8 | API |
| `GCodeWriter.hpp` | +4 | API |

Plus 4 lines of GUI touch (AboutDialog, ConfigManipulation, GUI_App,
Plater) for attribution and validation strings.

## Porting steps (revised)

1. **Land the new module first** (`WaveOverhangs/` directory) as a
   self-contained unit. The 5 files there have *no* upstream
   dependencies — they're new C++ that can land without touching
   any existing path. Add to `CMakeLists.txt` but don't call from
   anywhere yet.
2. **Add the 20+ config keys** in `PrintConfig.cpp/hpp` defaulted to
   off / 0. Master toggle: `wave_overhangs_enable = false`. This
   commit is a no-op behaviorally but makes every existing test
   keep passing.
3. **Hook into `PerimeterGenerator`** — the +379-line diff is the
   most delicate change. Cherry-pick PR-by-PR following dennisklappe's
   own history (he stages each feature behind its own PR: e.g. PR #29
   "corner-aware spacing taper", PR #34 "Hilbert-curve floor", etc).
4. **Layer regions + pipeline integration** (`LayerRegion.cpp`,
   `PrintObject.cpp`) — these are tested by re-running the wave
   regression file from dennisklappe's repo.
5. **Support material exclusions** (`Support*.cpp`) — defensive,
   ensures no double-supporting where waves cover.
6. **G-code emission** (`GCode.cpp/hpp`, `GCodeWriter.cpp/hpp`,
   `CoolingBuffer.cpp`) — markers + retraction + fan.
7. **GUI** (`AboutDialog.cpp`, `ConfigManipulation.cpp`, `GUI_App.cpp`,
   `Plater.cpp`) — last, since the algorithm is testable without it.

## Compatibility with our existing passes

- **Pass 1 (no-op tool swap removal)** — operates on `T<n>` lines in
  the final G-code, blind to perimeter origin. No conflict.
- **Pass 2 (purge shrink)** — operates on `CP TOOLCHANGE` blocks.
  Wave Overhangs doesn't generate those. No conflict.
- **Pass 3 (retract collapse)** — Wave Overhangs adds end-of-line
  retracts in wave segments (PR #28). Pass 3 only collapses
  *back-to-back* retract+unretract — wave retracts have travel in
  between, so they pass through untouched. Verify with a wave
  regression file once ported.
- **Pass 4 (curvature E scaling)** — Wave Overhangs produces curved
  segments with high local curvature by design. Our Pass 4 already
  protects via the `feature_cap` per-region table — verify that
  wave segments get a Bridge-style cap (0 % reduction) so we don't
  thin the supportless overhangs. **This is a real interaction —
  add a new `FeatureType::WaveOverhang` enum value and feature_cap
  entry as part of step 6.**
- **Pass 5 (verification)** — mass conservation across the wave
  rewrites. Already accounts for `stats.extrusion_saved_mm`. Should
  pass without changes.
- **FullSpectrum + BambuConvert** — wave overhangs touch *where*
  plastic goes, FullSpectrum touches *which physical extruder* is
  active per layer. They commute. The chromatic / balanced strategy
  picks aren't affected.

## License + attribution

AGPL-3.0 carried through. Commit message must credit
- Steven McCulloch (original PrusaSlicer port)
- dennisklappe (OrcaSlicer port)
- The pre-print authors at doi.org/10.2139/ssrn.6640458

## Estimated effort

| Phase | Effort |
|---|---|
| Clone + diff identification | half day |
| Config keys + stub | 1 day |
| Algorithm body + tests | 3 to 5 days |
| GUI tab | 1 to 2 days |
| Cross-platform CI debugging | 1 day buffer |
| **Total** | **1.5 to 2 weeks** |

## Decision flag

Do NOT start the port until at least:
- The LeanSpectrum branch has a green build_all CI on the current
  feature set (currently still pending).
- A real-world Bambu .3mf round-trip print has been validated on a U1
  (sanity check that nothing else is broken before adding a feature
  with this much surface area).
