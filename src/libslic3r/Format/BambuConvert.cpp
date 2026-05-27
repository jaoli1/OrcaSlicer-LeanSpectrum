#include "BambuConvert.hpp"

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <limits>
#include <numeric>
#include <sstream>

// Bambu Lab -> Snapmaker U1 .3mf converter, pure-data layer.
//
// All public API is documented in BambuConvert.hpp. This .cpp focuses on:
//   1. sRGB <-> linear RGB conversion (IEC 61966-2-1 piecewise, gamma 2.4)
//   2. linear RGB <-> CIELAB via XYZ (D65 illuminant, 2-degree observer)
//   3. CIEDE2000 perceptual color difference (Sharma 2005, eqs. 1-22)
//   4. linear-RGB midpoint mixing used to model a 2-filament FullSpectrum mix
//   5. the assignment algorithm: top-N by usage become physical filaments,
//      every leftover input is matched against the best (pair_a, pair_b, ratio)
//      combination via CIEDE2000 minimisation

namespace Slic3r {
namespace BambuConvert {

namespace {

// D65 reference white, 2-degree observer (sRGB matches D65, so this is
// the natural choice for our sRGB -> Lab pipeline).
constexpr double kXn = 95.047;
constexpr double kYn = 100.000;
constexpr double kZn = 108.883;

inline double clamp01(double v) {
    if (v < 0.0) return 0.0;
    if (v > 1.0) return 1.0;
    return v;
}

inline double deg2rad(double d) { return d * M_PI / 180.0; }
inline double rad2deg(double r) { return r * 180.0 / M_PI; }

inline double pow7(double x) {
    double x2 = x * x;
    return x2 * x2 * x2 * x; // x^7
}

inline int hex_nibble(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return 10 + (c - 'a');
    if (c >= 'A' && c <= 'F') return 10 + (c - 'A');
    return -1;
}

} // namespace

// ---------------------------------------------------------------------------
// Hex parsing / formatting
// ---------------------------------------------------------------------------

Rgb parse_srgb_hex(const std::string &hex) {
    Rgb out;
    if (hex.empty() || hex[0] != '#')
        return out;
    // Accept #RRGGBB (7 chars) or #RRGGBBAA (9 chars); alpha is ignored.
    if (hex.size() != 7 && hex.size() != 9)
        return out;

    int rh = hex_nibble(hex[1]); int rl = hex_nibble(hex[2]);
    int gh = hex_nibble(hex[3]); int gl = hex_nibble(hex[4]);
    int bh = hex_nibble(hex[5]); int bl = hex_nibble(hex[6]);
    if (rh < 0 || rl < 0 || gh < 0 || gl < 0 || bh < 0 || bl < 0)
        return out;

    double r_srgb = ((rh << 4) | rl) / 255.0;
    double g_srgb = ((gh << 4) | gl) / 255.0;
    double b_srgb = ((bh << 4) | bl) / 255.0;

    out.r = srgb_component_to_linear(r_srgb);
    out.g = srgb_component_to_linear(g_srgb);
    out.b = srgb_component_to_linear(b_srgb);
    return out;
}

std::string format_srgb_hex(const Rgb &c) {
    auto to_byte = [](double linear) -> int {
        double s = linear_component_to_srgb(linear);
        int v = (int)std::round(clamp01(s) * 255.0);
        if (v < 0) v = 0;
        if (v > 255) v = 255;
        return v;
    };
    char buf[8];
    std::snprintf(buf, sizeof(buf), "#%02X%02X%02X",
                  to_byte(c.r), to_byte(c.g), to_byte(c.b));
    return std::string(buf);
}

// ---------------------------------------------------------------------------
// sRGB <-> linear RGB
// ---------------------------------------------------------------------------

double srgb_component_to_linear(double s) {
    s = clamp01(s);
    if (s <= 0.04045)
        return s / 12.92;
    return std::pow((s + 0.055) / 1.055, 2.4);
}

double linear_component_to_srgb(double l) {
    l = clamp01(l);
    if (l <= 0.0031308)
        return l * 12.92;
    return 1.055 * std::pow(l, 1.0 / 2.4) - 0.055;
}

// ---------------------------------------------------------------------------
// RGB <-> XYZ <-> Lab (D65)
// ---------------------------------------------------------------------------

namespace {

void rgb_to_xyz(const Rgb &c, double &x, double &y, double &z) {
    // Linear RGB (D65) -> XYZ matrix, sRGB primaries.
    // Output X/Y/Z scaled to 0..100 to align with kXn/kYn/kZn.
    double r = c.r * 100.0;
    double g = c.g * 100.0;
    double b = c.b * 100.0;
    x = r * 0.4124564 + g * 0.3575761 + b * 0.1804375;
    y = r * 0.2126729 + g * 0.7151522 + b * 0.0721750;
    z = r * 0.0193339 + g * 0.1191920 + b * 0.9503041;
}

void xyz_to_rgb(double x, double y, double z, Rgb &c) {
    // Inverse of rgb_to_xyz. X/Y/Z come in scaled to 0..100.
    double x1 = x / 100.0;
    double y1 = y / 100.0;
    double z1 = z / 100.0;
    c.r = clamp01(x1 *  3.2404542 + y1 * -1.5371385 + z1 * -0.4985314);
    c.g = clamp01(x1 * -0.9692660 + y1 *  1.8760108 + z1 *  0.0415560);
    c.b = clamp01(x1 *  0.0556434 + y1 * -0.2040259 + z1 *  1.0572252);
}

inline double f_xyz_to_lab(double t) {
    constexpr double delta = 6.0 / 29.0;
    if (t > delta * delta * delta)
        return std::cbrt(t);
    return t / (3.0 * delta * delta) + 4.0 / 29.0;
}

inline double f_inv_xyz_to_lab(double t) {
    constexpr double delta = 6.0 / 29.0;
    if (t > delta)
        return t * t * t;
    return 3.0 * delta * delta * (t - 4.0 / 29.0);
}

} // namespace

Lab rgb_to_lab(const Rgb &c) {
    double x, y, z;
    rgb_to_xyz(c, x, y, z);
    double fx = f_xyz_to_lab(x / kXn);
    double fy = f_xyz_to_lab(y / kYn);
    double fz = f_xyz_to_lab(z / kZn);
    Lab out;
    out.l = 116.0 * fy - 16.0;
    out.a = 500.0 * (fx - fy);
    out.b = 200.0 * (fy - fz);
    return out;
}

Rgb lab_to_rgb(const Lab &c) {
    double fy = (c.l + 16.0) / 116.0;
    double fx = fy + c.a / 500.0;
    double fz = fy - c.b / 200.0;
    double x = kXn * f_inv_xyz_to_lab(fx);
    double y = kYn * f_inv_xyz_to_lab(fy);
    double z = kZn * f_inv_xyz_to_lab(fz);
    Rgb out;
    xyz_to_rgb(x, y, z, out);
    return out;
}

// ---------------------------------------------------------------------------
// CIEDE2000
// ---------------------------------------------------------------------------

// Implementation of "The CIEDE2000 Color-Difference Formula" by Sharma,
// Wu & Dalal (2005). Variable names follow eqs. 1-22 of that paper.
double ciede2000(const Lab &a, const Lab &b) {
    const double L1 = a.l, a1 = a.a, b1 = a.b;
    const double L2 = b.l, a2 = b.a, b2 = b.b;

    const double C1 = std::sqrt(a1 * a1 + b1 * b1);
    const double C2 = std::sqrt(a2 * a2 + b2 * b2);
    const double Cb = 0.5 * (C1 + C2);

    const double Cb7 = pow7(Cb);
    const double G = 0.5 * (1.0 - std::sqrt(Cb7 / (Cb7 + pow7(25.0))));

    const double a1p = (1.0 + G) * a1;
    const double a2p = (1.0 + G) * a2;
    const double C1p = std::sqrt(a1p * a1p + b1 * b1);
    const double C2p = std::sqrt(a2p * a2p + b2 * b2);

    auto hue_prime = [](double bp, double ap) -> double {
        if (bp == 0.0 && ap == 0.0)
            return 0.0;
        double h = rad2deg(std::atan2(bp, ap));
        if (h < 0.0) h += 360.0;
        return h;
    };
    const double h1p = hue_prime(b1, a1p);
    const double h2p = hue_prime(b2, a2p);

    const double dLp = L2 - L1;
    const double dCp = C2p - C1p;

    double dhp;
    if (C1p * C2p == 0.0) {
        dhp = 0.0;
    } else {
        double diff = h2p - h1p;
        if (diff > 180.0)       dhp = diff - 360.0;
        else if (diff < -180.0) dhp = diff + 360.0;
        else                    dhp = diff;
    }
    const double dHp = 2.0 * std::sqrt(C1p * C2p) * std::sin(deg2rad(dhp) / 2.0);

    const double Lbp = 0.5 * (L1 + L2);
    const double Cbp = 0.5 * (C1p + C2p);

    double Hbp;
    if (C1p * C2p == 0.0) {
        Hbp = h1p + h2p;
    } else {
        double sum = h1p + h2p;
        double diff = std::fabs(h1p - h2p);
        if (diff <= 180.0)         Hbp = 0.5 * sum;
        else if (sum < 360.0)      Hbp = 0.5 * (sum + 360.0);
        else                       Hbp = 0.5 * (sum - 360.0);
    }

    const double T = 1.0
                     - 0.17 * std::cos(deg2rad(Hbp - 30.0))
                     + 0.24 * std::cos(deg2rad(2.0 * Hbp))
                     + 0.32 * std::cos(deg2rad(3.0 * Hbp + 6.0))
                     - 0.20 * std::cos(deg2rad(4.0 * Hbp - 63.0));

    const double dTheta = 30.0 * std::exp(-std::pow((Hbp - 275.0) / 25.0, 2.0));
    const double Cbp7   = pow7(Cbp);
    const double Rc     = 2.0 * std::sqrt(Cbp7 / (Cbp7 + pow7(25.0)));
    const double Sl     = 1.0 + (0.015 * (Lbp - 50.0) * (Lbp - 50.0))
                              / std::sqrt(20.0 + (Lbp - 50.0) * (Lbp - 50.0));
    const double Sc     = 1.0 + 0.045 * Cbp;
    const double Sh     = 1.0 + 0.015 * Cbp * T;
    const double Rt     = -std::sin(deg2rad(2.0 * dTheta)) * Rc;

    const double kL = 1.0, kC = 1.0, kH = 1.0;
    const double tL = dLp / (kL * Sl);
    const double tC = dCp / (kC * Sc);
    const double tH = dHp / (kH * Sh);

    return std::sqrt(tL * tL + tC * tC + tH * tH + Rt * tC * tH);
}

double delta_e_hex(const std::string &a_hex, const std::string &b_hex) {
    return ciede2000(rgb_to_lab(parse_srgb_hex(a_hex)),
                     rgb_to_lab(parse_srgb_hex(b_hex)));
}

// ---------------------------------------------------------------------------
// Mixing
// ---------------------------------------------------------------------------

Rgb mix(const Rgb &a, const Rgb &b, double ratio_a) {
    if (ratio_a < 0.0) ratio_a = 0.0;
    if (ratio_a > 1.0) ratio_a = 1.0;
    const double rb = 1.0 - ratio_a;
    Rgb out;
    out.r = a.r * ratio_a + b.r * rb;
    out.g = a.g * ratio_a + b.g * rb;
    out.b = a.b * ratio_a + b.b * rb;
    return out;
}

// ---------------------------------------------------------------------------
// Mixing-ratio table
// ---------------------------------------------------------------------------

extern const std::array<double, 5> kMixingRatios = {
    0.25, 1.0 / 3.0, 0.5, 2.0 / 3.0, 0.75
};

// ---------------------------------------------------------------------------
// Assignment algorithm
// ---------------------------------------------------------------------------

namespace {

// Internal denser search table. The public kMixingRatios (5 values) is
// kept for backward compat and UI presentation; the assignment search
// uses this finer grid to find better matches when one of the 5
// "natural" cadences lands far from the perceptual sweet spot.
//
// Range deliberately stays inside [0.05, 0.95] — extreme ratios
// (1 layer A every 100 of B) don't produce a clean mix in practice
// because of color bleed across the very-thin film of A. 19 sample
// points × ~12 (a, b) pairs = ~228 evaluations per overflow, which is
// negligible compared to the C(N, cap) outer loop in chromatic mode.
const std::vector<double> &search_ratios()
{
    static const std::vector<double> kSearchRatios = []() {
        std::vector<double> v;
        for (int i = 1; i < 20; ++i) // 0.05, 0.10, ..., 0.95
            v.push_back(i * 0.05);
        return v;
    }();
    return kSearchRatios;
}

// Given a fixed list of physical filament inputs, compute the best
// virtual recipe for one overflow target. Pure function — returns
// the recipe with the lowest CIEDE2000.
VirtualFilament best_virtual_for_target(const std::vector<Rgb> &phys_rgb,
                                        const Rgb              &target_rgb,
                                        const Lab              &target_lab) {
    VirtualFilament best;
    best.physical_a = 0;
    best.physical_b = 0;
    best.ratio_a    = 0.5;
    best.target     = target_rgb;
    best.achieved   = phys_rgb.empty() ? Rgb{} : phys_rgb[0];
    best.delta_e    = std::numeric_limits<double>::infinity();

    const std::vector<double> &ratios = search_ratios();

    for (size_t a = 0; a < phys_rgb.size(); ++a) {
        for (size_t b = 0; b < phys_rgb.size(); ++b) {
            if (a == b) continue; // mixing X with X is just X
            for (double ratio_a : ratios) {
                Rgb mixed = mix(phys_rgb[a], phys_rgb[b], ratio_a);
                double de = ciede2000(rgb_to_lab(mixed), target_lab);
                if (de < best.delta_e) {
                    best.delta_e    = de;
                    best.physical_a = a;
                    best.physical_b = b;
                    best.ratio_a    = ratio_a;
                    best.achieved   = mixed;
                }
            }
        }
    }
    return best;
}

// For a candidate physical set, synthesize virtuals for every other
// input and return them along with the total deltaE sum (raw and
// usage-weighted).
struct AssignmentEval {
    std::vector<VirtualFilament> virtuals;
    double                       total_delta_e          = 0.0;
    double                       total_weighted_delta_e = 0.0;
};

AssignmentEval evaluate_assignment(const std::vector<InputFilament> &inputs,
                                   const std::vector<size_t>        &physicals) {
    AssignmentEval out;
    std::vector<Rgb> phys_rgb;
    phys_rgb.reserve(physicals.size());
    for (size_t p : physicals)
        phys_rgb.push_back(parse_srgb_hex(inputs[p].color_hex));

    std::vector<bool> in_phys(inputs.size(), false);
    for (size_t p : physicals)
        in_phys[p] = true;

    for (size_t k = 0; k < inputs.size(); ++k) {
        if (in_phys[k]) continue;
        const Rgb target_rgb = parse_srgb_hex(inputs[k].color_hex);
        const Lab target_lab = rgb_to_lab(target_rgb);
        VirtualFilament v = best_virtual_for_target(phys_rgb, target_rgb, target_lab);
        out.virtuals.push_back(v);
        out.total_delta_e          += v.delta_e;
        // Weight by used_mm; floor at 1 mm so a zero-usage filament still
        // contributes a tiny amount (and so an entirely synthetic input
        // list with used_mm = 0 collapses to the unweighted metric).
        const double w = std::max(1.0, inputs[k].used_mm);
        out.total_weighted_delta_e += v.delta_e * w;
    }
    return out;
}

// Pick physicals by usage ranking (top-N descending). Deterministic.
std::vector<size_t> pick_by_usage(const std::vector<InputFilament> &inputs,
                                  size_t cap) {
    std::vector<size_t> order(inputs.size());
    std::iota(order.begin(), order.end(), size_t{0});
    std::sort(order.begin(), order.end(), [&](size_t i, size_t j) {
        if (inputs[i].used_mm != inputs[j].used_mm)
            return inputs[i].used_mm > inputs[j].used_mm;
        return i < j;
    });
    const size_t n = std::min(cap, order.size());
    order.resize(n);
    return order;
}

// Exhaustively enumerate C(N, cap) physical subsets and pick the one
// minimising the chosen overflow metric. Tie-break by lexicographic
// combo for determinism.
//
// `weighted` = false  -> minimise sum of CIEDE2000 (chromatic strategy)
// `weighted` = true   -> minimise sum of CIEDE2000 * used_mm (balanced)
std::vector<size_t> pick_by_exhaustive(const std::vector<InputFilament> &inputs,
                                       size_t cap,
                                       bool   weighted) {
    const size_t n = inputs.size();
    if (n <= cap) {
        std::vector<size_t> all(n);
        std::iota(all.begin(), all.end(), size_t{0});
        return all;
    }

    std::vector<size_t> best;
    double best_total = std::numeric_limits<double>::infinity();

    std::vector<size_t> combo(cap);
    std::iota(combo.begin(), combo.end(), size_t{0});
    while (true) {
        AssignmentEval ev = evaluate_assignment(inputs, combo);
        const double cost = weighted ? ev.total_weighted_delta_e
                                     : ev.total_delta_e;
        if (cost < best_total) {
            best_total = cost;
            best       = combo;
        }
        size_t i = cap;
        while (i-- > 0) {
            if (combo[i] < n - (cap - i)) {
                ++combo[i];
                for (size_t j = i + 1; j < cap; ++j)
                    combo[j] = combo[j - 1] + 1;
                break;
            }
            if (i == 0)
                return best;
        }
    }
}

} // namespace

ConvertResult convert_filament_list(const std::vector<InputFilament> &inputs,
                                    Strategy strategy) {
    ConvertResult result;
    result.strategy = strategy;
    if (inputs.empty())
        return result;

    const size_t cap = 4; // U1 physical extruder count

    std::vector<size_t> physicals;
    switch (strategy) {
        case Strategy::Usage:
            physicals = pick_by_usage(inputs, cap);
            break;
        case Strategy::Chromatic:
            physicals = pick_by_exhaustive(inputs, cap, /*weighted=*/false);
            break;
        case Strategy::Balanced:
            physicals = pick_by_exhaustive(inputs, cap, /*weighted=*/true);
            break;
    }

    const size_t n_phys = std::min(cap, physicals.size());
    result.physical_count = n_phys;
    for (size_t i = 0; i < n_phys; ++i)
        result.physical_indices[i] = physicals[i];
    // Pad remaining slots with the last physical (harmless: physical_count
    // is the authoritative size).
    for (size_t i = n_phys; i < cap; ++i)
        result.physical_indices[i] = n_phys > 0 ? physicals[n_phys - 1] : 0;

    if (inputs.size() <= cap)
        return result;

    // Synthesize the virtuals against the picked physical set.
    AssignmentEval ev = evaluate_assignment(inputs, physicals);
    result.virtuals                        = std::move(ev.virtuals);
    result.total_overflow_delta_e          = ev.total_delta_e;
    result.total_overflow_weighted_delta_e = ev.total_weighted_delta_e;
    return result;
}

} // namespace BambuConvert
} // namespace Slic3r
