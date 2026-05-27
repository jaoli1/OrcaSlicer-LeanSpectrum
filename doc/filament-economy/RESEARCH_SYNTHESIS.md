# Research synthesis — LeanSpectrum v2 roadmap

> Synthesises (a) Al-Juboori 2026 on post-slicing curvature-aware G-code
> optimization, (b) ratdoux's FullSpectrum mixed-color implementation,
> (c) josuanbn's bl2u1 Bambu→U1 .3mf converter, and (d) Snapmaker U1
> hardware constraints, into a concrete roadmap for LeanSpectrum.

## Anchor reference

Al-Juboori, L. (2026). *AI-assisted, curvature-aware post-slicing G-code
optimization for material-efficient FDM printing.* Journal of King Saud
University — Engineering Sciences, 38:35.
DOI: [10.1007/s44444-026-00109-y](https://doi.org/10.1007/s44444-026-00109-y)
(Open Access, CC-BY 4.0)

## Quantified findings we will exploit

| Mechanism                              | Effect on print                              | Effect on time |
|----------------------------------------|----------------------------------------------|----------------|
| Curvature-aware adaptive layer height  | **-16 %** effective extrusion (dominant)     | small +        |
| Feedrate smoothing in high-curvature   | -31 % Ra (surface), -27 % Rz (roughness)     | +6 % .. +12 %  |
| Localized extrusion reinforcement      | +12 % peak load, +9 % stiffness (load zones) | negligible     |
| AI weak-zone detection (k-means k=5)   | +23 % detected zones vs. geometric rules     | negligible     |
| **Combined (ablation, full pipeline)** | **-25 % material**, RMS deviation < 0.05 mm  | **+7..9 %**    |

Validated on 3 geometries (Ender-3, Nylon, Cura 5.0 baseline). The paper's
authors note: results are indicative, not universal. Our work extends to
U1 + FullSpectrum which the paper does not cover.

## Mapping to LeanSpectrum passes

The skeleton in `FilamentEconomy.cpp` already exposes a multi-pass
architecture. We extend it to seven passes total, organised in three
tiers by reliability and ROI:

### Tier A — high-confidence, high-ROI (implement now)

1. **Pass 1 — Remove no-op tool changes** (already implemented, naïve)
   *Roadmap*: extend to also strip the orphan wipe-tower G-code block
   that follows the removed `T<n>`.

2. **Pass 4 — Curvature-aware adaptive layer height**
   *New.* Direct implementation of Al-Juboori §3.3–3.4.
   Per the ablation study, this is the single largest source of material
   savings (16 % of the 25 % global).

3. **Pass 5 — Physical correctness verification**
   *New.* Enforces M83 (relative E), bounds local E modifications to
   ±30 %, verifies mass conservation against the input file, checks
   instantaneous volumetric flow against the printer's max (default
   12 mm³/s for PLA, configurable per material), preserves every retract
   verbatim. Mandatory before any other pass can rewrite E values.

### Tier B — moderate ROI, needs U1 fixtures (design now, implement after Tier A validates on real prints)

4. **Pass 2 — Shrink purge volume** (design ready in `PASS_2_SHRINK_PURGE.md`)
   Specific to multi-material; benefits Snapmaker U1 + FullSpectrum.

5. **Pass 3 — Merge travel around swaps** (design ready in `PASS_3_MERGE_TRAVEL.md`)
   Disabled by default until heuristics are tuned on real U1 G-code.

6. **Pass 6 — Feedrate smoothing in high-curvature zones**
   *New.* Reduces F based on local curvature with a lower bound (default
   15 mm/s). Improves surface quality (-27..31 % Ra) at cost of +6..12 %
   print time.

### Tier C — research / experimental

7. **Pass 7 — AI weak-zone detection (k-means)**
   *New.* Port the paper's clustering pipeline. Feature vector =
   [curvature, ∂E/∂L, zone-membership]. k=5 by elbow + silhouette.
   Off by default; for users running functional / load-bearing parts.

## FullSpectrum optimisations

FullSpectrum's mixed-color algorithm already alternates layers between
two physical filaments at a fixed cadence
(`mixed_color_layer_height_a`/`_b`). Two extensions are proposed:

### F1 — Curvature-coupled cadence modulation

In high-curvature regions, *finer* cadence (smaller A/B heights) reduces
visible banding at color transitions — the layer step is closer to the
human visual acuity limit, so the additive mix reads cleaner.

In low-curvature regions, *coarser* cadence saves material (Pass 4
already does this for the Z height; F1 makes sure FullSpectrum follows
the same curve).

Implementation: compute a per-segment curvature once (shared with Pass 4),
then scale `mixed_color_layer_height_a/b` locally before invoking the
existing `MixedFilament::resolve_layer()` logic.

### F2 — Sub-pixel bias dithering

The current FullSpectrum bias is a static offset that recesses one of
the two pair components. Replace the static offset with an error-diffusion
dither (Floyd–Steinberg or blue-noise) along the Z axis. The per-layer
choice of which component is recessed depends on the accumulated error,
which over many layers converges to the exact target color.

This is a one-dimensional analogue of image dithering and has the same
property: the apparent color is more accurate at the cost of slightly
noisier per-layer composition. No print-time penalty, no extra G-code.

## bl2u1 native integration — no 4-color limit

bl2u1's `app.py` enforces `TARGET_FILAMENTS = 4` because that is the U1's
physical extruder count. Our fork has FullSpectrum *virtual* filaments,
so the effective filament count is unbounded: N physical + M virtual
mixes.

The native conversion module will:

1. Read the Bambu .3mf metadata (`slice_info.config`,
   `model_settings.config`, `project_settings.config`).
2. Extract the original filament list (id, RGB, type).
3. **If N ≤ 4** — straight remap, same as bl2u1 but inside the slicer's
   import pipeline. No web upload needed.
4. **If N > 4** — pick the 4 most-used physical filaments by mass, and
   synthesise mixed-color virtual filaments for the remaining ones.
   - For each extra filament `f_x` with color `C_x`, find the pair
     `(f_a, f_b)` of physical filaments whose mixed color minimises
     ΔE in CIELAB space.
   - Compute the optimal `mixed_color_layer_height_a/b` ratio that
     reproduces `C_x` given `f_a` and `f_b`.
   - Emit a FullSpectrum virtual filament definition with that ratio.
5. Remap the per-object filament-id metadata so the original assignments
   point to either a physical or a virtual entry.

License: bl2u1 is GPL-3.0, our fork is AGPL-3.0 (OrcaSlicer's license).
Compatible — we preserve attribution in source and CHANGELOG.

## Snapmaker_Orca reliability triage

Without running the slicer, we can target known stability classes:

| Class                            | How to triage                                                                  |
|----------------------------------|--------------------------------------------------------------------------------|
| `wxWidgets` UI thread races      | Search for `wxQueueEvent` calls without main-thread guard; add `wxASSERT(wxIsMainThread())` |
| Toolchange G-code mistakes for U1| Grep `start_print_gcode`, `change_filament_gcode` in U1 printer profile JSON   |
| 3MF round-trip data loss         | Test save → reload of a FullSpectrum project; diff metadata                    |
| Memory leaks on cancel-slicing   | Hook `BackgroundSlicingProcess::stop_internal()`; verify destructors           |
| Mixed-filament UI desync         | Audit `MixedFilament` signal handlers                                          |

This is mostly an issue-tracker triage task, not a single code change.
Best done by: (a) opening the fork's Issues, (b) running the slicer with
sanitisers (ASan/UBSan), (c) collecting U1-specific user reports. We will
file these as follow-up issues, not blockers for the economy work.

## Implementation order

```
v0.1.0  →  Already pushed: skeleton, Pass 1 naïve, settings, hook
v0.2.0  →  This roadmap: Pass 4 (curvature LH), Pass 5 (M83 + verify),
           extended Pass 1 (wipe-tower cleanup)
v0.3.0  →  F1 (cadence modulation), F2 (bias dither), Pass 6 (F smoothing)
v0.4.0  →  bl2u1 native import path (no 4-color cap)
v0.5.0  →  Pass 2 (purge shrink) once U1 fixtures are available
v0.6.0  →  Pass 3 (travel merge), Pass 7 (AI weak-zone) — opt-in
v1.0.0  →  Hardening, mechanical validation on U1 prints, UI polish
```

Each tag triggers a CI release (workflow auto-publishes from main). Tag
v0.2.0 once Pass 4 + Pass 5 land and tests pass.
