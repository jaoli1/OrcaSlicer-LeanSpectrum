#include <catch2/catch.hpp>

#include <fstream>
#include <sstream>
#include <string>
#include <vector>

#include <boost/filesystem.hpp>

#include "libslic3r/GCode/FilamentEconomy.hpp"

using namespace Slic3r;
using namespace Slic3r::FilamentEconomy;

namespace {

// Create a unique temp file in TEST_DATA_DIR's parent (the test binary's CWD),
// write the given content to it, and return its absolute path.
// The file is removed by the RAII guard returned alongside.
struct TempGcode
{
    boost::filesystem::path path;
    TempGcode(const std::string &content)
    {
        boost::filesystem::path dir   = boost::filesystem::temp_directory_path();
        boost::filesystem::path stem  = boost::filesystem::unique_path("leanspectrum-%%%%-%%%%.gcode");
        path                          = dir / stem;
        std::ofstream out(path.string(), std::ios::binary);
        out << content;
    }
    ~TempGcode()
    {
        boost::system::error_code ec;
        boost::filesystem::remove(path, ec);
    }
    std::string read() const
    {
        std::ifstream in(path.string(), std::ios::binary);
        std::stringstream ss;
        ss << in.rdbuf();
        return ss.str();
    }
};

Settings default_settings()
{
    Settings s;
    s.enable            = true;
    s.remove_noop_swaps = true;
    s.shrink_purge      = false; // Pass 2 not implemented yet
    s.merge_travel      = false; // Pass 3 not implemented yet
    s.curvature_lh      = false; // Pass 4 disabled to keep these unit tests focused on Pass 1
    s.force_m83         = false; // Pass 5 disabled (M82/M83 not relevant for these inputs)
    return s;
}

} // namespace

TEST_CASE("Empty G-code is unchanged", "[FilamentEconomy]")
{
    TempGcode gc("");
    Stats stats = process(gc.path.string(), default_settings());
    REQUIRE(stats.modified == false);
    REQUIRE(stats.swaps_removed == 0);
    REQUIRE(gc.read().empty());
}

TEST_CASE("Single-material print is a no-op", "[FilamentEconomy]")
{
    const std::string gcode =
        "; Single material print\n"
        "T0\n"
        "G1 X10 Y10 E5 F1000\n"
        "G1 X20 Y20 E5 F1000\n"
        "M104 S0\n";
    TempGcode gc(gcode);
    Stats stats = process(gc.path.string(), default_settings());
    REQUIRE(stats.modified == false);
    REQUIRE(stats.swaps_removed == 0);
    REQUIRE(gc.read() == gcode);
}

TEST_CASE("Alternating tool changes are kept", "[FilamentEconomy]")
{
    const std::string gcode =
        "T0\n"
        "G1 X1 Y1 E1\n"
        "T1\n"
        "G1 X2 Y2 E1\n"
        "T0\n"
        "G1 X3 Y3 E1\n";
    TempGcode gc(gcode);
    Stats stats = process(gc.path.string(), default_settings());
    REQUIRE(stats.swaps_removed == 0);
    // Every T<n> line should still be present.
    const std::string out = gc.read();
    REQUIRE_THAT(out, Catch::Matchers::Contains("T0"));
    REQUIRE_THAT(out, Catch::Matchers::Contains("T1"));
}

TEST_CASE("Adjacent same-tool changes are removed", "[FilamentEconomy]")
{
    // T1 appears twice in a row — the second is a no-op and should go.
    const std::string gcode =
        "T0\n"
        "G1 X1 Y1 E1\n"
        "T1\n"
        "G1 X2 Y2 E1\n"
        "T1\n"
        "G1 X3 Y3 E1\n";
    TempGcode gc(gcode);
    Stats stats = process(gc.path.string(), default_settings());
    REQUIRE(stats.swaps_removed == 1);
    REQUIRE(stats.modified == true);

    // The marker comment our Pass 1 leaves behind should be present.
    REQUIRE_THAT(gc.read(), Catch::Matchers::Contains("LeanSpectrum: removed no-op T1"));
}

TEST_CASE("Disabled module never modifies the file", "[FilamentEconomy]")
{
    const std::string gcode =
        "T0\n"
        "T0\n"   // would normally be removed by Pass 1
        "T0\n";
    TempGcode gc(gcode);
    Settings s = default_settings();
    s.enable   = false;
    Stats stats = process(gc.path.string(), s);
    REQUIRE(stats.modified == false);
    REQUIRE(stats.swaps_removed == 0);
    REQUIRE(gc.read() == gcode);
}

TEST_CASE("Pass 1 disabled flag respects setting", "[FilamentEconomy]")
{
    const std::string gcode =
        "T0\n"
        "T0\n"
        "T0\n";
    TempGcode gc(gcode);
    Settings s              = default_settings();
    s.remove_noop_swaps     = false;
    Stats stats             = process(gc.path.string(), s);
    REQUIRE(stats.swaps_removed == 0);
    REQUIRE(stats.modified == false);
}

TEST_CASE("Pass 5: M82 absolute G-code is converted to M83 relative", "[FilamentEconomy]")
{
    // Absolute-extrusion file with two short deposition moves. After
    // conversion the cumulative-E values should become per-line deltas.
    const std::string gcode =
        ";TYPE:Solid infill\n"
        "M82\n"
        "G92 E0\n"
        "G1 X10 Y0 E1.000 F1200\n"
        "G1 X20 Y0 E2.000 F1200\n"
        "G1 X20 Y10 E3.000 F1200\n";
    TempGcode gc(gcode);
    Settings  s = default_settings();
    s.force_m83 = true;
    Stats stats = process(gc.path.string(), s);
    REQUIRE(stats.converted_to_m83 == true);
    const std::string out = gc.read();
    REQUIRE_THAT(out, Catch::Matchers::Contains("M83"));
    // After conversion the second deposition should carry a delta of 1.0,
    // not the cumulative 2.0 from the absolute form.
    REQUIRE_THAT(out, Catch::Matchers::Contains("E1.00000"));
}

TEST_CASE("Pass 4: straight line is reduced, sharp corner is preserved", "[FilamentEconomy]")
{
    // A short toolpath: long straight run, then a 90 degree corner, then a
    // short stretch. Pass 4 should leave the corner segment at full E and
    // reduce the straight-run segments.
    const std::string gcode =
        ";TYPE:Sparse infill\n"
        "M83\n"
        "G1 X10 Y0   E1.000 F1200\n"
        "G1 X20 Y0   E1.000 F1200\n"
        "G1 X30 Y0   E1.000 F1200\n"
        "G1 X30 Y10  E1.000 F1200\n"
        "G1 X30 Y20  E1.000 F1200\n";
    TempGcode gc(gcode);
    Settings  s = default_settings();
    s.curvature_lh                  = true;
    s.curvature_low_deg             = 10.0;
    s.curvature_high_deg            = 45.0;
    s.curvature_max_pct             = 30;
    s.curvature_filter_window       = 1; // disable smoothing for this assertion
    s.force_m83                     = false;
    Stats stats = process(gc.path.string(), s);
    REQUIRE(stats.segments_scaled > 0);
    REQUIRE(stats.extrusion_saved_mm > 0.0);
    REQUIRE(stats.modified == true);
}

TEST_CASE("Pass 4: bridge segments are never reduced", "[FilamentEconomy]")
{
    const std::string gcode =
        ";TYPE:Bridge\n"
        "M83\n"
        "G1 X10 Y0 E1.000 F1200\n"
        "G1 X20 Y0 E1.000 F1200\n"
        "G1 X30 Y0 E1.000 F1200\n";
    TempGcode gc(gcode);
    Settings  s = default_settings();
    s.curvature_lh           = true;
    s.curvature_filter_window= 1;
    s.force_m83              = false;
    Stats stats = process(gc.path.string(), s);
    // Bridge cap is 0 — even though these segments are perfectly straight,
    // no extrusion should be reduced.
    REQUIRE(stats.segments_scaled == 0);
    REQUIRE(stats.extrusion_saved_mm == 0.0);
}

TEST_CASE("Pass 2: shrinks the purge inside a CP TOOLCHANGE block", "[FilamentEconomy]")
{
    // Build a synthetic file with: T0 extrusion, then a TOOLCHANGE block
    // targeting T1 a few seconds later (recent idle -> max shrink), then
    // back to T0. The purge inside the block has 4 positive-E moves we
    // expect to be scaled by (1 - 30%) = 0.7 each.
    const std::string gcode =
        "M83\n"
        "T0\n"
        "G1 X10 Y0  E1.000 F1200\n"   // T0 extrudes
        "G1 X20 Y0  E1.000 F1200\n"
        "; CP TOOLCHANGE START\n"
        "T1\n"
        "G1 E-2 F1800\n"               // retract — should NOT be touched
        "G1 X40 Y10 E2.000 F1500\n"    // purge starts here
        "G1 X50 Y10 E2.000 F1500\n"
        "G1 X60 Y10 E2.000 F1500\n"
        "G1 X70 Y10 E2.000 F1500\n"
        "G1 E2 F1800\n"                // unretract — no XY move, kept
        "; CP TOOLCHANGE END\n"
        "G1 X80 Y20 E1.000 F1200\n";   // back to printing
    TempGcode gc(gcode);
    Settings  s = default_settings();
    s.shrink_purge      = true;
    s.shrink_purge_pct  = 30;
    s.force_m83         = false; // already M83 in fixture
    s.curvature_lh      = false; // isolate Pass 2 in this test
    s.remove_noop_swaps = false; // T0->T1 is a real swap, not a no-op
    Stats stats = process(gc.path.string(), s);

    REQUIRE(stats.purges_shrunk == 1);
    // 4 purge segments * 2.0 * 0.3 = 2.4 mm saved (recent idle, max shrink).
    REQUIRE(stats.extrusion_saved_mm > 2.0);
    REQUIRE(stats.extrusion_saved_mm < 2.5);
    REQUIRE(stats.modified == true);

    // Retract / unretract values should still be present verbatim.
    const std::string out = gc.read();
    REQUIRE_THAT(out, Catch::Matchers::Contains("E-2"));
}

TEST_CASE("Pass 2: skipped when shrink_purge disabled", "[FilamentEconomy]")
{
    const std::string gcode =
        "M83\n"
        "; CP TOOLCHANGE START\n"
        "T1\n"
        "G1 X10 Y0 E2 F1200\n"
        "; CP TOOLCHANGE END\n";
    TempGcode gc(gcode);
    Settings  s = default_settings();
    s.shrink_purge = false;
    Stats stats = process(gc.path.string(), s);
    REQUIRE(stats.purges_shrunk == 0);
}

TEST_CASE("Pass 2: 0 percent is a no-op", "[FilamentEconomy]")
{
    const std::string gcode =
        "M83\n"
        "; CP TOOLCHANGE START\n"
        "T1\n"
        "G1 X10 Y0 E2 F1200\n"
        "; CP TOOLCHANGE END\n";
    TempGcode gc(gcode);
    Settings  s = default_settings();
    s.shrink_purge     = true;
    s.shrink_purge_pct = 0;
    Stats stats = process(gc.path.string(), s);
    REQUIRE(stats.purges_shrunk == 0);
    REQUIRE(stats.extrusion_saved_mm == 0.0);
}

TEST_CASE("Pass 3: back-to-back retract+unretract is collapsed", "[FilamentEconomy]")
{
    // No XY motion between the two — they cancel out exactly.
    const std::string gcode =
        "M83\n"
        "G1 X10 Y0 E1 F1200\n"
        "G1 E-2 F1800\n"     // retract
        "G1 E2 F1800\n"      // un-retract immediately after, no XY
        "G1 X20 Y0 E1 F1200\n";
    TempGcode gc(gcode);
    Settings  s = default_settings();
    s.merge_travel      = true;
    s.remove_noop_swaps = false;
    s.curvature_lh      = false;
    s.shrink_purge      = false;
    Stats stats = process(gc.path.string(), s);
    REQUIRE(stats.lines_removed == 2);
    REQUIRE(stats.modified == true);
    REQUIRE_THAT(gc.read(),
                 Catch::Matchers::Contains("LeanSpectrum: collapsed redundant retract"));
}

TEST_CASE("Pass 3: retract+travel+unretract is preserved", "[FilamentEconomy]")
{
    // A travel move sits between the retract and the un-retract — this is
    // a legitimate avoid-string pattern and must not be collapsed.
    const std::string gcode =
        "M83\n"
        "G1 X10 Y0 E1 F1200\n"
        "G1 E-2 F1800\n"
        "G1 X40 Y40 F9000\n"  // travel — preserves the retract's purpose
        "G1 E2 F1800\n"
        "G1 X50 Y40 E0.5 F1200\n";
    TempGcode gc(gcode);
    Settings  s = default_settings();
    s.merge_travel = true;
    s.remove_noop_swaps = false;
    s.curvature_lh = false;
    s.shrink_purge = false;
    Stats stats = process(gc.path.string(), s);
    REQUIRE(stats.lines_removed == 0);
}

TEST_CASE("Pass 3: disabled flag respects setting", "[FilamentEconomy]")
{
    const std::string gcode =
        "M83\n"
        "G1 E-2 F1800\n"
        "G1 E2 F1800\n";
    TempGcode gc(gcode);
    Settings  s = default_settings();
    s.merge_travel = false;
    Stats stats = process(gc.path.string(), s);
    REQUIRE(stats.lines_removed == 0);
}
