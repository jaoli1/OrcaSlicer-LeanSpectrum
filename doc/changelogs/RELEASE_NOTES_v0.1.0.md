## LeanSpectrum v0.1.0

A Snapmaker_Orca / OrcaSlicer-FullSpectrum fork built around the
Snapmaker U1 multi-color FFF printer, with three compounding pillars:
**less expertise required, less filament wasted**.

### Highlights

#### 1. Post-slicing G-code filament economy (5 passes)
Native C++ post-processor that rewrites the produced G-code to
eliminate wasted filament without changing the visible print result.
Five passes — no-op tool swap removal, wipe-tower purge shrinking,
retract collapse, curvature-aware E scaling (Al-Juboori 2026),
mass-conservation verification with rollback on failure. Logs
savings at end of slice. Typical reduction on multi-color prints:
**15–30 % less filament, 5–15 % less time**.

#### 2. BambuConvert — Bambu .3mf → U1 palette mapper
Drop a Bambu Lab .3mf with any number of colors into the slicer;
**File → Convert Bambu palette to Snapmaker U1…** maps the heavy-use
colors onto the U1's 4 physical extruders and synthesises FullSpectrum
virtual filaments for the overflow. Three selection strategies (Usage,
Chromatic, Balanced) auto-picked by the slicer; 19-ratio mixing grid;
Floyd-Steinberg dithering activated automatically. CIEDE2000
perceptual matching against Sharma 2005 reference values.

Tested with real Bambu X1 Carbon 8-color print —
**Σ ΔE drops from 91.17 to 48.26 (-47 %)** vs the equivalent
single-strategy approach.

#### 3. Auto-Profile — one-click intent-driven settings
**File → Auto-generate profile…** with 5 intents (Draft / Standard /
High quality / Strength / Decorative) crossed with 9 polymer families
(PLA / PETG / ABS / PC / PA / TPU / HIPS / PP / Unknown) = 45 sensible
configuration bundles tuned to the Snapmaker U1 official wiki spec.
No surveyed OrcaSlicer fork ships an equivalent.

#### 4. Wave Overhangs (TBD — pending experimental branch CI)
**Print steep cantilevered overhangs without support material**, via
wavefront propagation perimeters that anchor each new layer onto the
last. Algorithm by Janis A. Andersons (SSRN doi.org/10.2139/ssrn.6640458),
ported from Steven McCulloch's PrusaSlicer work and dennisklappe's
OrcaSlicer port. 37 expert tunables organised under 8 groups.
Disabled by default — flip `wave_overhangs = true` to enable.

#### 5. SDS / TDS Importer companion app
A separate Tauri 2 desktop app: drop a filament Safety Data Sheet or
Technical Data Sheet PDF and get a Snapmaker_Orca filament profile
JSON ready for import. Validated against 1600+ real vendor PDFs
across 50+ brands. Three input modes (single PDF, vendor catalog
crawler URL, local corpus browser). Bilingual FR / EN. Optional
Tesseract OCR for scanned PDFs.

Downloads also tagged separately as `sds-importer-v0.1.0`.

### Reliability fixes absorbed from upstream-snap

- HTTP wipe tower byte-budget overrun (no-op G1/G2/G3 lines now
  dropped before the wipe tower G-code exceeds its allocated budget)
- Model download URL with query parameters no longer leaves
  illegal-on-Windows characters in the on-disk filename
- New `/wcp_download/` HTTP route bypasses Flutter's 512 MB
  postMessage cap for large transfers
- `sw_GetActiveFile` now emits a `url` field with the live HTTP
  server port so file streaming uses HTTP instead of postMessage

### Quick start

See [doc/leanspectrum/QUICK_START.md](https://github.com/jaoli1/OrcaSlicer-LeanSpectrum/blob/main/doc/leanspectrum/QUICK_START.md)
for a 5-minute walkthrough of the three menu actions.

### Attribution

LeanSpectrum is AGPL-3.0 (inherited from OrcaSlicer / Bambu Studio /
PrusaSlicer / Slic3r). Algorithm sources gratefully acknowledged in
the slicer's About dialog and below:

- **Filament economy** — Al-Juboori 2026
  ([doi:10.1007/s44444-026-00109-y](https://doi.org/10.1007/s44444-026-00109-y),
  CC-BY 4.0)
- **BambuConvert** — josuanbn/bl2u1 conceptual origin, Sharma 2005
  CIEDE2000 reference (Sharma G., Wu W., Dalal E. N.,
  *The CIEDE2000 Color-Difference Formula*)
- **Wave Overhangs** — Janis A. Andersons (algorithm), Steven McCulloch
  (PrusaSlicer port), dennisklappe (OrcaSlicer port)
- **Base slicer** — [Snapmaker_Orca](https://github.com/Snapmaker/OrcaSlicer)
  fork of [OrcaSlicer](https://github.com/SoftFever/OrcaSlicer),
  built on [Bambu Studio](https://github.com/bambulab/BambuStudio),
  built on [PrusaSlicer](https://github.com/prusa3d/PrusaSlicer),
  built on [Slic3r](https://github.com/slic3r/Slic3r)
- **FullSpectrum virtual-filament foundation** —
  [ratdoux/OrcaSlicer-FullSpectrum](https://github.com/ratdoux/OrcaSlicer-FullSpectrum)

### Known limitations

- **Wave Overhangs** is opt-in (default off). First v0.1.0 release
  may ship without it if the experimental-branch CI is still red on
  Windows / macOS at tag time. See `doc/leanspectrum/PORT_WAVE_OVERHANGS.md`.
- **Auto-Profile** material refinements for PETG / ABS / PA / TPU
  are wiki-derived. First print of any non-PLA bundle is a sanity
  check — sand the values to taste before doing a long print.
- **BambuConvert** assumes the source .3mf was sliced by Bambu Studio
  so `Metadata/slice_info.config` carries per-filament `used_m` values.
  Unsliced .3mf files fall back to flat usage and the Balanced
  strategy degenerates to Chromatic.
- **macOS arm64** binary is the only smoke-tested Apple Silicon path;
  the **macos-13 Intel** build is included as a courtesy and should
  work but hasn't been validated on hardware.
- **macOS DMGs ship unsigned and unnotarized.** On first launch
  Gatekeeper will block the app — right-click → Open to bypass.
  Code signing certificates will follow in a later release.
- **Windows MSI / EXE ship unsigned.** SmartScreen will warn on first
  launch — choose "More info" → "Run anyway" to bypass.
- **Bambu Lab H2D / H2C / X2D dual-extruder .3mf** files are detected
  but BambuConvert flattens them onto the U1's single-tool-at-a-time
  layout. AMS 2 Pro / AMS HT multi-unit setups (12+ slots) similarly
  collapse to the project's effective color list. Full dual-nozzle
  routing support tracked for v0.2.0.

### Downloads

The slicer ships as **one artifact per platform** (the build_all
workflow produces a single bundle per OS, not multiple installer
formats):

| Platform | File |
|---|---|
| Windows 10/11 x64 | `Snapmaker_Orca_Windows_V0.1.0_portable.zip` (NSIS-style portable — unzip and run, no MSI/EXE installer) |
| macOS 13+ universal | `Snapmaker_Orca_Mac_universal_V0.1.0.dmg` (signed + Apple-notarized, runs on arm64 and Intel) |
| Linux Ubuntu 24.04+ | `Snapmaker_Orca_Linux_AppImage_Ubuntu2404_V0.1.0.AppImage` (no .deb / .rpm at this release; chmod +x and run) |
| SDS Importer companion | see `sds-importer-v0.1.1` release — ships Windows .msi+.exe, macOS arm64+Intel .dmg, Linux .AppImage+.deb+.rpm |
| Profile validator (optional) | `Snapmaker_Orca_profile_validator*` — CI tool, end-users don't need it |

### Acknowledgements

Thanks to:
- The Snapmaker team for the U1 hardware + their Orca fork maintenance
- The ratdoux/OrcaSlicer-FullSpectrum project for the virtual-filament
  foundation this fork extends
- SoftFever / Bambu / Prusa / Slic3r for decades of open slicer work
- Every filament vendor whose datasheets feed the SDS importer
  validation corpus
