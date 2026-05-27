# LeanSpectrum release readiness checklist

For tagging `leanspectrum-v0.1.0` — the first releasable build of
the fork. Targets: an OrcaSlicer-derived slicer binary for each of
Linux / macOS / Windows that includes all three LeanSpectrum pillars
(5-pass filament economy + BambuConvert + Auto-Profile) plus the
SDS Importer companion app.

## Must-have before tag

### Build

- [ ] `build_all` green on Ubuntu / macOS / Windows for the head of
      `feature/filament-economy` (after `experiment/wave-overhangs-phase3b`
      merge if Phase 3b lands; without it if Phase 3b is deferred).
      Current state: in_progress on
      [run 26478882947](https://github.com/jaoli1/OrcaSlicer-LeanSpectrum/actions/runs/26478882947).
- [ ] No new compiler warnings introduced by LeanSpectrum-specific
      files. Run with `-Wall -Wextra` and diff the count.
- [ ] All Catch2 tests pass (`ctest` from `build/`). Coverage:
      - `tests/fff_print/test_filament_economy.cpp`
      - `tests/libslic3r/test_bambu_convert.cpp`
      - `tests/libslic3r/test_fullspectrum_dither.cpp`
      - `tests/libslic3r/test_auto_profile.cpp`
      - `tests/libslic3r/test_mixed_filament.cpp`

### Smoke tests on real hardware

- [ ] **Single-material PLA** on the U1 — verify FilamentEconomy
      stats log shows `swaps_removed=0` and the print completes.
      Confirms the post-processor doesn't break single-material.
- [ ] **2-color FullSpectrum** print — verify Pass 1 trims the
      cadence-driven no-op swaps. Print should look identical to a
      pre-LeanSpectrum FullSpectrum print but use 10-25 % less
      filament (look at the wipe-tower tab in the slicer preview).
- [ ] **8-color Bambu .3mf conversion** — use the provided
      HarryPotter +Color Painted fixture. Verify:
      - File menu has "Convert Bambu palette to Snapmaker U1..."
      - Conversion dialog reports 4 physicals + 4 virtuals
      - Slicer's filament list shrinks to 4 after the conversion
      - G-code preview shows FullSpectrum cadence on the 4 virtuals
- [ ] **Auto-Profile on a known model** — pick "High quality / Detail"
      on a Benchy. Verify layer height drops to 0.12 mm, walls go to
      3, scarf seam engages.

### Documentation

- [ ] README.md three-pillar pitch (done — see commit e564a7a7f)
- [ ] CHANGELOG_LEANSPECTRUM.md (done — see commit 80093b1f7)
- [ ] QUICK_START.md user-facing guide (done — see commit 8343905607)
- [ ] PR #1 description matches actual scope (done as of
      autonomous extension session)
- [ ] LICENSE notes call out the AGPL-3.0 chain through Snapmaker_Orca
      / OrcaSlicer-FullSpectrum / OrcaSlicer / Bambu Studio /
      PrusaSlicer / Slic3r. Plus credits for: Al-Juboori 2026
      (filament economy), Janis A. Andersons + Steven McCulloch +
      dennisklappe (Wave Overhangs), josuanbn (bl2u1 conceptual
      origin), Sharma 2005 (CIEDE2000 reference values).
- [ ] AboutDialog.cpp credits panel updated with the attribution
      chain. **NOT DONE** — needs a design pass, not a copy-paste.

### Release artefacts

- [ ] Slicer binaries built for each OS:
      - `OrcaSlicer-LeanSpectrum_linux-amd64.AppImage`
      - `OrcaSlicer-LeanSpectrum_linux-amd64.deb`
      - `OrcaSlicer-LeanSpectrum_linux-amd64.rpm`
      - `OrcaSlicer-LeanSpectrum_macos-arm64.dmg`
      - `OrcaSlicer-LeanSpectrum_macos-intel.dmg`
      - `OrcaSlicer-LeanSpectrum_windows-amd64.msi`
      - `OrcaSlicer-LeanSpectrum_windows-amd64.exe` (NSIS)
- [ ] SDS Importer release `sds-importer-v0.1.0` published (in
      flight as of [run 26471073135](https://github.com/jaoli1/OrcaSlicer-LeanSpectrum/actions/runs/26471073135),
      still pending at last check)
- [ ] GitHub Release with auto-generated notes + manual highlights
      pointing to the three pillars + QUICK_START.md
- [ ] At least one community-tested print (Snapmaker Discord / forum)
      with a "before / after filament use" comparison screenshot

## Nice-to-have (defer to v0.2 if needed)

- [ ] Wave Overhangs Phases 3b / 4 / 5 fully landed and tested
- [ ] Bias Auto-Pick wizard (UI for FullSpectrum bias tuning via
      CIE-Lab grid)
- [ ] 3-physical mixing in BambuConvert (schema change on
      bambu_convert_recipe)
- [ ] Helio thermal simulation (deferred until user demand)
- [ ] IDEX copy / mirror modes (mutually exclusive with FullSpectrum,
      needs a UI radio choice first)

## Known limitations to disclose in the release notes

- **Wave Overhangs** is partially ported (algorithm module + config
  keys + scaffolding landed; not yet wired into PerimeterGenerator).
  The settings show up but `wave_overhangs = true` is currently a
  no-op until Phase 3b lands.
- **Auto-Profile** material refinements are wiki-derived; not yet
  validated against real prints for non-PLA materials (PETG / ABS /
  PA / TPU). First-print sanity check recommended.
- **BambuConvert** assumes the source .3mf was sliced by Bambu
  Studio (so `Metadata/slice_info.config` has per-filament `used_m`
  values). For unsliced .3mf files, the conversion falls back to a
  flat usage of 1 mm — the chromatic strategy still picks something
  sensible but the balanced strategy degenerates to chromatic.
- **macOS arm64 binary** is the only validated path on Apple Silicon;
  Intel macOS builds on `macos-13` should work but haven't been
  smoke-tested.

## How to actually cut the release

```
git checkout main
git merge feature/filament-economy
git tag -a leanspectrum-v0.1.0 -m "First releasable LeanSpectrum build"
git push origin main leanspectrum-v0.1.0
```

The tag will trigger the CI `build_all` workflow's release-artefact
path (if the workflow has tag-based dispatch wired). If not, add a
new `release` job to `build_all.yml` mirroring the
`build_sds_importer.yml` tag handling.
