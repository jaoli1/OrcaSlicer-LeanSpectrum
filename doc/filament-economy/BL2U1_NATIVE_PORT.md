# bl2u1 native port — Bambu → U1 with no 4-color limit

> Ports the Bambu Lab → Snapmaker U1 .3mf converter (originally
> [josuanbn/bl2u1](https://github.com/josuanbn/bl2u1), GPL-3.0) into the
> slicer's project-import path, and removes the 4-filament hard cap by
> synthesising FullSpectrum virtual filaments for the overflow.

## What bl2u1 does today

A standalone Flask web app that:

1. Accepts a Bambu Lab `.3mf` project.
2. Parses its filament list from `Metadata/slice_info.config` and
   `Metadata/project_settings.config`.
3. Lets the user select up to 4 filaments to keep.
4. Rewrites the project to match the U1 schema:
   - `printer_model_id` replaced with `"Snapmaker U1"`.
   - Filaments remapped to U1 profile IDs by type.
   - Per-object `extruder` metadata remapped with `id_mapping`.
   - All `filament_*` arrays normalised to length 4 (padded with white
     PLA dummies if fewer; cropped if more).
   - Template merged from a stock `u1_template.3mf` (or
     `u1_template_supports.3mf` if the input had supports).

The whole pipeline is ZIP-in / ZIP-out, with `Metadata/slice_info.config`,
`Metadata/model_settings.config`, and `Metadata/project_settings.config`
rewritten and the rest of the archive copied through.

## Why the 4-color cap should go

The U1 has 4 physical extruders, but **with FullSpectrum** the slicer
exposes additional *virtual* filaments built from mixing pairs of
physicals. A Bambu file with N filaments can therefore be expressed on
the U1 as:

```
N_physical = min(N, 4)
N_virtual  = max(0, N - 4)
```

Where each virtual filament is a `(physical_a, physical_b, ratio_A:ratio_B,
bias)` tuple chosen to approximate the original color.

## Algorithm

```
Inputs:  bambu_filaments = [(id, color_rgb, type), ...]  (any N)
         u1_template      = stock U1 .3mf template
Outputs: u1_filaments_physical = up to 4 entries
         u1_filaments_virtual  = list of FullSpectrum recipes
         id_mapping            = bambu_id -> u1_filament_index

Step 1 — Score by usage
    For each bambu filament, read `used_m` from slice_info.config.
    Sort descending by used_m (rank by how much plastic they use).

Step 2 — Pick the 4 physicals
    u1_filaments_physical = top-4 by used_m, mapped to nearest U1
    profile by `type` and `color`.

Step 3 — Synthesise virtuals for the rest
    For each remaining bambu filament `f_x` (color `C_x`):
        candidates = all C(4,2) = 6 ordered pairs of u1_filaments_physical
        for each pair (A, B):
            for r in {1/4, 1/3, 1/2, 2/3, 3/4}:    # cadence ratios
                C_pair = mix(A.color, B.color, r)
                ΔE = ciede2000(C_pair, C_x)
            best = argmin ΔE over (pair, r)
        emit virtual entry: (best.A, best.B, ratio = best.r, bias = 0)

Step 4 — Rewrite project_settings.config
    filament_colour    = [C_phys[0..3]] + [C_virt[0..]]
    filament_type      = [T_phys[0..3]] + [T_virt[0..]]
    filament_settings_id = mapped per-filament U1 profile IDs

    mixed_filament_pairs = JSON list of virtual recipes, recognised by
    the FullSpectrum MixedFilament loader.

Step 5 — Rewrite per-object extruder assignments
    For each metadata key="extruder" in model_settings.config:
        old = its value (bambu filament id)
        new = id_mapping[old]    # may point to a virtual entry now

Step 6 — Template merge
    if input had supports → merge with u1_template_supports.3mf
    else                 → merge with u1_template.3mf
    Carry over non-filament settings from the template; let
    filament-related keys be filled from steps 3–5.
```

## Color mixing model

The mixing approximation we use for `mix(A, B, r)`:

```
C_pair(r) = (r * C_A) + ((1 - r) * C_B)     in linear RGB
```

Linear RGB is correct for the additive optical mix that
FullSpectrum produces by stacking thin layers. Convert from sRGB to
linear before mixing, average, then convert back. Distance is computed
in CIELAB via CIEDE2000 because it correlates with perceived color
difference much better than RGB distance.

For higher-fidelity mixing we may later upgrade to a Kubelka-Munk
diffuse-reflection model (which captures the fact that the bottom layer
is partially obscured by the top layer), but for v0.4.0 linear RGB +
CIEDE2000 is sufficient and is a strict improvement over the current
"pick the first match" logic.

## Native integration points

1. **New header**: `src/libslic3r/Format/BambuConvert.hpp`
2. **New unit**: `src/libslic3r/Format/BambuConvert.cpp`
3. **Hook**: `src/libslic3r/Format/bbs_3mf.cpp` — when loading a `.3mf`,
   detect a Bambu printer_model_id and route through `BambuConvert::run()`
   before continuing the normal parse.
4. **UI**: a dialog mirroring bl2u1's web flow (select which filaments
   to keep as physical; the rest auto-virtualised). Skippable with an
   "auto" toggle that always uses top-4-by-usage.

## License compatibility

bl2u1 is GPL-3.0. OrcaSlicer is AGPL-3.0. AGPL-3.0 is a superset of
GPL-3.0 — we may freely re-implement bl2u1's logic in our codebase as
long as:

- The original copyright notice is preserved in `BambuConvert.cpp` header.
- Our derivative is licensed under AGPL-3.0 (which it already is).
- The bl2u1 repository link and acknowledgement appear in the file
  banner and in `doc/filament-economy/BL2U1_NATIVE_PORT.md` (this file).

We are not redistributing bl2u1's source; we re-implement its logic
from the open description.

## What does NOT carry over

- The 200 MB upload limit (we operate on local files).
- The session_id cleanup timer (we use a temp file with RAII).
- The Flask routes / web UI (replaced by native dialog).
- The 4-filament hard cap (the whole point of this exercise).

## Tests

- A Bambu .3mf with 2 filaments → loads as 2 physical, 0 virtual.
- A Bambu .3mf with 4 filaments → loads as 4 physical, 0 virtual.
- A Bambu .3mf with 6 filaments → loads as 4 physical (top by usage) +
  2 virtual. Verify the virtual ΔE < 5 from the original colors.
- An empty .3mf → graceful error, no crash.
- A non-Bambu .3mf → bypass converter, normal load path.
