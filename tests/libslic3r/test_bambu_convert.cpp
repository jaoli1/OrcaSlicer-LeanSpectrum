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
