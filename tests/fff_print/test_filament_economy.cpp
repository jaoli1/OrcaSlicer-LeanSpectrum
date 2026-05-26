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
