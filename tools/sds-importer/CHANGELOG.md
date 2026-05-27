# Changelog

All notable changes to the **Custom Filament Profile Creator** (formerly
"LeanSpectrum SDS Importer") are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/); versions follow
[SemVer](https://semver.org/).

> The directory `tools/sds-importer/` is retained for git-history
> continuity but the product, binary, and bundle identifiers are now
> `Custom Filament Profile Creator` (FR: *Créateur de profils filament
> adaptés*). Tag pattern: `profile-creator-v*` (legacy `sds-importer-v*`
> still triggers the workflow for backward compatibility).

## [0.1.2] — rename: Custom Filament Profile Creator

What changed:
- Product name **LeanSpectrum SDS Importer → Custom Filament Profile
  Creator** (FR display: *Créateur de profils filament adaptés*) — the
  new name reflects what the user actually gets: a Snapmaker_Orca
  filament profile *tailored* to the specific vendor filament, not just
  a generic SDS importer.
- Cargo package `leanspectrum-sds-importer → custom-filament-profile-creator`
- Tauri identifier `fork.leanspectrum.sds-importer → fork.leanspectrum.profile-creator`
- Tauri lib name `leanspectrum_sds_importer_lib → custom_filament_profile_creator_lib`
- npm package name + node-side workflow renamed to match
- Window title + bundle product name updated
- CI workflow trigger accepts both `profile-creator-v*` (new canonical)
  and `sds-importer-v*` (legacy)

Functional behaviour: unchanged from 0.1.1. Same SDS / TDS parser, same
output profile schema, same OCR fallback, same vendor catalog crawler.

The 0.1.0 and 0.1.1 release tags never published due to the icon bug
chain (fixed in 0.1.1's `bundle.icon` array). 0.1.2 is the first
publicly downloadable build.

## [0.1.1] — Windows bundle fix

The 0.1.0 release tag's Windows binary failed to bundle because
`tauri.conf.json` only listed `icons/icon.png` in `bundle.icon`,
which Tauri's bundler can't use as the Windows shortcut/EXE icon
(Windows requires `.ico` format). The `icons/icon.ico` file was
already present in the repo but not referenced.

0.1.1 lists all available icon variants (`icon.png`, `icon.ico`,
`32x32.png`, `128x128.png`, `128x128@2x.png`) in `bundle.icon` so
the Tauri bundler picks the right format per platform: `.ico` for
Windows MSI and NSIS bundles, `.png` for Linux `.deb` / `.rpm` /
`.AppImage`, the size variants for `.AppImage` icon-resolution
matching.

No other changes from 0.1.0. The Cargo source code is unchanged.

## [0.1.0] — first downloadable release

### What it does

Drop a filament Safety Data Sheet or Technical Data Sheet PDF; get a
Snapmaker_Orca filament profile written into your user-profile folder.
The next time you open Snapmaker_Orca, the new filament appears in
the Filament list — import it via the regular `Import` menu if you
prefer not to drop it directly in `~/.../Snapmaker_Orca/user/<id>/filament/`.

### Three input modes

- **Single PDF** — drag-and-drop or click to pick a PDF, optional
  online TDS lookup if the SDS only carries chemistry data.
- **Vendor catalog** — paste a "certificates" / "downloads" page URL
  (e.g. eryone3d.com/pages/filament-files,
  sunlu.com/products/.../downloads-section, atome3d.com/pages/...).
  The app fetches the page, lists every SDS / TDS PDF it can identify
  with `SDS` / `TDS` / `Unknown` badges and a polymer-family chip,
  and lets you batch-import any subset in one click.
- **Local database** — browse PDFs already on disk (defaults to
  `~/Downloads/filament-corpus/`, configurable). Files are grouped
  by brand sub-folder; click any to import.

### Parser coverage

Validated against 1600+ real vendor PDFs across 50+ brands collected
from the catalogue and direct manufacturer downloads. Specific
regression tests in `tds.rs` and `sds.rs` cover:

- Eryone (TDS, tabular layout with whitespace padding)
- eSun (SDS + TDS, French and English variants)
- ROSA3D (MSDS, Polish-style supplier line, reverse-column TDS)
- SUNLU (SDS with TDS data embedded in section 9, official TDS with
  values above labels)
- 10 polymer families detected by CAS number + name patterns:
  PLA, PETG, ABS, ASA, PC, TPU, PA6, PA12, HIPS, PP.

### Generated profile contents

The output JSON inherits from the closest Snapmaker U1-tuned
profile (`Snapmaker PLA SnapSpeed @U1`, `Generic ABS @U1`, …) so
you keep the parent profile's cooling / retract / pressure-advance
settings, and overrides only what the importer extracted or
backfilled:

- Nozzle temperature, range low / high, initial-layer temperature
- Bed temperature (start + initial layer)
- Filament density, vendor, filament type
- **Maximum volumetric speed** — extracted when present, otherwise
  filled from a conservative per-polymer default (PLA 12, PETG 9,
  ABS 9, PC 7, PA 8, TPU 4 mm³/s on a 0.4 mm nozzle)
- **Scarf-joint seam settings** tuned per polymer family
  (`seam_slope_min_length`, `seam_slope_steps`,
  `scarf_angle_threshold`, `scarf_joint_speed`,
  `scarf_joint_flow_ratio`, `seam_position`) — produces nearly
  invisible Z-seams on PLA / PETG / ABS / ASA / PC / PA / PP / HIPS
  with sensible defaults; disabled on TPU (rubber doesn't ramp
  cleanly).

Backfilled fields are tracked in `_leanspectrum_metadata.estimated_fields`
so the UI badges them as `Needs review` when the user should
sanity-check before printing critical parts.

### Bilingual UI

English and French built-in, switchable from the top bar; the
selection persists in `localStorage`. First launch follows the OS
locale.

### Optional OCR

Scanned (image-only) PDFs need OCR. To keep the default binary
slim and the CI cross-platform, OCR is gated behind the `ocr`
Cargo feature and is **not** in the prebuilt binaries. To enable
it, install system Tesseract and Leptonica then rebuild:

```bash
cargo tauri build --features ocr
```

### Platforms

| OS      | Artifact                              |
|---------|---------------------------------------|
| Linux   | `.AppImage` + `.deb` + `.rpm`         |
| macOS   | `.dmg` (separate arm64 and Intel)     |
| Windows | `.msi` (recommended) + `.exe` (NSIS)  |

### Known limitations

- Some vendor TDS layouts that pdftotext extracts column-first (eSun
  ePLA-HF in particular) are only partially parsed — nozzle temperature
  is picked up via a polymer-aware backward scan, but bed temperature
  and print speed in those layouts are lost. Workaround: drop the SDS
  for the same product instead, then the importer fills bed temperature
  from the polymer default.
- The local-database browser only walks one nesting level deep
  (`<corpus>/<brand>/*.pdf` and `<corpus>/<brand>/<product>/*.pdf`).
  Deeper trees are ignored.
- Process profile recommendation (which printer / nozzle preset to pair
  the filament with) returns `None` for now — to land in v0.2.

### License

AGPL-3.0-or-later. Built on:
- [Tauri 2](https://tauri.app/) — Apache-2.0 / MIT
- [pdf-extract](https://crates.io/crates/pdf-extract) — MIT / Apache-2.0
- [pdfium-render](https://crates.io/crates/pdfium-render) — Apache-2.0 / MIT
- [scraper](https://crates.io/crates/scraper) — ISC
- [tesseract-rs](https://crates.io/crates/tesseract) (optional) — Apache-2.0

ISO 11014-1 / GHS standards are public references; no proprietary
schema or profile content is reproduced.
