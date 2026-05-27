#include "FullSpectrumDither.hpp"

#include <algorithm>
#include <cstdint>

namespace Slic3r {
namespace FullSpectrumDither {

namespace {

// 1D Floyd-Steinberg with constant eastward weight 1.0 (no neighbour
// coupling beyond the next index — Z is intrinsically 1D for our
// purposes). We re-run the loop from layer 0 each call to remain
// stateless and thread-safe; the manager can cache if profiling
// shows this is hot.
bool floyd_steinberg_2way(int layer_index, int ratio_a, int ratio_b,
                          double bias_per_layer) {
    if (layer_index < 0) layer_index = 0;
    ratio_a = std::max(0, ratio_a);
    ratio_b = std::max(0, ratio_b);
    const int sum = ratio_a + ratio_b;
    if (sum == 0)
        return false;
    if (ratio_a == 0) return true;
    if (ratio_b == 0) return false;

    const double target = double(ratio_b) / double(sum);
    double err  = 0.0;
    bool   pick = false;

    for (int i = 0; i <= layer_index; ++i) {
        err += target + bias_per_layer;
        if (err >= 0.5) {
            pick = true;
            err -= 1.0;
        } else {
            pick = false;
        }
    }
    return pick;
}

} // namespace

bool use_component_b_floyd_steinberg(int layer_index, int ratio_a, int ratio_b) {
    return floyd_steinberg_2way(layer_index, ratio_a, ratio_b, 0.0);
}

std::size_t error_diffusion_select(int layer_index,
                                   const std::vector<int> &weights) {
    if (weights.empty())
        return 0;
    if (layer_index < 0)
        layer_index = 0;

    // Sum and skip the empty / degenerate cases.
    int sum = 0;
    for (int w : weights)
        sum += std::max(0, w);
    if (sum == 0)
        return 0;

    // Per-component targets in [0, 1] summing to 1.
    const std::size_t n = weights.size();
    std::vector<double> target(n);
    for (std::size_t k = 0; k < n; ++k)
        target[k] = double(std::max(0, weights[k])) / double(sum);

    // Walk from layer 0, maintain per-component error, pick the
    // component with the largest accumulated error each step.
    std::vector<double> err(n, 0.0);
    std::size_t pick = 0;
    for (int i = 0; i <= layer_index; ++i) {
        for (std::size_t k = 0; k < n; ++k)
            err[k] += target[k];

        // Find the largest err[k]; tie-break by lowest index for
        // determinism.
        pick = 0;
        double best = err[0];
        for (std::size_t k = 1; k < n; ++k) {
            if (err[k] > best) {
                best = err[k];
                pick = k;
            }
        }
        err[pick] -= 1.0;
    }
    return pick;
}

bool use_component_b_curvature_dither(int    layer_index,
                                      int    ratio_a,
                                      int    ratio_b,
                                      double curvature_gain) {
    // Clamp gain to a sensible range. A gain of 1.0 effectively forces
    // a transition every layer regardless of target; anything larger
    // would just clip identically.
    if (curvature_gain >  1.0) curvature_gain =  1.0;
    if (curvature_gain < -1.0) curvature_gain = -1.0;

    // Apply the gain as a small additive bias on the per-layer
    // target. Scaling by 0.5 keeps the bias bounded so positive gain
    // increases the transition rate proportionally without saturating
    // the dither.
    return floyd_steinberg_2way(layer_index, ratio_a, ratio_b,
                                0.5 * curvature_gain);
}

} // namespace FullSpectrumDither
} // namespace Slic3r
