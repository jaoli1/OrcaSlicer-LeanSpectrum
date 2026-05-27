# FullSpectrum optimisations (F1, F2)

> Two improvements to the FullSpectrum mixed-color algorithm that
> sharpen the apparent color and integrate cleanly with the
> curvature-aware infrastructure of Pass 4.

## Why FullSpectrum needs help

FullSpectrum alternates layers between two physical filaments
(`A`, `B`) at a fixed cadence — typically 1 A layer + 1 B layer.
With layers of 0.20 mm and a bias of 0, the eye perceives the mean
color but layer banding remains visible at oblique angles, on edges,
and where the print transitions in curvature.

Two structural issues:

1. **Static cadence**: the cadence is set globally, but the *visible*
   contribution of a layer depends on whether the part is flat (each
   layer fully visible from the side) or curved (layers are seen at
   varying angles and partially occluded). High-curvature regions
   need finer cadence to keep banding below the perceptual threshold.

2. **Static bias**: bias is a single per-pair offset applied uniformly.
   It cannot compensate for accumulated rounding error along Z, so over
   tall prints the apparent color drifts from the target.

F1 fixes (1); F2 fixes (2).

## F1 — Curvature-coupled cadence modulation

### Idea

Re-use the curvature signal computed by Pass 4. In high-curvature
regions, multiply the FullSpectrum cadence heights by a factor < 1
(finer alternation). In low-curvature regions, multiply by a factor
> 1 (coarser alternation, saves material).

### Algorithm sketch

```
Per layer L of the mixed-color part:
    κ_L = mean curvature of all outer-wall segments in L
    factor_L = lerp(κ_L, [κ_low, κ_high], [k_max, k_min])
            where k_min = 0.5  (finest cadence in high curvature)
                  k_max = 1.5  (coarsest in low curvature)
    height_A_L = base_height_A * factor_L
    height_B_L = base_height_B * factor_L
```

The cadence still alternates A/B/A/B/... but each step is now Z-scaled
by `factor_L`. Aggregated over the part, the *average* layer thickness
is unchanged (factor_L stays close to 1.0 on average) so global Z-stack
is preserved.

### Where to hook

`MixedFilament::resolve_layers()` (in `src/libslic3r/MixedFilament.cpp`)
computes the per-layer A/B sequence from the base cadence. We add a
curvature-aware wrapper that consults a per-layer κ map prepared by the
geometry pre-analysis stage.

### Constraints

- Total Z must still match the original layer Z-stack to keep the rest
  of the slicer happy. We enforce `Σ factor_L * base_height = original
  Z_stack`.
- Minimum cadence height bounded by the printer's physical Z resolution
  (0.05 mm on U1 by spec).
- Bias (F2) operates on top of factor_L.

## F2 — 1D Floyd–Steinberg dithering along Z

### Idea

In image dithering, when a pixel cannot represent the exact target
color, the residual error is propagated to neighbouring pixels. We
apply the same principle along the Z axis of a FullSpectrum print:
each layer must pick A or B (binary choice per layer when cadence
factor is 1), and the residual error after that choice is propagated
to the next layer.

This makes the apparent color converge to the target over a few layers
even when no static bias would exactly hit it.

### Algorithm

For a pair `(A, B)` with target proportion `t_target ∈ [0, 1]` (fraction
of B desired in the mix):

```
error = 0
for layer L in mixed region:
    desired_B = t_target + error
    if desired_B >= 0.5:
        emit B for this layer
        error = desired_B - 1.0
    else:
        emit A for this layer
        error = desired_B
```

This is a 1D error-diffusion dither. Over N layers, the actual
proportion of B converges to `t_target` with error O(1/N).

### Where to hook

Inside `MixedFilament::resolve_layers()` when computing the per-layer
A/B identity. Replace the current bias-based check
(`layer_index % cadence + bias_offset`) with the F2 error-diffusion
loop.

### Compatibility

- The legacy bias setting remains as the F2 *initial error* (so
  existing projects produce the same look on the first few layers).
- F2 is opt-in via a new setting `mixed_filament_dither_floyd_steinberg`.

## Settings

| Key                                            | Type | Default |
|------------------------------------------------|------|---------|
| `mixed_filament_curvature_modulation`          | bool | true    |
| `mixed_filament_curvature_cadence_min`         | float| 0.5     |
| `mixed_filament_curvature_cadence_max`         | float| 1.5     |
| `mixed_filament_dither_floyd_steinberg`        | bool | true    |

## Tests

- **F1**: A part with flat top and curved sides → flat top uses coarser
  cadence (factor > 1), sides use finer cadence. Sum of factors × base
  height matches the original Z-stack within 0.1 %.
- **F2**: For `t_target = 0.4`, 100 layers → the actual count of B
  layers is 40 ± 1. Apparent color (in CIELAB) within ΔE=1.0 of target.
- **F1 + F2**: combined, no Z drift, no banding visible at 20× zoom on
  a calibration print.

## Risks

- F1 changes Z geometry per-layer. If the slicer's wipe tower assumes
  uniform Z, the tower may become uneven. Mitigation: include the wipe
  tower in the layer Z calculation when modulating.
- F2 introduces non-determinism in the *layer count* of each component
  (could be N or N±1 for a given run). Not a functional issue but
  surprising in CI tests — pin the RNG / error seed to make it
  deterministic.
