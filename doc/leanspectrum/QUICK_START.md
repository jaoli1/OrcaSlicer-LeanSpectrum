# LeanSpectrum quick start

Five-minute guide to using the LeanSpectrum-specific features once
you have a binary build of the fork.

## 1. Pick an intent in two clicks

You don't have to learn 200 print settings to get a good first
result on the U1. Open the slicer, load any model, then:

  **File → Auto-generate profile…**

Pick one of:

| Intent | Use for |
|---|---|
| Draft / Fast | Prototype, fitment check, disposable parts |
| Standard / Balanced | Everyday prints (the safe default) |
| High quality / Detail | Display models, fine surface texture |
| Strength / Functional | Load-bearing parts, brackets, fixtures |
| Decorative / Display | Smooth surfaces, lightning infill |

The slicer pushes a curated bundle of overrides into your print
preset (layer height, walls, infill density / pattern, speeds, fan
curves, retraction). Material-specific values come from the active
filament's polymer family — PLA prints with fan at 100 %, ABS at
0–30 %, TPU drops speeds 50 %, and so on. The result dialog lists
every setting that changed so you can audit the bundle.

You can re-run the action with a different intent at any time;
overrides are applied on top of the current preset, not stacked.

## 2. Import a Bambu Lab .3mf with any number of colors

The U1 has 4 physical extruders. Bambu prints often use more (8 is
common on miniatures). LeanSpectrum maps the extra colors onto
FullSpectrum virtual filaments — layer-alternated mixes of two
physical filaments — so the print still comes out colored even when
you exceed the hardware cap.

Steps:
1. **File → Open Project** the Bambu .3mf normally.
2. **File → Convert Bambu palette to Snapmaker U1…**.
3. Confirm the dialog (it tells you how many colors you have and
   what the conversion will produce).

The slicer runs three different palette-selection strategies under
the hood (top-N by usage, exhaustive chromatic minimisation,
usage-weighted) and picks whichever one produces the smallest
visible mismatch on the final print. Floyd-Steinberg dithering is
activated automatically so the virtual filaments don't show banding.

The result dialog reports the perceptual deltaE for each strategy
so you can see the trade-off the slicer made.

## 3. Slice — economy passes run automatically

The 5-pass filament economy module runs at G-code export time. No
configuration needed; the master toggle is on by default and each
pass can be turned off individually in the **Multi-material** section
of the print settings.

What it does on each slice:

- **Pass 5 — Verification gate.** Converts M82 absolute extrusion to
  M83 relative, checks volumetric flow against per-material safety
  limits. If anything fails, the rest of the passes are skipped and
  the file is left as the slicer produced it.
- **Pass 1 — Remove no-op tool changes.** FullSpectrum's bias /
  dithering math sometimes resolves consecutive layers to the same
  physical filament. Those tool-change commands plus the wipe-tower
  block around them are commented out.
- **Pass 4 — Curvature-aware E scaling.** Long straight runs deposit
  slightly less plastic than sharp corners. Per-feature caps protect
  walls, bridges, and the first layer.
- **Pass 2 — Shrink wipe-tower purges.** Purge volumes are sized for
  worst-case nozzle cool-down; when the same extruder was active a
  few seconds ago, the purge is shrunk proportionally.
- **Pass 3 — Collapse redundant retracts.** Back-to-back
  retract + un-retract pairs with no XY motion between them are
  removed.
- **Post-pass verifier.** Re-walks the modified G-code and confirms
  retract count, retract volume, and total extrusion all match the
  expected values (input minus deliberate savings). If any drift,
  the file is reverted to the input.

Savings are logged at the end of the slice — look for
`FilamentEconomy: optimised G-code` in the slicer log. Typical
results on a multi-color print: 15–30 % less filament, 5–15 % faster.

## 4. Importing filament profiles from PDFs

Got a Safety Data Sheet or Technical Data Sheet for a new filament
that isn't in OrcaSlicer's profile library?

1. Download the LeanSpectrum SDS / TDS Importer separately — it's a
   companion Tauri app published at
   `https://github.com/jaoli1/OrcaSlicer-LeanSpectrum/releases`
   (tags `sds-importer-v*`).
2. Drag the PDF onto the importer window, or paste the vendor's
   "downloads" page URL to batch-import.
3. The importer writes a JSON profile into your `Snapmaker_Orca/user/`
   folder — restart the slicer and the new filament appears in the
   Filament tab.

Validated against 1600+ real vendor PDFs across 50+ brands.

## 5. (Coming soon) Wave overhangs

Steep cantilevered overhangs without support material — the
algorithm grows curved perimeters outward from the support-overhang
boundary, depositing extra non-load-bearing material *inside* the
part to anchor the next wave.

Phase 1 of the port has landed (algorithm module, 37 config keys,
data plumbing). The PerimeterGenerator hook + GUI tab + 8-group
settings panel land in subsequent sessions. Watch
[doc/leanspectrum/PORT_WAVE_OVERHANGS.md](PORT_WAVE_OVERHANGS.md)
for status.

## Troubleshooting

- **"This project's palette has already been converted to Snapmaker
  U1"** — the project already has a `bambu_convert_recipe` set. Open
  the multi-material settings, clear the recipe field, run the
  conversion again.
- **Auto-profile changed too much / not enough** — pick a different
  intent. Settings stack overrides on top of the current preset, so
  you can iterate.
- **Filament economy disabled itself mid-slice** — Pass 5's
  verification gate found an invariant violation (volumetric flow
  too high, mass conservation drift). The original G-code is
  preserved. Check the slicer log for the specific reason.
- **The U1 hardware ceiling (32 mm³/s max volumetric speed) seems
  ignored** — OrcaSlicer ships the U1 profile with the standard
  ceiling; Auto-Profile caps the per-intent value beneath that
  (28 / 22 / 20 / 18 / 15 mm³/s for the 5 intents on PLA, scaled
  down for slower materials).

## Where things live

- `src/libslic3r/GCode/FilamentEconomy.cpp` — the 5 passes
- `src/libslic3r/Format/BambuConvert.cpp` — the palette mapper
- `src/libslic3r/AutoProfile.cpp` — intent tables
- `src/libslic3r/MixedFilament.cpp` — FullSpectrum runtime + dither
- `src/libslic3r/FullSpectrumDither.cpp` — Floyd-Steinberg + curvature
- `tools/sds-importer/` — companion Tauri app
- `tools/bambu-3mf-probe/probe.py` — Python dry-run of BambuConvert
- `doc/leanspectrum/` — design docs + roadmap
