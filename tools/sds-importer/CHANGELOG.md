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

## [0.1.8] — fix: window closed silently on PDF import (UTF-8 panic)

The 0.1.7 release installed and launched correctly, but clicking
"Créer le profil filament" after dropping a PDF closed the application
window immediately. No error dialog, no log entry. Tested PDF:
ERYONE PLA+ TDS, which contains 9× `℃` (U+2103, 3 bytes UTF-8) plus
`°` (U+00B0, 2 bytes), fullwidth `）` (U+FF09), fullwidth `，` (U+FF0C),
and en-dashes.

**Root cause** (three senior-agent audits converged): the TDS / SDS
parsers slice the extracted text by **byte** index — `&text[idx..idx+200]`
to look at a 200-byte window after a label, `&text[..text.len().min(1500)]`
for the header, etc. When a bound lands inside a multi-byte UTF-8
sequence, Rust panics `"byte index N is not a char boundary"`. The
panic propagates out of the Tauri command worker thread, kills the
WebView2 host, and the window vanishes — exactly the symptom the user
saw, with no surviving stderr trace because the host died too fast for
env_logger to flush.

Fixes:

- **`text_utils.rs` (new module)**: `safe_slice(s, start, end)` clamps
  both bounds to the string length and snaps `start` DOWN / `end` UP
  to the nearest valid char boundary. Never panics. Comprehensive
  tests cover empty input, pure ASCII, U+2103 ℃, U+00B0 °, and the
  exact "label window after a degree sign" pattern from `tds.rs`.
- **`normalize_unicode(text)` (same module)**: folds `℃ → °C`, `℉ → °F`,
  en-dash / em-dash → ASCII hyphen, fullwidth parens / comma / colon /
  semicolon → ASCII, non-breaking space → space. Runs once on the
  extracted text before either parser sees it, so the regexes no
  longer have to carry per-pattern Unicode classes and the slice
  surface area shrinks.
- **`tds.rs`**: all 10 byte-slice sites refactored to `safe_slice(...)`
  (degree-sign tail check, label-forward / label-backward windows,
  header window for manufacturer + product, glass-transition window,
  print-speed window).
- **`sds.rs`**: all 5 byte-slice sites refactored to `safe_slice(...)`
  (section split, density window forward + backward, two Section-9
  range snippets).
- **`lib.rs`** — `run_command(...)` wrapper: every Tauri command
  (`import_pdf`, `import_from_urls`, `crawl_catalog`, `scan_corpus`)
  runs its body inside `std::panic::catch_unwind`. Any future panic
  surfaces as a structured `Error::Other("internal panic: …")`
  returned to the JS frontend instead of taking the worker thread
  down. The single safety net beneath the UTF-8 fix.
- **`lib.rs`** — persistent log file at
  `%LOCALAPPDATA%\Custom Filament Profile Creator\app.log` (Windows),
  `~/Library/Application Support/Custom Filament Profile Creator/app.log`
  (macOS), `~/.local/share/Custom Filament Profile Creator/app.log`
  (Linux). `env_logger` targets this file when running as a packaged
  app; a custom panic hook also appends panic info + location, so
  the next time something goes wrong the user has a file to send
  back. The release build no longer relies on stderr that nobody
  reads.

If you installed 0.1.7 and saw the window-closes-on-click symptom,
0.1.8 fixes it in place — the bundle identifier is unchanged
(`fork.leanspectrum.profile-creator`) so the new MSI / `.deb` / `.rpm`
upgrades over 0.1.7.

## [0.1.4–0.1.7] — IPC + serde plumbing (no functional changes)

Four rapid-fire bugfix releases to make the v0.1.2 build actually
usable end-to-end:

- **0.1.4** — visible boot-error trap so a JS syntax error in the
  loader script no longer paints a blank app window. Surfaced the
  v0.1.2-era scope collision below.
- **0.1.5** — fix `const t` from `main.js` colliding with `function t`
  from `i18n.js` at global scope (`Identifier 't' has already been
  declared`). Renamed the i18n helper to `tr` to free up the symbol.
  The drop zone, the three tabs and the language picker all start
  responding again.
- **0.1.6** — `invoke('import_pdf', { pdf_path })` reverted to the
  snake-case argument name expected by the Rust struct literal,
  removing the `missing field pdf_path` error.
- **0.1.7** — belt-and-suspenders: added
  `#[serde(rename_all = "camelCase")]` to `ImportRequest` /
  `BatchImportRequest` so both `pdf_path` and `pdfPath` deserialize
  correctly. The JS side is back to camelCase to keep the wire
  format consistent with Tauri's documented convention.

These four iterations shared a single underlying cause (a single
unaccounted-for runtime error blanked the entire frontend) and are
collapsed here for readability. The 0.1.8 entry above is where the
*data* path was made correct.

## [0.1.3] — CSP fix: drop zone + tabs were silently broken in 0.1.2

The 0.1.2 release installed correctly on Windows (MSI valid, app
launched, window painted) but neither the file drop zone nor the
three navigation tabs responded to user input. Root cause: the
Content-Security-Policy `script-src 'self'` blocked Tauri 2's
runtime injection of the `window.__TAURI__` global. The very first
line of `frontend/src/main.js` then threw
`TypeError: Cannot read properties of undefined (reading 'core')`,
which halted the entire script before any event listener
(tabs, drop zone, language picker, run button) could attach.

Fixes:
- **tauri.conf.json `security.csp`** rewritten to the Tauri 2
  recommended baseline:
  `default-src 'self' ipc: http://ipc.localhost; img-src 'self' data: asset: http://asset.localhost; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline'; connect-src 'self' ipc: http://ipc.localhost`
  — adds `'unsafe-inline'` to `script-src`, opens `ipc:` +
  `http://ipc.localhost` for the IPC bridge, opens
  `asset: http://asset.localhost` for in-app asset URLs.
- **frontend/src/main.js** now resolves the invoke handle defensively
  via `resolveInvoke()` that probes both `window.__TAURI__.core.invoke`
  (Tauri 2 stable) and `window.__TAURI__.invoke` (legacy fallback),
  and logs a console error with diagnostic info if neither is found.
  The tabs and i18n picker work even when invoke is null — useful
  for triaging future runtime issues.
- **windows[0]**: `devtools: true` (so users can open F12 / right-click
  Inspect to debug) and `dragDropEnabled: true` (explicit, since Tauri
  2 stable changed the default).

If you installed v0.1.2 and saw the dead drop zone, uninstall it (the
app identifier changed across the rename so the v0.1.3 MSI installs
side-by-side, but the v0.1.2 entry in Add/Remove Programs is the same
ID `fork.leanspectrum.profile-creator` so the new MSI upgrades it
in-place).

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
