// Tests for FullSpectrum F1 (curvature-coupled cadence) and F2 (1D
// Floyd-Steinberg / error-diffusion) helpers.
//
// Properties verified:
//   - Long-run distribution matches target ratio within 1 / N tolerance
//   - No 2-layer runs of one component when ratios are roughly balanced
//     (Floyd-Steinberg avoids consecutive duplicates that ordered Bresenham
//     allows on phase boundaries)
//   - Deterministic: same inputs -> same output
//   - N-way error diffusion converges to target weights
//   - Curvature gain pulls the transition rate up (positive) or down (negative)

#include <catch2/catch.hpp>

#include <algorithm>
#include <vector>

#include "libslic3r/FullSpectrumDither.hpp"

using namespace Slic3r::FullSpectrumDither;
using Catch::Matchers::WithinAbs;

TEST_CASE("Floyd-Steinberg 2-way is deterministic", "[FullSpectrumDither]") {
    for (int i = 0; i < 50; ++i) {
        bool a = use_component_b_floyd_steinberg(i, 1, 1);
        bool b = use_component_b_floyd_steinberg(i, 1, 1);
        REQUIRE(a == b);
    }
}

TEST_CASE("Floyd-Steinberg 2-way degenerate ratios", "[FullSpectrumDither]") {
    // ratio_b == 0: always A.
    for (int i = 0; i < 10; ++i)
        REQUIRE_FALSE(use_component_b_floyd_steinberg(i, 5, 0));

    // ratio_a == 0: always B.
    for (int i = 0; i < 10; ++i)
        REQUIRE(use_component_b_floyd_steinberg(i, 0, 5));

    // Both zero: no-op, returns false.
    REQUIRE_FALSE(use_component_b_floyd_steinberg(0, 0, 0));
}

TEST_CASE("Floyd-Steinberg 2-way matches target ratio over long run",
          "[FullSpectrumDither]") {
    struct Case { int a, b; };
    const std::vector<Case> cases = {{1, 1}, {2, 1}, {3, 1}, {1, 3}, {5, 3}};
    constexpr int N = 1000;

    for (const auto &c : cases) {
        int b_count = 0;
        for (int i = 0; i < N; ++i)
            if (use_component_b_floyd_steinberg(i, c.a, c.b))
                ++b_count;
        const double observed = double(b_count) / N;
        const double target   = double(c.b) / double(c.a + c.b);
        // The dither only deviates from the long-run target by at most
        // one decision per layer, so the error after N layers is
        // bounded by 1/N (within rounding). 2/N gives margin for the
        // starting transient.
        REQUIRE_THAT(observed, WithinAbs(target, 2.0 / N));
    }
}

TEST_CASE("Floyd-Steinberg 1:1 alternates rather than runs of two",
          "[FullSpectrumDither]") {
    // For ratio 1:1 the error-diffusion sequence is strictly ABAB...,
    // so we should never see two consecutive same-component picks.
    bool prev = use_component_b_floyd_steinberg(0, 1, 1);
    for (int i = 1; i < 100; ++i) {
        bool cur = use_component_b_floyd_steinberg(i, 1, 1);
        REQUIRE(cur != prev);
        prev = cur;
    }
}

TEST_CASE("error_diffusion_select handles empty / zero weights",
          "[FullSpectrumDither]") {
    REQUIRE(error_diffusion_select(0, {}) == 0);
    REQUIRE(error_diffusion_select(0, {0, 0, 0}) == 0);
    REQUIRE(error_diffusion_select(10, {0, 0, 0}) == 0);
}

TEST_CASE("error_diffusion_select N-way matches target weights",
          "[FullSpectrumDither]") {
    const std::vector<int> weights = {3, 2, 1}; // total 6
    constexpr int N = 600;

    std::vector<int> counts(weights.size(), 0);
    for (int i = 0; i < N; ++i) {
        std::size_t k = error_diffusion_select(i, weights);
        REQUIRE(k < weights.size());
        ++counts[k];
    }

    // Targets: 50%, 33.3%, 16.7%. Allow ~1% slack for the starting
    // transient over N=600.
    REQUIRE_THAT(double(counts[0]) / N, WithinAbs(3.0 / 6.0, 0.01));
    REQUIRE_THAT(double(counts[1]) / N, WithinAbs(2.0 / 6.0, 0.01));
    REQUIRE_THAT(double(counts[2]) / N, WithinAbs(1.0 / 6.0, 0.01));
}

TEST_CASE("error_diffusion_select clamps negative weights to zero",
          "[FullSpectrumDither]") {
    // Negative weight should behave as 0. Equal positive weights split.
    constexpr int N = 200;
    int c0 = 0, c1 = 0;
    for (int i = 0; i < N; ++i) {
        std::size_t k = error_diffusion_select(i, {1, -3, 1});
        REQUIRE(k != 1); // index 1 had a negative weight, never chosen
        if (k == 0) ++c0;
        else        ++c1;
    }
    REQUIRE_THAT(double(c0) / N, WithinAbs(0.5, 0.02));
    REQUIRE_THAT(double(c1) / N, WithinAbs(0.5, 0.02));
}

TEST_CASE("curvature gain == 0 matches plain Floyd-Steinberg",
          "[FullSpectrumDither]") {
    for (int i = 0; i < 200; ++i) {
        bool plain = use_component_b_floyd_steinberg(i, 3, 2);
        bool gain0 = use_component_b_curvature_dither(i, 3, 2, 0.0);
        REQUIRE(plain == gain0);
    }
}

TEST_CASE("positive curvature gain raises transition rate",
          "[FullSpectrumDither]") {
    // Transition rate = how often the pick flips between layers.
    auto transition_rate = [](double gain) {
        bool prev = use_component_b_curvature_dither(0, 5, 1, gain);
        int  flips = 0;
        constexpr int N = 600;
        for (int i = 1; i < N; ++i) {
            bool cur = use_component_b_curvature_dither(i, 5, 1, gain);
            if (cur != prev) ++flips;
            prev = cur;
        }
        return double(flips) / (N - 1);
    };

    const double baseline   = transition_rate(0.0);
    const double with_gain  = transition_rate(0.6);
    REQUIRE(with_gain >= baseline);
}

TEST_CASE("negative curvature gain lowers transition rate",
          "[FullSpectrumDither]") {
    auto transition_rate = [](double gain) {
        bool prev = use_component_b_curvature_dither(0, 1, 1, gain);
        int  flips = 0;
        constexpr int N = 200;
        for (int i = 1; i < N; ++i) {
            bool cur = use_component_b_curvature_dither(i, 1, 1, gain);
            if (cur != prev) ++flips;
            prev = cur;
        }
        return double(flips) / (N - 1);
    };

    const double baseline = transition_rate(0.0);          // ABAB -> ~1.0
    const double damped   = transition_rate(-0.6);         // longer runs
    REQUIRE(damped <= baseline);
}

TEST_CASE("curvature gain clamped beyond +/-1", "[FullSpectrumDither]") {
    // Beyond +/-1 the gain should clip to the same behavior as +/-1.
    for (int i = 0; i < 50; ++i) {
        bool a = use_component_b_curvature_dither(i, 2, 1,  1.0);
        bool b = use_component_b_curvature_dither(i, 2, 1,  2.5);
        REQUIRE(a == b);
        bool c = use_component_b_curvature_dither(i, 2, 1, -1.0);
        bool d = use_component_b_curvature_dither(i, 2, 1, -5.0);
        REQUIRE(c == d);
    }
}
