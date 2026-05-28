# OptimusOrca by Maison Drabiec — v0.1.2

First release under the **OptimusOrca by Maison Drabiec** name. This is a
**display-only rebrand** of the slicer — no behavioural change. Your existing
profiles keep working: the on-disk data directory and registry key stay
`Snapmaker_Orca`, only the shown application name changed.

OptimusOrca remains **open source** (AGPL-3.0) and is an OrcaSlicer fork for the
Snapmaker U1 multi-color FFF printer, integrating:

- **Filament economy** — 5-pass G-code post-processor with a mass-conservation
  rollback gate (−15 to −30 % filament on typical multi-color prints).
- **BambuConvert** — imports any Bambu Lab `.3mf`, maps onto the 4 U1 extruders,
  synthesises FullSpectrum virtual colors.
- **Auto-Profile** — intent-based profile generation tuned to the Snapmaker U1.
- **Wave Overhangs**, adaptive layer heights, G2/G3 arc fitting.

### Changed
- Application display name → "OptimusOrca by Maison Drabiec" (`SLIC3R_APP_NAME`).
- OOSlicer monogram app icons.
- Landing page (slicer.maisondrabiec.fr) rebranded to OptimusOrca.

### Unchanged (compatibility)
- Data dir / registry key: `%APPDATA%\Snapmaker_Orca` (profiles preserved).
- Release artifact filenames keep the `Snapmaker_Orca_*` prefix.

macOS artifacts published from this fork are unsigned and not notarized.
