# OrcaSlicer fork feature survey

Snapshot taken 2026-05-26 via GitHub search + repo-level inspection.
This document drives the LeanSpectrum roadmap: it identifies which
features pioneered by other forks are worth absorbing, and at what cost.

## Methodology

- GitHub search for "OrcaSlicer fork", inspection of the
  [SoftFever/OrcaSlicer network](https://github.com/SoftFever/OrcaSlicer/network/members)
  and direct README review on each candidate fork.
- Filter: only forks with substantial unique code changes (excludes
  personal mirrors of upstream).
- Score: user value (broad multi-color FFF audience), porting cost
  (lines + concept depth), and conflict with our existing architecture
  (FullSpectrum mixed-color, U1 4-extruder, filament economy passes).

## Surveyed forks

| Fork | Stars | Last activity | Diff focus |
|---|---:|---|---|
| [FULU-Foundation/OrcaSlicer-bambulab](https://github.com/FULU-Foundation/OrcaSlicer-bambulab) | 6.6 k | v1.0.0 May 2026 | Restores Bambu cloud connectivity via sandboxed plugin bridge |
| [jarczakpawel/OrcaSlicer-bambulab](https://github.com/jarczakpawel/OrcaSlicer-bambulab) | 605 | 2026 | Lighter alternative Bambu cloud restore |
| [ratdoux/OrcaSlicer-FullSpectrum](https://github.com/ratdoux/OrcaSlicer-FullSpectrum) | 661 | ongoing (our upstream) | Virtual mixed-color filaments + bias + dithering for U1 |
| [Snapmaker/OrcaSlicer](https://github.com/Snapmaker/OrcaSlicer) | 169 | v2.3.1 Apr 2026 | Official Snapmaker build + U1 / J1 / Artisan profiles + LAN upload |
| [dennisklappe/OrcaSlicer-WaveOverhangs](https://github.com/dennisklappe/OrcaSlicer-WaveOverhangs) | 236 | active 2026 | Support-free overhangs up to 90 deg via wavefront propagation; ~20 expert tunables; port of stmcculloch/PrusaSlicer-WaveOverhangs |
| [Helio-Additive/OrcaSlicer](https://github.com/Helio-Additive) | ~40 (org-wide) | May 2026 | Voxel-based thermal simulation + per-layer speed auto-tuning ("Dragon") |
| [fr3ak2402/GalaxySlicer](https://github.com/fr3ak2402/GalaxySlicer) | active | 2026 releases | Marlin2 + IDEX copy/mirror modes |
| [Polymaker3D/OrcaSlicer-snapmaker](https://github.com/Polymaker3D/OrcaSlicer-snapmaker) | low | secondary | Polymaker filament profile bundle on Snapmaker base |
| Personal mirrors | various | varies | Skipped — minor tweaks only |

## Wiki / upstream gap analysis

The [OrcaSlicer wiki](https://github.com/SoftFever/OrcaSlicer/wiki)
documents printer / material / process / prepare / calibration
sections, but no fork — and no upstream page — ships an *intent-driven
auto-profile mechanism* (one click → curated bundle of layer height,
walls, infill, speeds, cooling, ...). The closest equivalent is the
manual Temperature Tower workflow, which is too narrow to count.

This is the design space LeanSpectrum landed an original feature in
([AutoProfile](https://github.com/jaoli1/OrcaSlicer-LeanSpectrum/blob/feature/filament-economy/src/libslic3r/AutoProfile.hpp),
May 2026).

## Ranked adoption candidates

Ordered by `(user value) / (porting cost)`, decreasing.

| # | Feature | Source | User value | Port cost | Status |
|---|---|---|---|---|---|
| 1 | Auto-Profile (intent + material) | LeanSpectrum original | High — closes the "200 settings" UX gap for non-experts | Low | **Shipped** (d3035b22) |
| 2 | Gyroid Wave optimised (buckling-aware) | upstream 2.4.0-alpha | Medium — stronger parts at same infill | Low — rebase | Pending rebase |
| 3 | Wave Overhangs | dennisklappe | High — eliminates support filament (compounds Pass 1/2 economy) | Medium-high — new perimeter generator + ~20 settings | See [PORT_WAVE_OVERHANGS.md](PORT_WAVE_OVERHANGS.md) |
| 4 | Dynamic Infill Purging | community fork | High — eliminates wipe tower on multi-color | High — new infill scheduler + opacity model | See [PORT_DYNAMIC_INFILL_PURGING.md](PORT_DYNAMIC_INFILL_PURGING.md) |
| 5 | Bias Auto-Pick wizard | LeanSpectrum extension of ratdoux | Medium — UX win on FullSpectrum bias tuning | Medium — UI + CIE-Lab solver | Future |
| 6 | IDEX Copy / Mirror | GalaxySlicer | Medium — flagship demo on U1 hardware | Medium | **Conflicts** with FullSpectrum mixing — must be a UI-level radio choice |
| 7 | Helio thermal simulation | Helio-Additive | Low to medium — narrow audience | High | Defer until user demand surfaces |
| 8 | Bambu cloud restore | FULU-Foundation | Not relevant — U1 is the target, not Bambu hardware | Medium | Skip |

## Compound narrative

The three top-ranked novelties (Auto-Profile + Wave Overhangs +
Dynamic Infill Purging) align under a single LeanSpectrum tagline:

> Less expertise required, less filament wasted.

- Auto-Profile  → less expertise (one click → sensible settings)
- Wave Overhangs → less support filament
- DIP            → less transition filament (replaces the wipe tower)

Together they compound the existing 5-pass G-code economy module. Each
also keeps its master toggle off-by-default so they can be evaluated
independently.

## Snapmaker U1 reference values (2026-05-26 wiki snapshot)

Sourced from:
- [Snapmaker U1 specs page](https://www.snapmaker.com/snapmaker-u1/specs)
- [Snapmaker wiki — U1 multi-color guide](https://wiki.snapmaker.com/en/snapmaker_u1/printing_guides/multi-color_printing_guide)
- [Snapmaker wiki — filament library](https://wiki.snapmaker.com/en/general/manual/filament_library)
- [Snapmaker forum — prime tower minimisation](https://forum.snapmaker.com/t/u1-how-to-minimize-prime-tower-in-orca/40721)
- [JNP-1/Snapmaker-U1-Config](https://github.com/JNP-1/Snapmaker-U1-Config) — community Klipper cfg with real motion numbers

| Parameter | Value | Source / notes |
|---|---|---|
| Max volumetric flow (PLA, 0.4 mm) | **32 mm³/s** ceiling | Hardware spec; SnapSpeed PLA practical ceiling |
| Max travel speed | 500 mm/s | Snapmaker spec |
| Max print acceleration | 20 000 mm/s² (toolhead) | Spec; Klipper allows 25 000 |
| Input shaping | MZV ~54 Hz X / ~47.5 Hz Y | Auto-tuned via accelerometer; no slicer-side override |
| PLA temp / bed | 230-250 °C / 60-80 °C | Wiki filament library — high range matches direct-drive stainless hotend |
| PETG temp / bed | 230-240 / 70-80 °C | Cooling fan default OFF |
| ABS / ASA temp / bed | 230-250 / 60-80 °C | Enclosure recommended |
| Retraction (all) | 0.5-3 mm @ 30-70 mm/s | Direct-drive — keep low |
| **Purge volume per color swap** | **40-60 mm³** | Stock Orca over-purges; biggest economy win |
| Multi-tool ramming volume | 5 mm³ PLA | Wiki value |
| Multi-tool ramming flow | 1.2-1.5× max_volumetric_speed | Wiki guidance |
| Prime tower brim | 3-8 mm (10+ for tall, 1.5-2× model brim for PETG/ABS) | Forum + wiki |
| Z-hop type | **Normal, per-extruder** (NOT "Auto") | Mandatory on U1 IDEX per community guide |

These numbers are baked into
[AutoProfile.cpp](https://github.com/jaoli1/OrcaSlicer-LeanSpectrum/blob/feature/filament-economy/src/libslic3r/AutoProfile.cpp)
(intent + polymer refine tables) so the one-click flow lands on values
the U1 hardware was actually validated against.

## PrusaSlicer absorption candidates (2026-05-26)

PrusaSlicer is the upstream of much of OrcaSlicer's algorithm core via
the Slic3r → Bambu Studio → OrcaSlicer chain. Most of PrusaSlicer's
big wins are already merged:

| PrusaSlicer feature | Status in OrcaSlicer | Action |
|---|---|---|
| Arachne (variable-width walls) | Merged | None — rebase only |
| Lightning infill | Merged | None |
| Organic / tree supports v2 | Merged | None |
| Adaptive cubic infill (octree) | Merged | Verify Quality intent enables it |
| Variable layer height | Merged (simpler heuristic) | Could port the Wasserfall cusp-height metric — medium effort, future task |
| Ramping travel optimisation | Partial (smooth travel only) | Port from PrusaSlicer 2.7.2 — low effort |
| **G2/G3 arc-fitting on G-code export** | **NOT in OrcaSlicer mainline** | **Absorb candidate** — PrusaSlicer 2.7.0-alpha issue #4352, ArcWelderLib reference impl. Reduces .gcode size 15-76%; verify u1-klipper `[gcode_arcs]` is enabled |
| Improved seam placement (2.8) | OrcaSlicer has own logic | Cherry-pick test cases only |
| TBB multi-threaded slice pipeline | Merged | None |
| SLA analytical pipeline (2.9.5) | N/A | Skip — SLA only |

**Ranked add-ons specifically from PrusaSlicer**:

1. **G2/G3 arc-fitting** — biggest single PrusaSlicer-specific perf win
   not already in OrcaSlicer. Smaller .gcode + smoother motion on dense
   multi-color prints. Verify Klipper config first.
2. **Ramping travel from 2.7.2** — small but easy port, fewer stringing
   artefacts on color transitions.
3. **Wasserfall variable-layer-height refinement** — medium effort,
   compounds naturally with FullSpectrum (fewer wasted color
   transitions in low-detail Z bands).

## License notes

All inspected forks are AGPL-3.0 (inherited from OrcaSlicer / Bambu
Studio / PrusaSlicer / Slic3r). Re-derivative work in LeanSpectrum must
retain AGPL-3.0 and credit the prior fork authors in commit messages
and in the consolidated CHANGELOG_LEANSPECTRUM.md (at repo root).
