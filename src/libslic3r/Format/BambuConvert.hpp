#ifndef slic3r_Format_BambuConvert_hpp_
#define slic3r_Format_BambuConvert_hpp_

#include <array>
#include <string>
#include <vector>

#include "../libslic3r.h"

// Bambu Lab -> Snapmaker U1 .3mf converter, ported and extended from
// josuanbn/bl2u1 (GPL-3.0). The original web tool caps the target at 4
// filaments because the U1 hardware has 4 physical extruders; this port
// removes that cap by synthesising FullSpectrum virtual filaments for any
// overflow.
//
// See doc/filament-economy/BL2U1_NATIVE_PORT.md for the full design.
//
// This v0.1 module exposes the pure-data pieces of the conversion:
//   - color types and conversions (sRGB <-> linear RGB <-> CIELAB)
//   - CIEDE2000 perceptual color distance
//   - mixing of two linear-RGB colors at a ratio
//   - the algorithm that maps N input filaments to 4 physicals + M virtuals
//
// The .3mf zip / XML rewriting and the integration hook in bbs_3mf.cpp
// land in v0.2 once we have real Bambu .3mf fixtures to test against.
namespace Slic3r {
namespace BambuConvert {

// ---------------------------------------------------------------------------
// Color types
// ---------------------------------------------------------------------------

struct Rgb {
    double r = 0; // 0..1 linear
    double g = 0;
    double b = 0;
};

struct Lab {
    double l = 0; // 0..100
    double a = 0; // signed, ~ -128..127
    double b = 0;
};

// Parse "#RRGGBB" / "#RRGGBBAA" into linear RGB. Alpha is ignored. Returns
// {0,0,0} on malformed input.
Rgb parse_srgb_hex(const std::string &hex);

// Encode linear RGB back to "#RRGGBB" (uppercase, no alpha).
std::string format_srgb_hex(const Rgb &c);

// Convert between color spaces.
double  srgb_component_to_linear(double s); // 0..1
double  linear_component_to_srgb(double l); // 0..1
Lab     rgb_to_lab(const Rgb &c);
Rgb     lab_to_rgb(const Lab &c);

// Perceptual color distance (lower is closer). Implementation of the CIE
// DeltaE 2000 formula (CIEDE2000). Range typically 0..100; <2 is
// indistinguishable to most viewers.
double  ciede2000(const Lab &a, const Lab &b);

// Convenience: distance between two sRGB hex codes.
double  delta_e_hex(const std::string &a_hex, const std::string &b_hex);

// ---------------------------------------------------------------------------
// Filament mixing
// ---------------------------------------------------------------------------

// Linear-RGB interpolation. ratio = 1.0 returns `a`, 0.0 returns `b`.
Rgb mix(const Rgb &a, const Rgb &b, double ratio_a);

// A FullSpectrum virtual filament: built from two physical filaments
// alternated layer-by-layer at a given ratio. `ratio_a` is the fraction
// of layers using `physical_a` (the rest use `physical_b`).
struct VirtualFilament {
    size_t physical_a = 0;     // index into the physical filament list
    size_t physical_b = 0;
    double ratio_a    = 0.5;   // 0..1, fraction of A layers
    Rgb    target;             // color we tried to match
    Rgb    achieved;           // color we actually produce (mix output)
    double delta_e    = 0;     // CIEDE2000 to target — quality metric
};

// Given an input filament list (color + relative usage), pick the best 4
// physicals and synthesise a virtual entry for each remaining filament,
// targeting the input color via CIEDE2000 minimisation over all
// (pair_a, pair_b, ratio) combinations.
//
// Returns the indices of the 4 chosen physicals (into the input vector)
// and the per-overflow virtual recipes.
struct InputFilament {
    std::string color_hex;
    double      used_mm  = 0;   // total extrusion length, used to rank
    std::string type;           // "PLA", "PETG", etc — passes through
};

// Selection strategy for the 4 physical filament slots.
//   Usage     — pick top-4 by used_mm. Deterministic, cheap, the
//               default. Works well when the most-used colors are
//               also chromatically diverse.
//   Chromatic — exhaustively search every C(N, 4) physical subset
//               and pick the one that minimises the total overflow
//               CIEDE2000 to the targets. Costlier (C(16, 4) = 1820)
//               but drastically better when a rarely-used color is
//               chromatically isolated and impossible to mix from
//               the heavy-usage palette. See doc/filament-economy/
//               BL2U1_NATIVE_PORT.md for the comparison data on
//               real Bambu prints.
enum class Strategy {
    // Pick top-N by usage. Fast, deterministic. Best when the most-used
    // colors are chromatically diverse.
    Usage     = 0,
    // Exhaustively search C(N, 4) physical subsets, minimise unweighted
    // sum of overflow CIEDE2000. Best raw perceptual quality but ignores
    // how much filament each overflow color actually consumes.
    Chromatic = 1,
    // Same exhaustive search as Chromatic but minimises sum of
    // (CIEDE2000 * used_mm) instead. A high-deltaE overflow on a barely-
    // used color costs less than a low-deltaE drift on a heavy filament.
    // This is usually what users actually want — the metric correlates
    // with how visually wrong the final print looks, not with abstract
    // per-color color distance.
    Balanced  = 2,
};

struct ConvertResult {
    std::array<size_t, 4>        physical_indices = {0, 0, 0, 0};
    size_t                       physical_count   = 0;
    std::vector<VirtualFilament> virtuals;
    // Sum of CIEDE2000 across all virtuals. 0 when there's no overflow.
    // Useful for comparing strategies on perceptual quality alone.
    double                       total_overflow_delta_e          = 0.0;
    // Sum of (CIEDE2000 * max(1, used_mm)) across all virtuals. Used
    // by the Plater auto-pick to choose the strategy that minimises
    // visible impact on the final print, not abstract per-color drift.
    double                       total_overflow_weighted_delta_e = 0.0;
    // Echoes back the strategy actually used (so callers can log it).
    Strategy                     strategy = Strategy::Usage;
};

ConvertResult convert_filament_list(const std::vector<InputFilament> &inputs,
                                    Strategy strategy = Strategy::Usage);

// ---------------------------------------------------------------------------
// Mixing-ratio sampling
// ---------------------------------------------------------------------------

// Discrete ratios sampled when searching for the best virtual mix. These
// are the FullSpectrum cadence values that produce visually distinct
// blends without exceeding the slicer's per-pair complexity budget.
extern const std::array<double, 5> kMixingRatios; // {0.25, 0.333, 0.5, 0.667, 0.75}

} // namespace BambuConvert
} // namespace Slic3r

#endif /* slic3r_Format_BambuConvert_hpp_ */
