# PR: Wave Overhangs Phases 3b + 4 + 5 — `experiment/wave-overhangs-phase3b` → `feature/filament-economy`

**Status template** — fill in CI links once `a68d678a5e` build_all completes.

## Summary

Lands the full dennisklappe Wave Overhangs port (~3 000 lines) on the
LeanSpectrum feature branch. Phases 1, 2, 3a and 6 already shipped on
`feature/filament-economy`; this PR adds:

- **Phase 3b** — `PerimeterGenerator` + `PrintObject` wavefront hooks
  (~750 lines)
- **Phase 4** — G-code emission + cooling buffer + support exclusions
  + `FilamentEconomy` wave-overhang feature cap
- **Phase 5** — `Tab.cpp` new "Wave overhangs" GUI page +
  `ConfigManipulation` visibility wiring

## Why merge into `feature/filament-economy` rather than `main`

`feature/filament-economy` is the integration branch where every
LeanSpectrum delta lands and gets validated end-to-end before any
release tag. `main` continues to track the slim Snapmaker_Orca base
until v0.1.0 ships.

## Validation

### Phase 3b alone (`781a40eea7`)
- Ubuntu 24.04: SUCCESS (07:01 UTC, 27 May)
- Windows latest: SUCCESS (07:08 UTC)
- macOS 14 arm64: SUCCESS (~07:45 UTC)
- Workflow conclusion: `cancelled` (superseded by Phase 4 push, but
  every Build Snapmaker_Orca job ran to success before the cancel)

### Phase 3b + 4 + 5 (`a68d678a5e`, fix on top of `8ab2e2d`)
- TBD — run `26506752470` is in flight at time of writing

The intermediate `8ab2e2d` failed identically on the 3 OS at
`src/libslic3r/GCode.cpp:2249` because dennisklappe's fork emits
`WAVE_OVERHANGS_VERSION` and `SoftFever_VERSION` as CMake-time
defines that Snapmaker_Orca does not. Fix `a68d678a5e` pins
`"0.5.0-leanspectrum-port"` as a literal and substitutes our
`SLIC3R_VERSION` macro (reachable through `libslic3r.h`).

## Files touched (vs `feature/filament-economy`)

| File | Δ lines | Phase |
|---|---:|---|
| `src/libslic3r/PerimeterGenerator.cpp` | +375 | 3b |
| `src/libslic3r/PerimeterGenerator.hpp` | +15 | 3b |
| `src/libslic3r/PrintObject.cpp` | +343 | 3b |
| `src/libslic3r/LayerRegion.cpp` | +11 | 3b |
| `src/libslic3r/Print.hpp` | +13 | 3b |
| `src/libslic3r/GCode.cpp` | +267 | 4 |
| `src/libslic3r/GCode.hpp` | +8 | 4 |
| `src/libslic3r/GCodeWriter.cpp` | +24 | 4 |
| `src/libslic3r/GCodeWriter.hpp` | +4 | 4 |
| `src/libslic3r/GCode/CoolingBuffer.cpp` | +84 | 4 |
| `src/libslic3r/GCode/FilamentEconomy.cpp` | +8 | 4 (wave_overhang FeatureType cap) |
| `src/libslic3r/Fill/Fill.cpp` | +78 | 4 |
| `src/libslic3r/Support/SupportMaterial.cpp` | +21 | 4 |
| `src/libslic3r/Support/TreeSupport.cpp` | +36 | 4 |
| `src/slic3r/GUI/Tab.cpp` | +67 | 5 |
| `src/slic3r/GUI/ConfigManipulation.cpp` | +67 | 5 |

Total: ~1 400 net additions across 16 files.

## Conflicts resolved during the port

- **PerimeterGenerator.cpp** — 2 Arachne `wall_direction` conflicts
  resolved as "ours". We have a `WallDirection` enum at a different
  callsite; keeping `extrusion_loop.make_counter_clockwise()` does the
  right thing in both layouts.
- **GCode.cpp** — 4 conflicts:
  1. `has_BTT_thumbnail` whitespace nit → ours
  2. `min_layer_time` G4 dwell → theirs (uses
     `m_wave_layer_accumulated_time`)
  3. Wave-overhang speed override → theirs, with explicit `F` recompute
     since our base already has `double F = speed * 60` before the block
  4. `travel_to_xyz` overload selection → theirs (uses `speed_override`)
- **ConfigManipulation.cpp** — skipped one unrelated dennisklappe line
  that depends on a `DevPrinterConfigUtil` class we don't carry; kept
  the wave-overhang visibility logic + our existing
  `infill_overhang_angle` toggle.

## Manual test plan

- [ ] Slice the `tests/handy_models/wave-overhang-90deg.stl` benchmark
      (if present, else use a 90° overhang test cube) with
      `wave_overhangs = true`; confirm no Pass-1/2 economy interference
      via the `WAVE_OVERHANG_BUILD` G-code header line.
- [ ] Toggle the "Wave overhangs" tab in Process settings; verify all
      ~20 sub-options appear / disappear together via the
      `ConfigManipulation` visibility hook.
- [ ] Run the existing `tests/fff_print/test_filament_economy.cpp`
      suite to confirm no regression on the 5-pass module.
- [ ] Re-slice a colour-swap multi-material print; verify Pass 5
      (volumetric flow ceiling) still rejects feedrate over-shoots —
      Wave Overhangs introduces a `feature_cap = 0.0` on
      `FeatureType::WaveOverhang` which must remain a no-touch zone for
      FilamentEconomy.

## Risk

- **Bounded** — the wave-overhang code path only activates with
  `wave_overhangs = true`, which defaults to false. Existing prints
  unaffected.
- **Off-by-default** — same toggle gate as Phases 1/2 from the
  original port.
- **Cooling-buffer change** is the one cross-cutting concern; the
  +84 line block in `CoolingBuffer.cpp` adds the
  `m_wave_layer_accumulated_time` accumulator that the min-layer-time
  override needs, but does not change non-wave layer cooling.

## Post-merge

- Update `doc/leanspectrum/ROADMAP.md` to flip Phases 3b/4/5 from
  "experimental" to "shipped"
- Drop `experiment/wave-overhangs-phase3b` once `feature/filament-economy`
  contains the merge
- Tag bump deferred until v0.1.0 release checklist runs (separate PR)
