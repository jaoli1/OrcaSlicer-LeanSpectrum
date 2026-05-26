#ifndef slic3r_GCode_FilamentEconomy_hpp_
#define slic3r_GCode_FilamentEconomy_hpp_

#include <string>

#include "../libslic3r.h"
#include "../PrintConfig.hpp"

namespace Slic3r {

// Post-slicing filament economy optimizer for Snapmaker U1 with FullSpectrum
// mixed-color filaments. Operates on the produced G-code file in place.
//
// See doc/filament-economy/ARCHITECTURE.md for the full design.
//
// The module runs three optional passes:
//   1. Remove no-op tool changes (same physical extruder before and after).
//   2. Shrink purge volumes when a recent same-tool extrusion makes the full
//      purge unnecessary.
//   3. Merge redundant travel + retract sequences around kept swaps.
//
// All passes are no-ops on single-material prints.
namespace FilamentEconomy {

// Settings used by the module. Built from a DynamicPrintConfig via from_config().
struct Settings
{
    bool enable                = true;
    bool remove_noop_swaps     = true;
    bool shrink_purge          = true;
    int  shrink_purge_pct      = 30;   // 0..100
    bool merge_travel          = false;

    static Settings from_config(const DynamicPrintConfig &config);
};

// Statistics reported back to the caller. All values are deltas vs. the input
// G-code (positive = filament/time/lines saved).
struct Stats
{
    size_t lines_removed       = 0;
    size_t swaps_removed       = 0;
    double extrusion_saved_mm  = 0.0;
    double time_saved_seconds  = 0.0;
    size_t purges_shrunk       = 0;

    // Whether the file was modified at all.
    bool   modified            = false;
};

// Process the G-code at gcode_path in place. Returns stats describing the
// savings. If settings.enable is false, returns an empty Stats and leaves the
// file untouched.
//
// Throws Slic3r::RuntimeError on parse/IO errors.
Stats process(const std::string &gcode_path, const Settings &settings);

// Convenience overload that derives Settings from a config object.
inline Stats process(const std::string &gcode_path, const DynamicPrintConfig &config)
{
    return process(gcode_path, Settings::from_config(config));
}

} // namespace FilamentEconomy
} // namespace Slic3r

#endif /* slic3r_GCode_FilamentEconomy_hpp_ */
