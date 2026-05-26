#include "AutoProfile.hpp"

#include <algorithm>
#include <cctype>
#include <sstream>

namespace Slic3r {
namespace AutoProfile {

// ---------------------------------------------------------------------------
// Labels
// ---------------------------------------------------------------------------

const char *intent_label(Intent intent)
{
    switch (intent) {
        case Intent::Draft:       return "Draft / Fast";
        case Intent::Standard:    return "Standard / Balanced";
        case Intent::HighQuality: return "High quality / Detail";
        case Intent::Strength:    return "Strength / Functional";
        case Intent::Decorative:  return "Decorative / Display";
    }
    return "Standard / Balanced";
}

const char *intent_description(Intent intent)
{
    switch (intent) {
        case Intent::Draft:
            return "Fastest print. Big layer, single wall, low infill. "
                   "Use for prototyping and disposable parts.";
        case Intent::Standard:
            return "Balanced quality / time / filament for general-purpose "
                   "prints. The safe default.";
        case Intent::HighQuality:
            return "Small layers, smooth surfaces, scarf seam enabled. "
                   "Slower outer wall for clean overhangs.";
        case Intent::Strength:
            return "Many walls and dense gyroid infill for mechanical / "
                   "load-bearing parts.";
        case Intent::Decorative:
            return "Medium layers, low lightning infill, scarf seam. "
                   "Optimised for visual surfaces on display models.";
    }
    return "";
}

// ---------------------------------------------------------------------------
// Polymer detection
// ---------------------------------------------------------------------------

Polymer polymer_from_type(const std::string &filament_type)
{
    if (filament_type.empty())
        return Polymer::Unknown;
    std::string t;
    t.reserve(filament_type.size());
    for (char c : filament_type)
        t.push_back(static_cast<char>(std::toupper(static_cast<unsigned char>(c))));

    auto has = [&](const char *needle) {
        return t.find(needle) != std::string::npos;
    };

    if (has("TPU") || has("TPE") || has("FLEX"))  return Polymer::TPU;
    if (has("PETG") || has("PET-G") || has("PCTG")) return Polymer::PETG;
    if (has("HIPS"))                              return Polymer::HIPS;
    if (has("ASA") || has("ABS"))                 return Polymer::ABS;
    if (has("PC") || has("POLYCARBONATE"))        return Polymer::PC;
    if (has("PA12") || has("PA6") || has("NYLON") || has("PA-")) return Polymer::PA;
    if (has("PP"))                                return Polymer::PP;
    if (has("PLA"))                               return Polymer::PLA;
    return Polymer::Unknown;
}

// ---------------------------------------------------------------------------
// Intent overrides
// ---------------------------------------------------------------------------

namespace {

struct IntentOverrides {
    double layer_height;          // mm
    int    wall_loops;
    int    top_shell_layers;
    int    bottom_shell_layers;
    int    sparse_infill_density; // percent
    InfillPattern sparse_infill_pattern;
    double outer_wall_speed;      // mm/s
    bool   enable_scarf_seam;     // wires seam_slope_min_length etc.
};

IntentOverrides overrides_for(Intent intent)
{
    switch (intent) {
        case Intent::Draft:
            return {0.28, 1, 3, 3,  8,  ipGyroid,     80.0, false};
        case Intent::Standard:
            return {0.20, 2, 4, 4, 15,  ipGyroid,     60.0, false};
        case Intent::HighQuality:
            return {0.12, 3, 6, 5, 20,  ipGyroid,     40.0, true};
        case Intent::Strength:
            return {0.20, 4, 5, 5, 35,  ipGyroid,     50.0, false};
        case Intent::Decorative:
            return {0.16, 2, 5, 4, 10,  ipLightning,  45.0, true};
    }
    return {0.20, 2, 4, 4, 15, ipGyroid, 60.0, false};
}

// Material-aware refinements applied on top of the intent overrides.
// These leave the structural choices (layer height, walls, infill) from
// the intent intact and only tweak material-specific knobs.
struct MaterialRefine {
    int  fan_max_speed_pct;       // percent
    int  fan_min_speed_pct;
    bool enable_retract_lift;     // big retraction-Z helps stringy materials
    bool override_scarf_off;      // some materials don't ramp cleanly
    double speed_scale;           // multiplier on outer/inner/infill speeds
};

MaterialRefine refine_for(Polymer polymer)
{
    switch (polymer) {
        case Polymer::PLA:     return {100, 100, false, false, 1.0};
        case Polymer::PETG:    return { 50,  30, true,  false, 0.9};
        case Polymer::ABS:     return { 30,   0, false, false, 0.9};
        case Polymer::PC:      return { 30,   0, false, false, 0.85};
        case Polymer::PA:      return { 40,  10, true,  true,  0.8};
        case Polymer::TPU:     return { 50,  30, false, true,  0.5};
        case Polymer::HIPS:    return { 60,  30, false, false, 0.9};
        case Polymer::PP:      return { 80,  40, false, true,  0.8};
        case Polymer::Unknown: return { 80,  40, false, false, 0.9};
    }
    return {80, 40, false, false, 0.9};
}

const char *infill_pattern_name(InfillPattern p)
{
    switch (p) {
        case ipGyroid:    return "gyroid";
        case ipLightning: return "lightning";
        case ipCubic:     return "cubic";
        default:          return "(other)";
    }
}

void set_float(DynamicPrintConfig &c, const char *key, double v,
               std::vector<std::string> &notes, const char *unit = "")
{
    if (auto *opt = c.option<ConfigOptionFloat>(key)) {
        std::ostringstream ss;
        ss << key << " -> " << v << unit;
        notes.push_back(ss.str());
        opt->value = v;
    }
}

void set_int(DynamicPrintConfig &c, const char *key, int v,
             std::vector<std::string> &notes, const char *unit = "")
{
    if (auto *opt = c.option<ConfigOptionInt>(key)) {
        std::ostringstream ss;
        ss << key << " -> " << v << unit;
        notes.push_back(ss.str());
        opt->value = v;
    }
}

void set_percent(DynamicPrintConfig &c, const char *key, double v,
                 std::vector<std::string> &notes)
{
    if (auto *opt = c.option<ConfigOptionPercent>(key)) {
        std::ostringstream ss;
        ss << key << " -> " << v << " %";
        notes.push_back(ss.str());
        opt->value = v;
    }
}

void set_enum(DynamicPrintConfig &c, const char *key, int v,
              const char *display, std::vector<std::string> &notes)
{
    if (auto *opt = c.option<ConfigOptionEnumGeneric>(key)) {
        std::ostringstream ss;
        ss << key << " -> " << display;
        notes.push_back(ss.str());
        opt->value = v;
    }
}

} // namespace

// ---------------------------------------------------------------------------
// Apply
// ---------------------------------------------------------------------------

std::vector<std::string> apply(DynamicPrintConfig &config,
                               Intent              intent,
                               Polymer             polymer)
{
    std::vector<std::string> notes;
    const IntentOverrides   io = overrides_for(intent);
    const MaterialRefine    mr = refine_for(polymer);

    notes.push_back(std::string("Intent: ") + intent_label(intent));

    // --- structural settings from intent ---
    set_float (config, "layer_height",         io.layer_height,         notes, " mm");
    set_int   (config, "wall_loops",           io.wall_loops,           notes);
    set_int   (config, "top_shell_layers",     io.top_shell_layers,     notes);
    set_int   (config, "bottom_shell_layers",  io.bottom_shell_layers,  notes);
    set_percent(config, "sparse_infill_density", io.sparse_infill_density, notes);
    set_enum  (config, "sparse_infill_pattern", io.sparse_infill_pattern,
               infill_pattern_name(io.sparse_infill_pattern), notes);

    // --- speeds (intent baseline, scaled by material refine) ---
    const double outer_speed = io.outer_wall_speed * mr.speed_scale;
    set_float (config, "outer_wall_speed", outer_speed, notes, " mm/s");

    // --- scarf seam (gated by intent + material override-off) ---
    const bool scarf_on = io.enable_scarf_seam && !mr.override_scarf_off;
    if (scarf_on) {
        // The five scarf-related keys; setting the threshold to a finite
        // angle and giving the slope a non-trivial length is enough to
        // engage Orca's scarf path. Other knobs keep their profile defaults.
        if (auto *opt = config.option<ConfigOptionFloat>("seam_slope_min_length")) {
            opt->value = 10.0;
            notes.emplace_back("seam_slope_min_length -> 10 mm (scarf enabled)");
        }
        if (auto *opt = config.option<ConfigOptionInt>("seam_slope_steps")) {
            opt->value = 10;
            notes.emplace_back("seam_slope_steps -> 10");
        }
    } else {
        // Disable the scarf if a previous intent had enabled it.
        if (auto *opt = config.option<ConfigOptionFloat>("seam_slope_min_length")) {
            if (opt->value > 0.0) {
                opt->value = 0.0;
                notes.emplace_back("seam_slope_min_length -> 0 (scarf disabled)");
            }
        }
    }

    // --- material-aware cooling ---
    set_int (config, "fan_max_speed", mr.fan_max_speed_pct, notes, " %");
    set_int (config, "fan_min_speed", mr.fan_min_speed_pct, notes, " %");

    return notes;
}

std::vector<std::string> apply(DynamicPrintConfig &config, Intent intent)
{
    Polymer p = Polymer::Unknown;
    if (const auto *types = config.option<ConfigOptionStrings>("filament_type");
        types != nullptr && !types->values.empty())
        p = polymer_from_type(types->values.front());
    return apply(config, intent, p);
}

} // namespace AutoProfile
} // namespace Slic3r
