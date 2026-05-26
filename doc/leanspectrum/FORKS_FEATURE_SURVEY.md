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
([AutoProfile](../../src/libslic3r/AutoProfile.hpp), May 2026).

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

## License notes

All inspected forks are AGPL-3.0 (inherited from OrcaSlicer / Bambu
Studio / PrusaSlicer / Slic3r). Re-derivative work in LeanSpectrum must
retain AGPL-3.0 and credit the prior fork authors in commit messages
and in [doc/THIRD_PARTY.md](../THIRD_PARTY.md) (TBD).
