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

// Per-intent overrides. Values calibrated against the Snapmaker U1
// official hardware envelope (see wiki.snapmaker.com/en/snapmaker_u1):
// max_volumetric_speed ceiling 32 mm^3/s on a 0.4 mm nozzle, max travel
// 500 mm/s, max accel 20 000 mm/s^2. The per-intent max_volumetric
// values pick a sensible point under that ceiling depending on how
// aggressively the user wants to print.
struct IntentOverrides {
    double layer_height;            // mm
    int    wall_loops;
    int    top_shell_layers;
    int    bottom_shell_layers;
    int    sparse_infill_density;   // percent
    InfillPattern sparse_infill_pattern;
    double outer_wall_speed;        // mm/s
    double max_volumetric_speed;    // mm^3/s   (PLA reference, scaled by polymer refine)
    bool   enable_scarf_seam;       // wires seam_slope_min_length etc.
};

IntentOverrides overrides_for(Intent intent)
{
    switch (intent) {
        // {layer, walls, top, bot, density, pattern,    outer, max_vol, scarf}
        case Intent::Draft:
            return {0.28, 1, 3, 3,  8, ipGyroid,    80.0, 28.0, false};
        case Intent::Standard:
            return {0.20, 2, 4, 4, 15, ipGyroid,    60.0, 22.0, false};
        case Intent::HighQuality:
            return {0.12, 3, 6, 5, 20, ipGyroid,    40.0, 15.0, true};
        case Intent::Strength:
            return {0.20, 4, 5, 5, 35, ipGyroid,    50.0, 20.0, false};
        case Intent::Decorative:
            return {0.16, 2, 5, 4, 10, ipLightning, 45.0, 18.0, true};
    }
    return {0.20, 2, 4, 4, 15, ipGyroid, 60.0, 22.0, false};
}

// Material-aware refinements applied on top of the intent overrides.
// Values cross-referenced with Snapmaker's wiki filament library and
// real-world community settings for the U1 direct-drive head. retract_*
// and max_vol_scale tighten the intent's nominal numbers per polymer.
struct MaterialRefine {
    int    fan_max_speed_pct;       // percent
    int    fan_min_speed_pct;
    bool   override_scarf_off;      // some materials don't ramp cleanly
    double speed_scale;             // multiplier on outer/inner/infill speeds
    double max_vol_scale;           // multiplier on intent's max_volumetric_speed
    double retract_length_mm;       // U1 direct-drive — keep low (0.5..3 mm range)
    int    retract_speed_mm_s;      // 30..70 mm/s per Snapmaker filament library
};

MaterialRefine refine_for(Polymer polymer)
{
    // PLA range on U1 hotend: nominally 230..250 C (stainless direct-drive).
    // Retract numbers are PLA-conservative (0.8 mm at 40 mm/s) on direct
    // drive; stringy materials (PETG, PA, PP) get longer retracts.
    switch (polymer) {
        // {fan_max, fan_min, scarf_off, speed, vol,  retract_mm, retract_mm_s}
        case Polymer::PLA:     return {100, 100, false, 1.00, 1.00, 0.8, 40};
        case Polymer::PETG:    return { 50,  30, false, 0.90, 0.70, 1.5, 40};
        case Polymer::ABS:     return { 30,   0, false, 0.90, 0.80, 1.0, 40};
        case Polymer::PC:      return { 30,   0, false, 0.85, 0.65, 1.0, 35};
        case Polymer::PA:      return { 40,  10, true,  0.80, 0.65, 2.0, 50};
        case Polymer::TPU:     return { 50,  30, true,  0.50, 0.40, 0.0, 30}; // no retract
        case Polymer::HIPS:    return { 60,  30, false, 0.90, 0.75, 1.0, 40};
        case Polymer::PP:      return { 80,  40, true,  0.80, 0.65, 2.0, 50};
        case Polymer::Unknown: return { 80,  40, false, 0.90, 0.75, 1.0, 40};
    }
    return {80, 40, false, 0.90, 0.75, 1.0, 40};
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

    // --- per-extruder max volumetric speed (Snapmaker U1 ceiling 32 mm^3/s).
    // The slicer reads this as a per-filament list (ConfigOptionFloats), one
    // value per loaded filament. We apply the same intent + polymer-scaled
    // value to every filament slot. Materials with low max_vol (e.g. TPU)
    // get a much tighter cap. The actual U1 hardware enforces 32 mm^3/s on
    // top, so this is purely a slicer-side hint.
    {
        const double mv = io.max_volumetric_speed * mr.max_vol_scale;
        if (auto *opt = config.option<ConfigOptionFloats>("filament_max_volumetric_speed")) {
            const size_t n = std::max<size_t>(1, opt->values.size());
            std::ostringstream ss;
            ss << "filament_max_volumetric_speed -> " << mv << " mm^3/s "
               << "(x" << n << " filaments)";
            notes.push_back(ss.str());
            opt->values.assign(n, mv);
        }
    }

    // --- retraction (U1 direct-drive — short retracts, mid speeds).
    // filament_retraction_length / filament_retraction_speed are
    // per-filament lists too.
    if (auto *opt = config.option<ConfigOptionFloats>("filament_retraction_length")) {
        const size_t n = std::max<size_t>(1, opt->values.size());
        std::ostringstream ss;
        ss << "filament_retraction_length -> " << mr.retract_length_mm << " mm";
        notes.push_back(ss.str());
        opt->values.assign(n, mr.retract_length_mm);
    }
    if (auto *opt = config.option<ConfigOptionFloats>("filament_retraction_speed")) {
        const size_t n = std::max<size_t>(1, opt->values.size());
        std::ostringstream ss;
        ss << "filament_retraction_speed -> " << mr.retract_speed_mm_s << " mm/s";
        notes.push_back(ss.str());
        opt->values.assign(n, double(mr.retract_speed_mm_s));
    }

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
