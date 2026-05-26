# LeanSpectrum changelog

Changes added by the LeanSpectrum fork on top of Snapmaker_Orca /
OrcaSlicer-FullSpectrum. Upstream changes are tracked separately in
the OrcaSlicer release notes and not duplicated here.

## [unreleased] — `feature/filament-economy` branch

### Added

**Post-slicing filament economy (5-pass G-code optimiser)**
- Pass 1 — remove no-op tool changes + the surrounding orphan
  CP TOOLCHANGE block (walks backward 80 / forward 200 lines for
  START / END markers)
- Pass 2 — shrink wipe-tower purges based on per-extruder idle time;
  ratio grows from `shrink_purge_pct` (recent reuse) to 1.0 (long idle)
- Pass 3 — collapse back-to-back retract + unretract pairs with zero
  XY motion between
- Pass 4 — curvature-aware adaptive E scaling with first-layer guard
  (skip Z ≤ modal layer height + 1 µm) and mass-conservation rollback
  if observed reduction exceeds the cap + 1 %
- Pass 5 — verification gate: M82 → M83 conversion (I1), volumetric
  flow check against per-material safety factor (I5), post-pass
  invariants on retract count + cumulative volume (I2) and total
  extrusion vs (input − stats.extrusion_saved_mm) within
  `mass_tolerance_pct` (I3); reverts to input snapshot on any failure
- `filament_economy_*` config keys with sensible defaults
- 5 Catch2 test files covering each pass + integration

**BambuConvert — Bambu .3mf → Snapmaker U1 palette mapper**
- `src/libslic3r/Format/BambuConvert.{hpp,cpp}` — sRGB ↔ linear ↔ CIELAB
  (D65 illuminant), CIEDE2000 (Sharma 2005 reference pairs in tests),
  linear-RGB midpoint mixing
- Three physical-slot selection strategies:
  - **Usage** — top-N by extrusion length, deterministic, fast
  - **Chromatic** — exhaustive C(N, 4) search minimising Σ overflow ΔE
  - **Balanced** — same search minimising Σ (ΔE × used_mm),
    correlates with visible impact on the print
- 19-ratio mixing grid (0.05–0.95 step 0.05); runtime cadence already
  0–100 integer so the finer ratios survive the round-trip
- `apply_bambu_to_u1_conversion()` adapter integrates BambuConvert
  into the bbs_3mf importer's PlateData path
- `MixedFilamentManager::load_bambu_convert_recipe()` hydrates the
  manager with custom mixed-filament rows from the recipe string
- Plater auto-picks the best of all 3 strategies by weighted ΔE and
  also flips `flush_into_infill = true`, `flush_into_support = true`
  and enables `DitherMode::FloydSteinberg` automatically post-conversion
- GUI: **File → Convert Bambu palette to Snapmaker U1...**

**FullSpectrumDither (F1 + F2)**
- F2 — 1D Floyd-Steinberg error diffusion for the per-layer A/B
  decision, replaces the rotated-Bresenham ordered dither on
  long-period periodic banding
- F1 — per-layer curvature gain in [-1, +1] that biases the dither
  threshold (positive = more transitions in detail-dense zones,
  negative = longer runs in flat zones)
- `MixedFilamentManager` opt-in via `set_dither_mode()` and
  `set_layer_curvature_field()` — Ordered remains the default

**Auto-Profile — one-click intent-driven settings**
- 5 intents × 9 polymer families = 45 settings bundles tuned to the
  Snapmaker U1 official wiki ceilings (max_vol 32 mm³/s, retract
  0.5–3 mm direct-drive, purge 40–60 mm³)
- Polymer detection by pattern-matching against `filament_type` string
- Writes `layer_height`, `wall_loops`, `top_shell_layers`,
  `bottom_shell_layers`, `sparse_infill_density`, `sparse_infill_pattern`,
  `outer_wall_speed`, `filament_max_volumetric_speed`,
  `filament_retraction_length`, `filament_retraction_speed`,
  `fan_max_speed`, `fan_min_speed`, scarf-seam knobs, DIP flush flags
- GUI: **File → Auto-generate profile...**

**Wave Overhangs (partial port from
dennisklappe/OrcaSlicer-WaveOverhangs)**
- Phase 1 — `src/libslic3r/WaveOverhangs/` algorithm module
  (1117 lines, 5 files) landed verbatim from upstream HEAD f6853e5a8d
- Phase 2 — 37 config keys + 3 enum maps (489 lines)
- Phase 3a — Layer + PerimeterGenerator Polygons output fields
- Phase 6 — ExtrusionPath flag propagation through copy / move /
  operator=
- **Phases 3b (PerimeterGenerator hook), 4 (G-code emission), and 5
  (GUI tab) deferred** — see `doc/leanspectrum/PORT_WAVE_OVERHANGS.md`
  for the 7-step landing plan

**SDS / TDS Importer companion app** (`tools/sds-importer/`)
- Tauri 2 + Rust desktop app, ~50k LOC
- 3 input modes: single PDF drop, vendor catalog crawler URL, local
  corpus browser (defaults to `~/Downloads/filament-corpus/`)
- Parses 1600+ real vendor PDFs across 50+ brands (Eryone, eSun,
  SUNLU, ROSA3D, Atome3D, Bambu Lab EU, Jayo, Prusament, …)
- 10 polymer families detected by CAS number + name patterns
- Generates Snapmaker_Orca filament profile JSON with `inherits` →
  U1-tuned base preset, extracts or backfills nozzle / bed temp / max
  volumetric speed, populates polymer-specific scarf seam settings
- Bilingual FR / EN UI with `localStorage` persistence
- Optional Tesseract OCR behind the `ocr` Cargo feature
- Cross-platform CI: `.AppImage` + `.deb` + `.rpm` for Linux,
  `.dmg` arm64 + Intel for macOS, `.msi` + NSIS `.exe` for Windows

**Tooling**
- `tools/bambu-3mf-probe/probe.py` — Python port of BambuConvert for
  dry-running real Bambu .3mf files without a slicer build, supports
  `--strategy {usage|chromatic|balanced|all}` and `--extra` synthetic
  overflow injection
- `doc/leanspectrum/FORKS_FEATURE_SURVEY.md` — survey of 9 active
  OrcaSlicer forks + ranked absorption candidates + Snapmaker U1
  official wiki reference values
- `doc/leanspectrum/PORT_WAVE_OVERHANGS.md` — file-precise port plan
  (26 files, 3008 lines, 7 steps)
- `doc/leanspectrum/PORT_DYNAMIC_INFILL_PURGING.md` — audit of
  OrcaSlicer's existing `flush_into_*` knobs

### Changed

- Slicer pipeline now invokes `FilamentEconomy::process()` after
  `run_post_process_scripts()` in `BackgroundSlicingProcess` and logs
  the savings stats (swaps removed, segments scaled, purges shrunk,
  extrusion saved, retracts removed, M83 conversion, max flow)
- `MixedFilamentManager::resolve()` consults `m_dither_mode` and the
  per-layer curvature field when `advanced_dithering` is active
- `bbs_3mf` adapter now supports the
  `apply_bambu_to_u1_conversion()` route for Bambu → U1 palette
  rewrites; idempotency guard refuses double conversion

### Decided not to absorb

- **G2 / G3 arc-fitting** — OrcaSlicer already has `enable_arc_fitting`
  + the `ArcFitter` class; the upstream tooltip explicitly recommends
  keeping it **off** on Klipper machines, which the U1 uses.
  Disabled by default, no port needed.
- **"Dynamic Infill Purging" community fork** — turned out to be
  hyping OrcaSlicer's existing `flush_into_infill` /
  `flush_into_support` / `flush_into_objects` mainline keys; the only
  LeanSpectrum action is the AutoProfile flipping these to `true`
  by default.

## [upstream baseline] — FullSpectrum ratdoux

See [Snapmaker Orca FullSpectrum](https://github.com/ratdoux/OrcaSlicer-FullSpectrum)
release notes for the virtual mixed-color filament foundation this
fork builds on.
