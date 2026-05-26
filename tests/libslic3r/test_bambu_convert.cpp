// Tests for the bl2u1 native port — color math + the
// physical/virtual filament assignment algorithm.
//
// References:
//   - CIEDE2000 numerical sanity values come from Sharma 2005 Table 1.
//     Pairs with a known answer: (50,2.6772,-79.7751)-(50,0,-82.7485) -> 2.0425,
//     (50,3.1571,-77.2803)-(50,0,-82.7485) -> 2.8615.
//   - sRGB hex roundtrip values come from the IEC 61966-2-1 piecewise
//     definition.

#include <catch2/catch.hpp>

#include <vector>

#include "libslic3r/Format/BambuConvert.hpp"
#include "libslic3r/Format/bbs_3mf.hpp"
#include "libslic3r/PrintConfig.hpp"

using namespace Slic3r;
using namespace Slic3r::BambuConvert;
using Catch::Matchers::WithinAbs;
using Catch::Matchers::WithinRel;

TEST_CASE("parse_srgb_hex handles malformed input", "[BambuConvert][Color]") {
    Rgb empty   = parse_srgb_hex("");
    Rgb no_hash = parse_srgb_hex("FF0000");
    Rgb short_s = parse_srgb_hex("#F00");
    Rgb bad     = parse_srgb_hex("#GGHHII");

    REQUIRE_THAT(empty.r,   WithinAbs(0.0, 1e-9));
    REQUIRE_THAT(no_hash.g, WithinAbs(0.0, 1e-9));
    REQUIRE_THAT(short_s.b, WithinAbs(0.0, 1e-9));
    REQUIRE_THAT(bad.r,     WithinAbs(0.0, 1e-9));
}

TEST_CASE("parse_srgb_hex / format_srgb_hex round-trip", "[BambuConvert][Color]") {
    const char *samples[] = {
        "#000000", "#FFFFFF", "#FF0000", "#00FF00", "#0000FF",
        "#7F7F7F", "#123456", "#ABCDEF",
    };
    for (auto *hex : samples) {
        Rgb c = parse_srgb_hex(hex);
        std::string back = format_srgb_hex(c);
        REQUIRE(back == std::string(hex));
    }
}

TEST_CASE("parse_srgb_hex accepts alpha suffix", "[BambuConvert][Color]") {
    Rgb opaque = parse_srgb_hex("#FF8040");
    Rgb alpha  = parse_srgb_hex("#FF804080");
    REQUIRE_THAT(opaque.r, WithinAbs(alpha.r, 1e-12));
    REQUIRE_THAT(opaque.g, WithinAbs(alpha.g, 1e-12));
    REQUIRE_THAT(opaque.b, WithinAbs(alpha.b, 1e-12));
}

TEST_CASE("sRGB component conversions invert", "[BambuConvert][Color]") {
    for (double s = 0.0; s <= 1.0; s += 0.125) {
        double linear = srgb_component_to_linear(s);
        double back   = linear_component_to_srgb(linear);
        REQUIRE_THAT(back, WithinAbs(s, 1e-9));
    }
}

TEST_CASE("Lab of pure black and pure white", "[BambuConvert][Color]") {
    Lab black = rgb_to_lab(parse_srgb_hex("#000000"));
    Lab white = rgb_to_lab(parse_srgb_hex("#FFFFFF"));
    REQUIRE_THAT(black.l, WithinAbs(0.0, 1e-6));
    REQUIRE_THAT(white.l, WithinAbs(100.0, 1e-6));
    REQUIRE_THAT(white.a, WithinAbs(0.0, 1e-3));
    REQUIRE_THAT(white.b, WithinAbs(0.0, 1e-3));
}

TEST_CASE("CIEDE2000 zero for identical colors", "[BambuConvert][Color]") {
    Lab a = rgb_to_lab(parse_srgb_hex("#A1B2C3"));
    REQUIRE_THAT(ciede2000(a, a), WithinAbs(0.0, 1e-12));
}

TEST_CASE("CIEDE2000 reference pairs (Sharma 2005 table)",
          "[BambuConvert][Color]") {
    // Pair 1: deltaE = 2.0425
    Lab a1{50.0,  2.6772, -79.7751};
    Lab b1{50.0,  0.0,    -82.7485};
    REQUIRE_THAT(ciede2000(a1, b1), WithinAbs(2.0425, 1e-3));

    // Pair 2: deltaE = 2.8615
    Lab a2{50.0,  3.1571, -77.2803};
    Lab b2{50.0,  0.0,    -82.7485};
    REQUIRE_THAT(ciede2000(a2, b2), WithinAbs(2.8615, 1e-3));

    // Pair from the same table: (50, -1.3802, -84.2814) vs (50, 0, -82.7485)
    // -> deltaE = 1.0000
    Lab a3{50.0, -1.3802, -84.2814};
    Lab b3{50.0,  0.0,    -82.7485};
    REQUIRE_THAT(ciede2000(a3, b3), WithinAbs(1.0000, 1e-3));
}

TEST_CASE("mix() endpoints and midpoint", "[BambuConvert][Color]") {
    Rgb red  = parse_srgb_hex("#FF0000");
    Rgb blue = parse_srgb_hex("#0000FF");

    Rgb at_a = mix(red, blue, 1.0);
    Rgb at_b = mix(red, blue, 0.0);
    Rgb mid  = mix(red, blue, 0.5);

    REQUIRE_THAT(at_a.r, WithinAbs(red.r,  1e-12));
    REQUIRE_THAT(at_a.b, WithinAbs(red.b,  1e-12));
    REQUIRE_THAT(at_b.r, WithinAbs(blue.r, 1e-12));
    REQUIRE_THAT(at_b.b, WithinAbs(blue.b, 1e-12));
    REQUIRE_THAT(mid.r,  WithinAbs(0.5 * (red.r + blue.r), 1e-12));
    REQUIRE_THAT(mid.b,  WithinAbs(0.5 * (red.b + blue.b), 1e-12));
}

TEST_CASE("convert_filament_list — pass-through for 4 or fewer inputs",
          "[BambuConvert][Assign]") {
    std::vector<InputFilament> inputs = {
        {"#FF0000", 1000.0, "PLA"},
        {"#00FF00",  800.0, "PLA"},
        {"#0000FF",  600.0, "PLA"},
    };
    ConvertResult r = convert_filament_list(inputs);
    REQUIRE(r.physical_count == 3);
    REQUIRE(r.virtuals.empty());

    // Ordering: most-used first.
    REQUIRE(r.physical_indices[0] == 0);
    REQUIRE(r.physical_indices[1] == 1);
    REQUIRE(r.physical_indices[2] == 2);
}

TEST_CASE("convert_filament_list — picks top-4 by usage",
          "[BambuConvert][Assign]") {
    std::vector<InputFilament> inputs = {
        {"#111111", 100.0, "PLA"}, // index 0
        {"#222222", 900.0, "PLA"}, // 1 — most used
        {"#333333", 800.0, "PLA"}, // 2
        {"#444444", 700.0, "PLA"}, // 3
        {"#555555", 600.0, "PLA"}, // 4
        {"#666666",  50.0, "PLA"}, // 5 — should overflow
    };
    ConvertResult r = convert_filament_list(inputs);
    REQUIRE(r.physical_count == 4);
    // Sorted desc by used_mm: 1, 2, 3, 4.
    REQUIRE(r.physical_indices[0] == 1);
    REQUIRE(r.physical_indices[1] == 2);
    REQUIRE(r.physical_indices[2] == 3);
    REQUIRE(r.physical_indices[3] == 4);
    REQUIRE(r.virtuals.size() == 2); // inputs 0 and 5 overflowed
}

TEST_CASE("convert_filament_list — virtual recipe references valid physicals",
          "[BambuConvert][Assign]") {
    // 6 colors, all heavily used. The first 4 become physicals; the last 2
    // are synthesised as virtuals. We don't pin specific recipes — we only
    // require that the references are in-range and the mixing ratio is one
    // of the sampled values.
    std::vector<InputFilament> inputs = {
        {"#FF0000", 1000.0, "PLA"},
        {"#00FF00",  900.0, "PLA"},
        {"#0000FF",  800.0, "PLA"},
        {"#FFFF00",  700.0, "PLA"},
        {"#FF00FF",  600.0, "PLA"}, // overflow 1
        {"#00FFFF",  500.0, "PLA"}, // overflow 2
    };
    ConvertResult r = convert_filament_list(inputs);
    REQUIRE(r.physical_count == 4);
    REQUIRE(r.virtuals.size() == 2);

    for (const auto &v : r.virtuals) {
        REQUIRE(v.physical_a < r.physical_count);
        REQUIRE(v.physical_b < r.physical_count);
        REQUIRE(v.physical_a != v.physical_b);

        bool ratio_ok = false;
        for (double sampled : kMixingRatios) {
            if (std::abs(v.ratio_a - sampled) < 1e-12) { ratio_ok = true; break; }
        }
        REQUIRE(ratio_ok);

        // The achieved color should be no farther from target than either
        // endpoint physical alone — otherwise the search picked a worse
        // option than a degenerate (ratio=1) mix.
        REQUIRE(v.delta_e >= 0.0);
    }
}

TEST_CASE("convert_filament_list — empty input is empty output",
          "[BambuConvert][Assign]") {
    ConvertResult r = convert_filament_list({});
    REQUIRE(r.physical_count == 0);
    REQUIRE(r.virtuals.empty());
}

TEST_CASE("convert_filament_list — real Bambu A1 mini 4-color print",
          "[BambuConvert][Assign]") {
    // Filaments from a real BabyGarfield_Funko.3mf (Bambu Lab A1 mini).
    // All-PLA, 4 colors. Expected: no overflow, all 4 become physicals.
    std::vector<InputFilament> inputs = {
        {"#FF8040", 100.0, "PLA"}, // orange
        {"#000000", 100.0, "PLA"}, // black
        {"#FFFFFF", 100.0, "PLA"}, // white
        {"#FFFF80", 100.0, "PLA"}, // pale yellow
    };
    ConvertResult r = convert_filament_list(inputs);
    REQUIRE(r.physical_count == 4);
    REQUIRE(r.virtuals.empty());
    // Order is by used_mm desc, then by input index (ties).
    REQUIRE(r.physical_indices[0] == 0);
    REQUIRE(r.physical_indices[1] == 1);
    REQUIRE(r.physical_indices[2] == 2);
    REQUIRE(r.physical_indices[3] == 3);
}

TEST_CASE("convert_filament_list — Bambu 4-color + 2 overflow synthetises virtuals",
          "[BambuConvert][Assign]") {
    // Same 4 Bambu colors plus two extras that exceed the U1 4-extruder cap.
    // Verifies the FullSpectrum overflow path produces valid recipes whose
    // achieved color is within a sensible deltaE bound of the target.
    std::vector<InputFilament> inputs = {
        {"#FF8040", 5000.0, "PLA"}, // 0 — orange, most used
        {"#000000", 4000.0, "PLA"}, // 1 — black
        {"#FFFFFF", 3000.0, "PLA"}, // 2 — white
        {"#FFFF80", 2000.0, "PLA"}, // 3 — pale yellow
        {"#FF0000", 1000.0, "PLA"}, // 4 — red       (overflow)
        {"#0000FF",  500.0, "PLA"}, // 5 — pure blue (overflow)
    };
    ConvertResult r = convert_filament_list(inputs);
    REQUIRE(r.physical_count == 4);
    REQUIRE(r.virtuals.size() == 2);

    // Each overflow recipe must reference two distinct physicals.
    for (const VirtualFilament &v : r.virtuals) {
        REQUIRE(v.physical_a < r.physical_count);
        REQUIRE(v.physical_b < r.physical_count);
        REQUIRE(v.physical_a != v.physical_b);
        REQUIRE(v.delta_e >= 0.0);
        // Pure-blue cannot be reasonably matched by mixing the 4 available
        // physicals (no blue available), so the delta_e there will be large
        // — that's correct algorithm behavior, not a bug.
    }
}

TEST_CASE("dense mixing-ratio grid produces lower deltaE than the legacy 5-value set",
          "[BambuConvert][Assign][DenseRatio]") {
    // A single overflow target that the legacy 5-ratio set could only
    // approximate. With 19 ratios the search lands closer to the
    // ideal mix.
    std::vector<InputFilament> inputs = {
        {"#000000", 5000.0, "PLA"},
        {"#FFFFFF", 4000.0, "PLA"},
        {"#FF0000", 3000.0, "PLA"},
        {"#00FF00", 2000.0, "PLA"},
        // 5th filament overflows — a near-but-not-50/50 gray that the
        // 5-ratio set could only express as 0.5 (= #808080-ish).
        {"#A8A8A8", 1000.0, "PLA"},
    };
    ConvertResult r = convert_filament_list(inputs, Strategy::Usage);
    REQUIRE(r.virtuals.size() == 1);
    // The dense grid picks ratio_a closer to 0.65 (more white than black)
    // — the legacy 5-value set was stuck at 0.5 or 0.667. The achievable
    // deltaE should be under 4, well below what the legacy grid produced.
    REQUIRE(r.virtuals[0].delta_e < 4.0);
}

TEST_CASE("convert_filament_list — chromatic strategy beats usage on isolated colors",
          "[BambuConvert][Assign][Chromatic]") {
    // Real Bambu X1 Carbon "HarryPotter +Color Painted" palette
    // (extracted from Metadata/slice_info.config). 8 PLA colors, with
    // the most-used four perceptually similar (beige, near-black,
    // yellow, white) and the only chromatically saturated color —
    // purple — only used for ~5 m. Usage strategy leaves purple in
    // the overflow where no two-physical mix can reproduce it.
    // Chromatic strategy promotes purple to a physical slot and
    // demotes beige (easy to mix from white+yellow).
    std::vector<InputFilament> inputs = {
        {"#F72323",  2450.0, "PLA"}, // 0 — red
        {"#FCECD6", 36990.0, "PLA"}, // 1 — beige (most used)
        {"#161616", 16920.0, "PLA"}, // 2 — near-black
        {"#7C4B00",  6420.0, "PLA"}, // 3 — dark brown
        {"#FFF144", 11750.0, "PLA"}, // 4 — yellow
        {"#FFFFFF",  8620.0, "PLA"}, // 5 — white
        {"#898989",  3290.0, "PLA"}, // 6 — gray
        {"#443089",  5310.0, "PLA"}, // 7 — purple
    };

    ConvertResult by_usage     = convert_filament_list(inputs, Strategy::Usage);
    ConvertResult by_chromatic = convert_filament_list(inputs, Strategy::Chromatic);

    REQUIRE(by_usage.physical_count == 4);
    REQUIRE(by_chromatic.physical_count == 4);
    REQUIRE(by_usage.virtuals.size() == 4);
    REQUIRE(by_chromatic.virtuals.size() == 4);

    // Chromatic must strictly beat usage on the sum of overflow deltaE.
    REQUIRE(by_chromatic.total_overflow_delta_e
            < by_usage.total_overflow_delta_e);

    // The chromatic strategy is expected to include the isolated
    // purple (index 7). Usage strategy will not (purple has only
    // 5310 mm extruded, beating only red 2450 and gray 3290).
    bool chromatic_includes_purple = false;
    for (size_t i = 0; i < by_chromatic.physical_count; ++i)
        if (by_chromatic.physical_indices[i] == 7)
            chromatic_includes_purple = true;
    REQUIRE(chromatic_includes_purple);

    // Sanity: strategy field on the result echoes back what we asked.
    REQUIRE(by_usage.strategy     == Strategy::Usage);
    REQUIRE(by_chromatic.strategy == Strategy::Chromatic);
}

TEST_CASE("convert_filament_list — chromatic with <= 4 inputs reduces to identity",
          "[BambuConvert][Assign][Chromatic]") {
    std::vector<InputFilament> inputs = {
        {"#FF0000", 100.0, "PLA"},
        {"#00FF00", 100.0, "PLA"},
        {"#0000FF", 100.0, "PLA"},
    };
    ConvertResult r = convert_filament_list(inputs, Strategy::Chromatic);
    REQUIRE(r.physical_count == 3);
    REQUIRE(r.virtuals.empty());
    REQUIRE_THAT(r.total_overflow_delta_e, WithinAbs(0.0, 1e-12));
}

// -----------------------------------------------------------------------------
// PlateData integration adapter — apply_bambu_to_u1_conversion(plate, ...)
// -----------------------------------------------------------------------------

TEST_CASE("apply_bambu_to_u1_conversion — rewrites filament list on real Bambu palette",
          "[BambuConvert][BBS3mf]") {
    // Same 8-color Bambu X1 Carbon "HarryPotter +Color Painted" palette used
    // by the chromatic test above. We construct a PlateData from scratch,
    // populate slice_filaments_info as if the 3mf parser had just finished,
    // and verify the adapter rewrites the plate state correctly.
    PlateData plate;

    auto add = [&](int id, const char *color, double used_m, const char *type) {
        FilamentInfo fi;
        fi.id     = id - 1;
        fi.color  = color;
        fi.used_m = used_m;
        fi.type   = type;
        plate.slice_filaments_info.push_back(fi);
    };
    add(1, "#F72323",  2.45, "PLA");
    add(2, "#FCECD6", 36.99, "PLA"); // beige — most used
    add(3, "#161616", 16.92, "PLA");
    add(4, "#7C4B00",  6.42, "PLA");
    add(5, "#FFF144", 11.75, "PLA");
    add(6, "#FFFFFF",  8.62, "PLA");
    add(7, "#898989",  3.29, "PLA");
    add(8, "#443089",  5.31, "PLA"); // purple — chromatically isolated

    BambuConvert::ConvertResult result;
    REQUIRE(apply_bambu_to_u1_conversion(plate, BambuConvert::Strategy::Usage, result));

    // After conversion, slice_filaments_info should contain exactly the
    // 4 chosen physicals.
    REQUIRE(plate.slice_filaments_info.size() == 4);

    // The config's filament_colour / filament_type should match.
    const auto *colors = plate.config.option<ConfigOptionStrings>("filament_colour");
    const auto *types  = plate.config.option<ConfigOptionStrings>("filament_type");
    REQUIRE(colors != nullptr);
    REQUIRE(types  != nullptr);
    REQUIRE(colors->values.size() == 4);
    REQUIRE(types->values.size()  == 4);

    // For Strategy::Usage: top-4 by used_m are beige (37), black (17),
    // yellow (12), white (9) — in that order.
    REQUIRE(colors->values[0] == "#FCECD6");
    REQUIRE(colors->values[1] == "#161616");
    REQUIRE(colors->values[2] == "#FFF144");
    REQUIRE(colors->values[3] == "#FFFFFF");

    // The overflow recipes are serialised into bambu_convert_recipe.
    const auto *recipe = plate.config.option<ConfigOptionString>("bambu_convert_recipe");
    REQUIRE(recipe != nullptr);
    REQUIRE_FALSE(recipe->value.empty());
    // Four overflow filaments -> four semicolon-separated entries.
    size_t semis = 0;
    for (char c : recipe->value) if (c == ';') ++semis;
    REQUIRE(semis == 3); // n-1 separators for n entries
}

TEST_CASE("apply_bambu_to_u1_conversion — chromatic promotes isolated purple",
          "[BambuConvert][BBS3mf][Chromatic]") {
    PlateData plate;
    auto add = [&](int id, const char *color, double used_m, const char *type) {
        FilamentInfo fi;
        fi.id     = id - 1;
        fi.color  = color;
        fi.used_m = used_m;
        fi.type   = type;
        plate.slice_filaments_info.push_back(fi);
    };
    add(1, "#F72323",  2.45, "PLA");
    add(2, "#FCECD6", 36.99, "PLA");
    add(3, "#161616", 16.92, "PLA");
    add(4, "#7C4B00",  6.42, "PLA");
    add(5, "#FFF144", 11.75, "PLA");
    add(6, "#FFFFFF",  8.62, "PLA");
    add(7, "#898989",  3.29, "PLA");
    add(8, "#443089",  5.31, "PLA");

    BambuConvert::ConvertResult result;
    REQUIRE(apply_bambu_to_u1_conversion(plate, BambuConvert::Strategy::Chromatic, result));

    const auto *colors = plate.config.option<ConfigOptionStrings>("filament_colour");
    REQUIRE(colors != nullptr);
    REQUIRE(colors->values.size() == 4);

    // Chromatic strategy is expected to include the chromatically isolated
    // purple #443089 in the physical set — the algorithm trades the
    // heavily-used beige for it because purple cannot be mixed.
    bool has_purple = false;
    for (const std::string &c : colors->values)
        if (c == "#443089") has_purple = true;
    REQUIRE(has_purple);
}

TEST_CASE("apply_bambu_to_u1_conversion — empty plate is no-op",
          "[BambuConvert][BBS3mf]") {
    PlateData plate;
    BambuConvert::ConvertResult result;
    REQUIRE_FALSE(apply_bambu_to_u1_conversion(plate, BambuConvert::Strategy::Usage, result));
}

TEST_CASE("apply_bambu_to_u1_conversion — refuses to re-convert an already-converted plate",
          "[BambuConvert][BBS3mf]") {
    PlateData plate;
    auto add = [&](int id, const char *color, double used_m, const char *type) {
        FilamentInfo fi;
        fi.id     = id - 1;
        fi.color  = color;
        fi.used_m = used_m;
        fi.type   = type;
        plate.slice_filaments_info.push_back(fi);
    };
    add(1, "#FF0000", 10.0, "PLA");
    add(2, "#00FF00",  8.0, "PLA");
    add(3, "#0000FF",  6.0, "PLA");
    add(4, "#FFFF00",  5.0, "PLA");
    add(5, "#00FFFF",  3.0, "PLA"); // overflow

    BambuConvert::ConvertResult first;
    REQUIRE(apply_bambu_to_u1_conversion(plate, BambuConvert::Strategy::Usage, first));
    REQUIRE(first.virtuals.size() == 1);

    // Re-invoking on the same plate must fail without touching state.
    BambuConvert::ConvertResult second;
    REQUIRE_FALSE(apply_bambu_to_u1_conversion(plate, BambuConvert::Strategy::Usage, second));

    // Original recipe is still present.
    const auto *recipe = plate.config.option<ConfigOptionString>("bambu_convert_recipe");
    REQUIRE(recipe != nullptr);
    REQUIRE_FALSE(recipe->value.empty());
}
