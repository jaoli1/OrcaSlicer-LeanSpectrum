# Pass 4 — Curvature-aware adaptive layer height

> Adapts effective extrusion per-segment based on local XY curvature.
> Direct port of Al-Juboori 2026 §3.3–3.4 to the LeanSpectrum
> module, with constraints adapted to FullSpectrum mixed-color prints
> on the Snapmaker U1.

## Theory

Local geometric curvature κ at toolpath point `P_i` is approximated by
the angle between the two segments meeting at `P_i`:

```
        v1 = P_i  - P_{i-1}
        v2 = P_{i+1} - P_i
        κ_i = acos( clamp((v1 · v2) / (|v1| * |v2|), -1, 1) )
```

`κ_i = 0` means a straight segment; large `κ_i` means a sharp corner.

The paper's central observation: low-curvature regions tolerate thicker
layers (less effective extrusion per Z-unit), high-curvature regions
need thin layers for surface fidelity.

In a post-slicing pass we cannot move Z — that would desynchronise from
the slicer's planned Z layers and break the wipe tower, the layer-change
G-code, and the FullSpectrum cadence. Instead we adjust **effective
extrusion** per segment: scaling `E` by `r_i` where `r_i ∈ [r_min, 1]`,
with `r_min = 1 - max_reduction_pct/100`.

This is mathematically equivalent to printing with a thinner layer for
that segment (less plastic deposited per unit length) without moving Z.

## Mapping κ to reduction ratio

```
κ_high = 45°  (default for PLA; 30° for Nylon; configurable)
κ_low  = 10°  (configurable)

if κ_i >= κ_high → r_i = 1.0           (no reduction, full extrusion)
if κ_i <= κ_low  → r_i = r_min          (maximum reduction)
else             → r_i linearly interpolated
```

A moving median filter of window 7 is applied to the `{r_i}` sequence
before rewriting, to avoid abrupt extrusion changes that would show up
as banding.

## Per-region overrides

Following the paper, the reduction is gated by segment type:

| Segment type (detected via OrcaSlicer's `; FEATURE:` comments) | Max reduction |
|----------------------------------------------------------------|---------------|
| Outer wall                                                     | ±15 %         |
| Inner wall                                                     | ±20 %         |
| Solid infill / top / bottom                                    | ±25 %         |
| Sparse infill                                                  | ±35 %         |
| Bridge                                                         | **0 %** (no reduction) |
| Wipe tower / prime tower                                       | 0 % (handled by Pass 2) |
| Support                                                        | ±30 %         |
| Custom / user G-code                                           | 0 %           |

These caps protect interlayer adhesion on walls and prevent sagging on
bridges. The numbers are derived from the paper's Nylon results, tuned
down (more conservative) for PLA, which is what the U1 primarily uses.

## FullSpectrum interaction

FullSpectrum mixes layers between two physical filaments. Pass 4 operates
*within* a layer (per-segment E scaling) and never crosses layer
boundaries. The cadence — which physical filament is active for layer N
— is unchanged. The amount of plastic deposited *within* each FullSpectrum
half-layer is scaled by Pass 4.

This composes cleanly with F1 (curvature-coupled cadence modulation)
because:

- Pass 4 changes `E_segment`
- F1 changes `mixed_color_layer_height_a/b` per layer

They are orthogonal axes; you can enable either, both, or neither.

## Algorithm

```
1. Modal-state pass: walk the file once, building a list of segments
   with (X, Y, Z, F, E, feature_type, is_retract, ...).

2. Curvature pass: for each segment, compute κ from the three
   consecutive non-retract XY positions.

3. Ratio pass: map κ → r_segment using the table above and the
   feature-type cap.

4. Filter: apply a length-7 moving median to the r_segment sequence.

5. Rewrite pass: walk the file a second time. For each G1 line with
   positive E (deposition), multiply E by the segment's r. Retracts,
   unretracts, travels, and explicit non-extrusion lines are not
   touched.

6. Verification: compute Σ E_new vs. Σ E_old. The reduction must equal
   1 - mean(r_segment), modulo per-feature caps. Reject the rewrite if
   the deviation exceeds 1 % (indicates a parser bug).
```

## Pseudo-code

```cpp
struct Segment {
    Vec2d   p_start, p_end;
    double  z;
    double  feedrate;
    double  e;            // positive = extrusion, 0 = travel, < 0 = retract
    FeatureType feature;
    bool    is_retract;
};

double curvature(const Segment &prev, const Segment &cur)
{
    Vec2d v1 = (prev.p_end - prev.p_start).normalized();
    Vec2d v2 = (cur.p_end  - cur.p_start ).normalized();
    return std::acos(std::clamp(v1.dot(v2), -1.0, 1.0));  // radians
}

double feature_cap(FeatureType f)
{
    switch (f) {
        case FeatureType::OuterWall: return 0.15;
        case FeatureType::InnerWall: return 0.20;
        case FeatureType::SolidInfill: return 0.25;
        case FeatureType::SparseInfill: return 0.35;
        case FeatureType::Bridge:    return 0.0;
        case FeatureType::WipeTower: return 0.0;
        case FeatureType::Support:   return 0.30;
        default:                     return 0.0;
    }
}

double ratio_from_curvature(double kappa_rad,
                            double kappa_low_rad,
                            double kappa_high_rad,
                            double max_reduction)  // 0..1
{
    if (kappa_rad >= kappa_high_rad) return 1.0;
    if (kappa_rad <= kappa_low_rad)  return 1.0 - max_reduction;
    double t = (kappa_rad - kappa_low_rad) / (kappa_high_rad - kappa_low_rad);
    return (1.0 - max_reduction) + t * max_reduction;
}
```

## Settings

To add to `PrintConfig`:

| Key                                            | Type  | Default          |
|------------------------------------------------|-------|------------------|
| `filament_economy_curvature_lh`                | bool  | true             |
| `filament_economy_curvature_low_deg`           | float | 10.0             |
| `filament_economy_curvature_high_deg`          | float | 45.0             |
| `filament_economy_curvature_max_reduction_pct` | int   | 30 (PLA) / 25 (Nylon by profile override) |
| `filament_economy_curvature_filter_window`     | int   | 7                |

## Risks

- **Pressure-advance interaction** — the paper notes that pressure-advance
  algorithms anticipate based on E rate. Scaling E changes the apparent
  rate. The paper concluded this is acceptable at ±30 % bounds.
  We propagate the same bound.
- **Wipe tower coupling** — wipe-tower segments must be excluded
  (`feature_cap = 0`). The detection relies on `;TYPE:Wipe tower`
  comments emitted by Snapmaker_Orca. Verify these markers are present.
- **U1 toolchanger ramps** — the U1 has unique start/end-of-swap
  sequences. The custom G-code blocks (`; CP TOOLCHANGE START`) must
  also have `feature_cap = 0`.
- **First layer** — should be excluded entirely (no reduction) for
  adhesion. Detected via the `; LAYER:0` comment or a Z < initial_layer_height + ε
  check.

## Tests

- Single straight line → `r = 1.0` everywhere, no change.
- 90° corner alone → `r → r_min` at the corner only; gradient ramps over
  the median window.
- Bridge segment in middle → `r = 1.0` for that segment regardless of κ.
- First layer entirely preserved (r = 1.0).
- Mass conservation: `|Σ E_new - target| / Σ E_old < 0.01`.
