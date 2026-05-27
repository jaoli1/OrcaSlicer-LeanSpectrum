// Tests for the LeanSpectrum auto-profile generator.

#include <catch2/catch.hpp>

#include "libslic3r/AutoProfile.hpp"
#include "libslic3r/PrintConfig.hpp"

using namespace Slic3r;
using namespace Slic3r::AutoProfile;
using Catch::Matchers::WithinAbs;

namespace {

// A minimal DynamicPrintConfig pre-populated with the keys AutoProfile
// touches. Without this seed the option<>() lookups would return null
// and the apply() calls would silently no-op. The structure mirrors
// what FullPrintConfig provides in the real slicer.
DynamicPrintConfig make_seeded_config()
{
    DynamicPrintConfig c;
    c.set_key_value("layer_height",        new ConfigOptionFloat(0.20));
    c.set_key_value("wall_loops",          new ConfigOptionInt(2));
    c.set_key_value("top_shell_layers",    new ConfigOptionInt(4));
    c.set_key_value("bottom_shell_layers", new ConfigOptionInt(4));
    c.set_key_value("sparse_infill_density",
                    new ConfigOptionPercent(15.0));
    c.set_key_value("sparse_infill_pattern",
                    new ConfigOptionEnumGeneric{int(ipGyroid)});
    c.set_key_value("outer_wall_speed",    new ConfigOptionFloat(60.0));
    c.set_key_value("seam_slope_min_length", new ConfigOptionFloat(0.0));
    c.set_key_value("seam_slope_steps",    new ConfigOptionInt(0));
    c.set_key_value("fan_max_speed",       new ConfigOptionInt(100));
    c.set_key_value("fan_min_speed",       new ConfigOptionInt(50));

    // Per-filament options (one slot by default — the test fixture pretends
    // a single-material print, which is enough for the auto-profile tables).
    c.set_key_value("filament_max_volumetric_speed",
                    new ConfigOptionFloats{30.0});
    c.set_key_value("filament_retraction_length",
                    new ConfigOptionFloats{1.0});
    c.set_key_value("filament_retraction_speed",
                    new ConfigOptionFloats{40.0});

    // DIP / flush options — OrcaSlicer ships flush_into_infill=false by
    // default; AutoProfile should flip it on.
    c.set_key_value("flush_into_infill",  new ConfigOptionBool(false));
    c.set_key_value("flush_into_support", new ConfigOptionBool(false));

    std::vector<std::string> types{"PLA"};
    c.set_key_value("filament_type", new ConfigOptionStrings(types));
    return c;
}

} // namespace

TEST_CASE("polymer_from_type recognises common families", "[AutoProfile][Polymer]")
{
    REQUIRE(polymer_from_type("PLA")     == Polymer::PLA);
    REQUIRE(polymer_from_type("PLA+")    == Polymer::PLA);
    REQUIRE(polymer_from_type("PETG")    == Polymer::PETG);
    REQUIRE(polymer_from_type("PET-G")   == Polymer::PETG);
    REQUIRE(polymer_from_type("ABS")     == Polymer::ABS);
    REQUIRE(polymer_from_type("ASA")     == Polymer::ABS); // grouped with ABS
    REQUIRE(polymer_from_type("PC")      == Polymer::PC);
    REQUIRE(polymer_from_type("Nylon")   == Polymer::PA);
    REQUIRE(polymer_from_type("PA12")    == Polymer::PA);
    REQUIRE(polymer_from_type("TPU")     == Polymer::TPU);
    REQUIRE(polymer_from_type("HIPS")    == Polymer::HIPS);
    REQUIRE(polymer_from_type("PP")      == Polymer::PP);
    REQUIRE(polymer_from_type("")        == Polymer::Unknown);
    REQUIRE(polymer_from_type("XYZ-42")  == Polymer::Unknown);
}

TEST_CASE("Draft intent applies thick layer + single wall", "[AutoProfile][Intent]")
{
    DynamicPrintConfig c = make_seeded_config();
    auto notes = apply(c, Intent::Draft);

    REQUIRE_FALSE(notes.empty());
    REQUIRE_THAT(c.option<ConfigOptionFloat>("layer_height")->value,
                 WithinAbs(0.28, 1e-9));
    REQUIRE(c.option<ConfigOptionInt>("wall_loops")->value == 1);
    REQUIRE_THAT(c.option<ConfigOptionPercent>("sparse_infill_density")->value,
                 WithinAbs(8.0, 1e-9));
}

TEST_CASE("HighQuality intent applies small layer + scarf seam",
          "[AutoProfile][Intent]")
{
    DynamicPrintConfig c = make_seeded_config();
    apply(c, Intent::HighQuality);

    REQUIRE_THAT(c.option<ConfigOptionFloat>("layer_height")->value,
                 WithinAbs(0.12, 1e-9));
    REQUIRE(c.option<ConfigOptionInt>("wall_loops")->value == 3);
    // Scarf seam should be enabled — seam_slope_min_length non-zero.
    REQUIRE(c.option<ConfigOptionFloat>("seam_slope_min_length")->value > 0.0);
    REQUIRE(c.option<ConfigOptionInt>("seam_slope_steps")->value > 0);
}

TEST_CASE("Strength intent applies many walls + dense infill",
          "[AutoProfile][Intent]")
{
    DynamicPrintConfig c = make_seeded_config();
    apply(c, Intent::Strength);

    REQUIRE(c.option<ConfigOptionInt>("wall_loops")->value == 4);
    REQUIRE_THAT(c.option<ConfigOptionPercent>("sparse_infill_density")->value,
                 WithinAbs(35.0, 1e-9));
}

TEST_CASE("Decorative intent uses lightning + scarf", "[AutoProfile][Intent]")
{
    DynamicPrintConfig c = make_seeded_config();
    apply(c, Intent::Decorative);

    REQUIRE(c.option<ConfigOptionEnumGeneric>("sparse_infill_pattern")->value
            == int(ipLightning));
    REQUIRE_THAT(c.option<ConfigOptionPercent>("sparse_infill_density")->value,
                 WithinAbs(10.0, 1e-9));
    REQUIRE(c.option<ConfigOptionFloat>("seam_slope_min_length")->value > 0.0);
}

TEST_CASE("PLA polymer keeps full cooling fan", "[AutoProfile][Material]")
{
    DynamicPrintConfig c = make_seeded_config();
    apply(c, Intent::Standard, Polymer::PLA);
    REQUIRE(c.option<ConfigOptionInt>("fan_max_speed")->value == 100);
}

TEST_CASE("ABS polymer suppresses cooling fan", "[AutoProfile][Material]")
{
    DynamicPrintConfig c = make_seeded_config();
    apply(c, Intent::Standard, Polymer::ABS);
    REQUIRE(c.option<ConfigOptionInt>("fan_max_speed")->value <= 50);
    REQUIRE(c.option<ConfigOptionInt>("fan_min_speed")->value == 0);
}

TEST_CASE("TPU polymer slows everything down + disables scarf",
          "[AutoProfile][Material]")
{
    DynamicPrintConfig c = make_seeded_config();
    apply(c, Intent::HighQuality, Polymer::TPU);
    // HighQuality alone would set scarf on; TPU material refine must
    // override it back off (rubber doesn't ramp cleanly).
    REQUIRE(c.option<ConfigOptionFloat>("seam_slope_min_length")->value == 0.0);
    // TPU speed scale is 0.5 -> HQ outer_wall_speed 40 * 0.5 = 20.
    REQUIRE_THAT(c.option<ConfigOptionFloat>("outer_wall_speed")->value,
                 WithinAbs(20.0, 1e-9));
}

TEST_CASE("apply() with auto-detected polymer uses filament_type",
          "[AutoProfile][Material]")
{
    DynamicPrintConfig c = make_seeded_config();
    // Pre-seed with ABS so the auto-detection picks ABS, not PLA.
    c.option<ConfigOptionStrings>("filament_type")->values = {"ABS"};
    apply(c, Intent::Standard); // no explicit polymer
    REQUIRE(c.option<ConfigOptionInt>("fan_max_speed")->value <= 50);
}

TEST_CASE("max_volumetric_speed respects U1 ceiling (32 mm^3/s)",
          "[AutoProfile][U1]")
{
    DynamicPrintConfig c = make_seeded_config();
    // Draft on PLA is the most aggressive: intent 28 * polymer 1.0 = 28.
    apply(c, Intent::Draft, Polymer::PLA);
    REQUIRE_THAT(c.option<ConfigOptionFloats>("filament_max_volumetric_speed")->values.at(0),
                 WithinAbs(28.0, 1e-9));
    // High quality on PLA: intent 15 * polymer 1.0 = 15.
    apply(c, Intent::HighQuality, Polymer::PLA);
    REQUIRE_THAT(c.option<ConfigOptionFloats>("filament_max_volumetric_speed")->values.at(0),
                 WithinAbs(15.0, 1e-9));
    // TPU caps at intent 22 * polymer 0.4 = 8.8 even on Standard — TPU
    // physically can't push more than ~5 mm^3/s anyway.
    apply(c, Intent::Standard, Polymer::TPU);
    REQUIRE(c.option<ConfigOptionFloats>("filament_max_volumetric_speed")->values.at(0)
            < 10.0);
}

TEST_CASE("AutoProfile enables DIP flush-into-infill + support",
          "[AutoProfile][DIP]")
{
    DynamicPrintConfig c = make_seeded_config();
    apply(c, Intent::Standard, Polymer::PLA);
    REQUIRE(c.option<ConfigOptionBool>("flush_into_infill")->value);
    REQUIRE(c.option<ConfigOptionBool>("flush_into_support")->value);
}

TEST_CASE("retraction tuned for direct-drive on U1", "[AutoProfile][U1]")
{
    DynamicPrintConfig c = make_seeded_config();
    apply(c, Intent::Standard, Polymer::PLA);
    // PLA on U1 direct-drive: 0.8 mm @ 40 mm/s — well under generic 2-5 mm.
    REQUIRE_THAT(c.option<ConfigOptionFloats>("filament_retraction_length")->values.at(0),
                 WithinAbs(0.8, 1e-9));
    REQUIRE_THAT(c.option<ConfigOptionFloats>("filament_retraction_speed")->values.at(0),
                 WithinAbs(40.0, 1e-9));

    apply(c, Intent::Standard, Polymer::TPU);
    // TPU: zero retract (rubber stretches, retract just snaps the filament).
    REQUIRE_THAT(c.option<ConfigOptionFloats>("filament_retraction_length")->values.at(0),
                 WithinAbs(0.0, 1e-9));
}
