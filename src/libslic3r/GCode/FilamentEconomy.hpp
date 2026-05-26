#ifndef slic3r_GCode_FilamentEconomy_hpp_
#define slic3r_GCode_FilamentEconomy_hpp_

#include <string>
#include <vector>

#include "../libslic3r.h"
#include "../PrintConfig.hpp"

namespace Slic3r {

// Post-slicing filament economy optimizer for Snapmaker U1 with FullSpectrum
// mixed-color filaments. Operates on the produced G-code file in place.
//
// See doc/filament-economy/ARCHITECTURE.md and RESEARCH_SYNTHESIS.md for the
// full design. The module currently exposes five passes:
//
//   Pass 1 - remove no-op tool changes (T<n> targeting the active tool)
//   Pass 2 - shrink purge volumes based on extruder idle time   [stub]
//   Pass 3 - merge travel + retract pairs around kept swaps     [stub]
//   Pass 4 - curvature-aware adaptive E scaling per segment     [skeleton]
//   Pass 5 - physical correctness verification (M83 + mass + flow)
//
// Pass 5 runs first (gate). If it fails, the file is left untouched.
//
// References:
//   Al-Juboori, L. (2026). AI-assisted, curvature-aware post-slicing G-code
//   optimization for material-efficient FDM printing. JKSUES 38:35.
//   DOI: 10.1007/s44444-026-00109-y (CC-BY 4.0)
namespace FilamentEconomy {

// Per-material volumetric flow limits (mm^3/s). Used by Pass 5 (I5).
// Defaults match published Snapmaker SnapSpeed PLA spec and the paper's
// Nylon limit.
struct FlowLimits
{
    double pla   = 15.0;
    double petg  = 11.0;
    double abs   = 12.0;
    double nylon = 12.0;
    double tpu   = 5.0;
    // Safety factor applied to all of the above: actual cap = factor * value.
    double safety = 0.9;
};

// Settings used by the module. Built from a DynamicPrintConfig via from_config().
struct Settings
{
    // Master switch.
    bool enable                = true;

    // Pass 1
    bool   remove_noop_swaps      = true;

    // Pass 2 (stub)
    bool   shrink_purge           = true;
    int    shrink_purge_pct       = 30;     // 0..100

    // Pass 3 (stub)
    bool   merge_travel           = false;

    // Pass 4 - curvature-aware adaptive E scaling
    bool   curvature_lh           = true;
    double curvature_low_deg      = 10.0;
    double curvature_high_deg     = 45.0;
    int    curvature_max_pct      = 30;     // global cap; per-feature caps lower
    int    curvature_filter_window= 7;      // moving-median window length

    // Pass 5 - verification
    bool   force_m83              = true;
    double mass_tolerance_pct     = 1.0;
    FlowLimits flow_limits;

    static Settings from_config(const DynamicPrintConfig &config);
};

// Statistics reported back to the caller. All deltas are positive when they
// represent savings (filament saved, time saved).
struct Stats
{
    // Pass 1
    size_t swaps_removed       = 0;
    size_t retracts_removed    = 0;        // inside no-op wipe blocks
    double retract_volume_removed_mm = 0.0;
    // Pass 4
    size_t segments_scaled     = 0;
    double extrusion_saved_mm  = 0.0;
    // Pass 2 / 5
    size_t purges_shrunk       = 0;
    // Pass 3
    size_t lines_removed       = 0;
    // Pass 5 / general
    size_t time_saved_seconds  = 0;
    bool   converted_to_m83    = false;
    double max_flow_mm3s       = 0.0;       // observed peak after rewrite

    // Whether the file was modified at all.
    bool   modified            = false;
    // Pass 5 verdict: false means the file was rejected and downstream passes
    // were skipped. Stats may still contain pre-checks performed before the
    // rejection.
    bool   verification_ok     = true;

    // Human-readable diagnostics, one per detected issue.
    std::vector<std::string> notes;
};

// Process the G-code at gcode_path in place. Returns stats describing the
// savings. If settings.enable is false, returns an empty Stats and leaves the
// file untouched. On any failure of the verification pass (Pass 5), the file
// is left untouched and stats.verification_ok is set to false.
//
// Throws Slic3r::RuntimeError only on unrecoverable IO errors.
Stats process(const std::string &gcode_path, const Settings &settings);

// Convenience overload that derives Settings from a config object.
inline Stats process(const std::string &gcode_path, const DynamicPrintConfig &config)
{
    return process(gcode_path, Settings::from_config(config));
}

} // namespace FilamentEconomy
} // namespace Slic3r

#endif /* slic3r_GCode_FilamentEconomy_hpp_ */
