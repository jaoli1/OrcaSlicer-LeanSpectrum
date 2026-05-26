#ifndef slic3r_FullSpectrumDither_hpp_
#define slic3r_FullSpectrumDither_hpp_

#include <cstddef>
#include <vector>

// FullSpectrum dithering helpers — F1 (curvature-coupled cadence) and
// F2 (1D Floyd-Steinberg error diffusion on Z).
//
// These are pure, side-effect-free helpers that can be unit-tested in
// isolation from the slicing pipeline. MixedFilamentManager wires them
// in opt-in; see doc/filament-economy/FULLSPECTRUM_F1_F2.md for the
// integration plan and rationale.
//
// Why F2 (Floyd-Steinberg) over the existing rotated-Bresenham helper:
//   - Bresenham gives a periodic phase-stable pattern; over long runs
//     the same A/B alternation repeats, which can produce subtle visible
//     vertical banding.
//   - 1D error diffusion (Floyd-Steinberg with eastward weight 1.0) on
//     a constant target collapses to the same long-run *average* but
//     desynchronises the phase, eliminating the banding without adding
//     transitions.
//   - On a non-constant target (true gradient with per-layer weights),
//     Floyd-Steinberg is the right tool: it tracks the running error
//     and dispenses transitions where they minimise drift.
//
// Why F1 (curvature gain):
//   - A region with dense Z-curvature features (small overhangs, sharp
//     corners) benefits from extra A/B transitions to mask the seams.
//   - A flat region prefers longer runs (fewer transitions = fewer
//     interfaces where the components don't fully homogenise).
//   - We expose this as a per-layer "curvature gain" in [-1, +1] that
//     biases the dither threshold: positive = prefer transition,
//     negative = prefer hold.

namespace Slic3r {
namespace FullSpectrumDither {

// ---------------------------------------------------------------------------
// F2: Floyd-Steinberg / 1D error diffusion
// ---------------------------------------------------------------------------

// Pick component B (true) or A (false) for `layer_index`, given the
// integer ratios (ratio_a + ratio_b > 0). Uses 1D Floyd-Steinberg with
// eastward weight 1.0 starting from layer 0.
//
// Cost: O(layer_index). For typical 1000-3000 layer prints this is fine
// (sub-microsecond per call). For very tall prints with frequent resolve
// calls, callers can cache the resulting pattern.
//
// Deterministic: same inputs always yield the same output.
bool use_component_b_floyd_steinberg(int layer_index,
                                     int ratio_a,
                                     int ratio_b);

// N-way error-diffusion select. Given integer `weights` (each >= 0,
// sum > 0), returns an index in [0, weights.size()) for the given
// layer index such that the long-run distribution matches the weight
// ratios with minimal banding.
//
// When weights.size() == 2 this is identical to
// use_component_b_floyd_steinberg interpreted with weights = {a, b}.
std::size_t error_diffusion_select(int layer_index,
                                   const std::vector<int> &weights);

// ---------------------------------------------------------------------------
// F1: curvature-coupled cadence (gain modulation of the dither threshold)
// ---------------------------------------------------------------------------

// Pick component B (true) or A (false) for `layer_index`, with the
// dither threshold biased by `curvature_gain` in [-1, +1].
//
// curvature_gain == 0  -> identical to use_component_b_floyd_steinberg.
// curvature_gain > 0   -> prefer transition (use more swaps).
// curvature_gain < 0   -> prefer hold (use longer runs of A or B).
//
// The gain is clamped to [-1, +1] and applied as an additive offset on
// the accumulated error at decision time. This keeps the *long-run*
// average ratio close to the target while letting local geometry pull
// transitions toward (or away from) the current layer.
bool use_component_b_curvature_dither(int    layer_index,
                                      int    ratio_a,
                                      int    ratio_b,
                                      double curvature_gain);

} // namespace FullSpectrumDither
} // namespace Slic3r

#endif /* slic3r_FullSpectrumDither_hpp_ */
