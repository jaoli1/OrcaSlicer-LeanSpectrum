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

## Porting steps

1. **Clone dennisklappe's fork locally** and diff against its
   upstream-merge tag (the README mentions ongoing rebase against
   SoftFever/OrcaSlicer main).
2. **Identify changed files**. Expected scope from the README:
   - `src/libslic3r/PerimeterGenerator.cpp` — new wavefront emitter
   - `src/libslic3r/PrintConfig.cpp` — config key definitions
   - `src/slic3r/GUI/Tab.cpp` — new "Wave overhangs" settings tab
   - `src/libslic3r/Layer*.cpp` — possibly seed-layer detection
   - Tests under `tests/libslic3r/`
3. **Sequence the import** as a series of small commits in this order
   so each step keeps the tree compilable:
   1. Add config keys (defaulted to off / 0).
   2. Add the perimeter-generator hook stub (no-op when disabled).
   3. Port the algorithm body and unit tests.
   4. Add the GUI tab (last, since it's wxWidgets and slowest to test).
4. **Adapt to FullSpectrum**: wave overhangs touch perimeter
   generation, FullSpectrum cadence touches *which* extruder a layer
   uses. They commute. No conflict expected, but spot-check Pass 1's
   no-op tool change detection against the new perimeter markers.
5. **Run our economy passes after**: Wave Overhangs emits more total
   line segments than a stock perimeter (smaller turns), so Pass 4
   (curvature E scaling) has more work to do — exactly the kind of
   regression test we want to add.

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
