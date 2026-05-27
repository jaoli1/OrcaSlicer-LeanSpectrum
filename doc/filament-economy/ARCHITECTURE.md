# LeanSpectrum — Filament Economy Module

> Post-slicing filament economy optimizer for Snapmaker U1 with FullSpectrum
> mixed-color support.

## Goal

Reduce filament waste in multi-color prints on the Snapmaker U1 by adding a
post-slicing optimization pass that runs after the standard G-code is
generated.

The U1 already eliminates inter-tool purges via SnapSwap (tool changer).
However when FullSpectrum mixed-color filaments are used, the slicer
alternates between two physical filaments layer-by-layer. Even with SnapSwap,
this generates:

1. **Wipe-tower extrusions** that may be larger than necessary because the
   slicer assumes worst-case purge volumes.
2. **Redundant tool changes** when consecutive layers happen to use the same
   physical filament (e.g. with bias settings, identical mixed pairs, etc.).
3. **Travel-and-retract overhead** around each swap (extra retracts/un-retracts
   and zig-zags that can be merged).

The Filament Economy module performs a deterministic G-code rewrite that
eliminates these inefficiencies without changing the visible print result.

## Non-goals

- The module does not change print quality settings, geometry, or layer
  height.
- It does not modify the FullSpectrum mixing algorithm itself (handled in
  `MixedFilament.cpp` and `filament_mixer.cpp`).
- It does not target single-material prints — those have nothing to gain
  here.

## Architecture

```
+--------------------+         +-----------------------+
|   Slicing engine   |         |   Wipe tower writer   |
|   (Print.cpp)      |-------->|   (WipeTower2.cpp)    |
+--------------------+         +-----------------------+
                                          |
                                          v
                                +--------------------+
                                |   GCode generator   |
                                |   (GCode.cpp)       |
                                +--------------------+
                                          |
                                          v
                          +-----------------------------+
                          |  FilamentEconomy (NEW)      |
                          |  --------------------------  |
                          |  Pass 1: detect no-op swaps |
                          |  Pass 2: shrink purge vol.  |
                          |  Pass 3: merge travel       |
                          +-----------------------------+
                                          |
                                          v
                                +--------------------+
                                |  PostProcessor      |
                                |  (external scripts) |
                                +--------------------+
                                          |
                                          v
                                  output.gcode
```

The module sits between `GCode::do_export()` writing the file and
`run_post_process_scripts()` running user scripts. It reads the produced
G-code, applies the passes in order, and writes back to the same file.

## Optimization passes

### Pass 1 — No-op swap detection

For each tool change (`T<n>`), look back to the previous tool change on the
same logical filament *slot* (mixed-pair component). If the swap doesn't
actually change the physical extruder, remove the entire swap block (T-line,
purge, wipe-tower segment for that swap).

This pass is the highest-value: a single avoided swap typically saves
50–200 mm of extruded filament on the wipe tower plus 5–10 seconds.

### Pass 2 — Adaptive purge volume

The slicer uses a fixed purge volume per (from, to) filament pair. After
slicing, we know:

- How long the previous extruder sat idle (Z-distance since last use)
- How much the current extruder will print in the next layer

We can reduce the purge in two cases:

- **Short idle time** — the nozzle didn't cool fully, less ooze.
- **Same color family** — perceptually close colors hide bleed-through.

Reduction is capped (default 30 %) and configurable.

### Pass 3 — Travel and retract merging

Around each kept swap, collapse:

- Repeated retract → unretract → retract sequences into a single retract.
- Two consecutive travel moves with no extrusion in between into one
  diagonal move (when avoid-crossing is off for that segment).

## File layout

```
src/libslic3r/GCode/
├── FilamentEconomy.hpp            # public API + config struct
├── FilamentEconomy.cpp            # passes implementation
└── FilamentEconomy/               # (later) per-pass split if it grows
    ├── PassNoopSwap.cpp
    ├── PassPurgeShrink.cpp
    └── PassTravelMerge.cpp

src/libslic3r/PrintConfig.cpp      # add filament_economy_enable etc.
src/libslic3r/GCode.cpp            # call FilamentEconomy::process() at end
tests/fff_print/test_filament_economy.cpp   # Catch2 tests with sample G-code

doc/filament-economy/
├── ARCHITECTURE.md                # this file
├── GCODE_PATTERNS.md              # patterns to match/rewrite
└── BENCHMARKS.md                  # measured savings on test prints
```

## Settings (PrintConfig)

| Key                                  | Type   | Default | Description                                           |
|--------------------------------------|--------|---------|-------------------------------------------------------|
| `filament_economy_enable`            | bool   | true    | Master switch                                          |
| `filament_economy_remove_noop_swaps` | bool   | true    | Enable Pass 1                                          |
| `filament_economy_shrink_purge`      | bool   | true    | Enable Pass 2                                          |
| `filament_economy_shrink_purge_pct`  | int    | 30      | Max % a purge can be reduced (Pass 2)                  |
| `filament_economy_merge_travel`      | bool   | false   | Enable Pass 3 (off by default — needs more validation) |

## Integration points

1. **`src/libslic3r/GCode.cpp`** — after the final write of the G-code file
   (`gcode_path` is known), and before any external post-process script is
   run, invoke `FilamentEconomy::process(gcode_path, m_config)`.
2. **`src/libslic3r/PrintConfig.cpp`** — register the 5 settings above with
   defaults, UI categories, and tooltips. UI category: *Multi-material →
   Filament economy* (new sub-section).
3. **`src/libslic3r/CMakeLists.txt`** — add `GCode/FilamentEconomy.cpp` to
   the `libslic3r_sources` list.
4. **`src/slic3r/GUI/Tab.cpp`** — wire the 5 settings into the Filament tab
   under a new collapsible *Filament economy* group.

## Testing

- Unit tests under `tests/fff_print/test_filament_economy.cpp` use sample
  G-code files placed in `tests/data/filament_economy/`:
  - `input_2color_bias.gcode` — FullSpectrum mixed print with bias > 0
  - `input_2color_balanced.gcode` — 1:1 ratio
  - `input_singlematerial.gcode` — single material (should be a no-op)
- Each test slices a fixture, runs the module, and asserts:
  - Final position/extruder match before/after
  - No removed `T<n>` left a dangling reference
  - Total extruded length is `<` original for multi-material cases
  - Total extruded length is `==` original for single material

## Risks

- **G-code parsing fragility** — Snapmaker_Orca uses a specific G-code
  flavor with custom comments (`; FEATURE:`, `; WIPE:`, etc.). The module
  must respect these markers when rewriting.
- **Wipe tower coupling** — the wipe tower is generated as a separate
  geometric object before the final G-code. Trimming purges in the linear
  G-code might desynchronize from the planned tower geometry. Mitigation:
  Pass 2 reduces extrusion *rate* (E values), not motion, so the head still
  traces the tower outline.
- **Multi-tool sequences** — U1 has 4 tools. Two consecutive same-tool
  events can also originate from infill-vs-perimeter swaps inside one tool;
  Pass 1 must only collapse swaps that cross logical filament boundaries.

## Versioning

The module is versioned independently from FullSpectrum and OrcaSlicer:

- Base: FullSpectrum 0.9.9
- Module: `leanspectrum_economy_version` = 0.1.0 (skeleton)
