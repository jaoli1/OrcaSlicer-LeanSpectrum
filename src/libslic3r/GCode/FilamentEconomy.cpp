#include "FilamentEconomy.hpp"

#include <algorithm>
#include <array>
#include <cctype>
#include <cmath>
#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <fstream>
#include <limits>
#include <regex>
#include <sstream>
#include <string>
#include <string_view>
#include <vector>

#include <boost/log/trivial.hpp>

#include "../PrintConfig.hpp"

#ifndef M_PI
#define M_PI 3.14159265358979323846
#endif

namespace Slic3r {
namespace FilamentEconomy {

// ---------------------------------------------------------------------------
// Settings::from_config
// ---------------------------------------------------------------------------

Settings Settings::from_config(const DynamicPrintConfig &config)
{
    Settings s;

    auto get_bool = [&](const char *key, bool &out) {
        if (const ConfigOptionBool *opt = config.option<ConfigOptionBool>(key))
            out = opt->value;
    };
    auto get_int = [&](const char *key, int &out) {
        if (const ConfigOptionInt *opt = config.option<ConfigOptionInt>(key))
            out = opt->value;
    };
    auto get_float = [&](const char *key, double &out) {
        if (const ConfigOptionFloat *opt = config.option<ConfigOptionFloat>(key))
            out = opt->value;
    };

    get_bool ("filament_economy_enable",            s.enable);
    get_bool ("filament_economy_remove_noop_swaps", s.remove_noop_swaps);
    get_bool ("filament_economy_shrink_purge",      s.shrink_purge);
    get_int  ("filament_economy_shrink_purge_pct",  s.shrink_purge_pct);
    get_bool ("filament_economy_merge_travel",      s.merge_travel);

    get_bool ("filament_economy_curvature_lh",            s.curvature_lh);
    get_float("filament_economy_curvature_low_deg",       s.curvature_low_deg);
    get_float("filament_economy_curvature_high_deg",      s.curvature_high_deg);
    get_int  ("filament_economy_curvature_max_pct",       s.curvature_max_pct);
    get_int  ("filament_economy_curvature_filter_window", s.curvature_filter_window);

    get_bool ("filament_economy_force_m83",          s.force_m83);
    get_float("filament_economy_mass_tolerance_pct", s.mass_tolerance_pct);

    s.shrink_purge_pct  = std::clamp(s.shrink_purge_pct,  0, 100);
    s.curvature_max_pct = std::clamp(s.curvature_max_pct, 0, 100);
    if (s.curvature_filter_window < 1)
        s.curvature_filter_window = 1;

    return s;
}

// ---------------------------------------------------------------------------
// Internal types and helpers
// ---------------------------------------------------------------------------

namespace {

enum class FeatureType : uint8_t {
    Unknown = 0,
    OuterWall,
    InnerWall,
    SolidInfill,
    SparseInfill,
    TopSurface,
    BottomSurface,
    Bridge,
    Support,
    WipeTower,
    Custom,
    Travel,
};

// Forward declarations for helpers defined later in this anonymous namespace
// but used by passes that appear earlier (Pass 1's wipe-tower cleanup needs
// these — they were originally added for Pass 2).
bool line_starts_toolchange_block(const std::string &raw);
bool line_ends_toolchange_block(const std::string &raw);

double feature_cap(FeatureType f)
{
    switch (f) {
        case FeatureType::OuterWall:     return 0.15;
        case FeatureType::InnerWall:     return 0.20;
        case FeatureType::SolidInfill:   return 0.25;
        case FeatureType::TopSurface:    return 0.15;
        case FeatureType::BottomSurface: return 0.15;
        case FeatureType::SparseInfill:  return 0.35;
        case FeatureType::Bridge:        return 0.0;
        case FeatureType::WipeTower:     return 0.0;
        case FeatureType::Support:       return 0.30;
        case FeatureType::Custom:        return 0.0;
        default:                         return 0.0;
    }
}

// A single parsed G-code line in semantic form.
struct Line
{
    std::string raw;
    bool   is_g1            = false;
    bool   has_x = false, has_y = false, has_z = false;
    bool   has_e = false, has_f = false;
    double x = 0, y = 0, z = 0;
    double e = 0;           // value as written (mode-dependent)
    double f = 0;           // feedrate (mm/min)
    bool   is_tool_change   = false;
    int    tool             = -1;
    bool   is_retract       = false;
    bool   is_unretract     = false;
    bool   is_extrusion     = false;
    bool   is_travel        = false;
    bool   is_layer_change  = false;
    double delta_x = 0, delta_y = 0;
    double length_xy = 0;
    double curvature_rad = 0;
    double e_ratio = 1.0;
    FeatureType feature = FeatureType::Unknown;
};

struct Parsed
{
    std::vector<Line> lines;
    bool   absolute_e         = false;
    bool   m83_present        = false;
    size_t retract_count_in   = 0;
    double retract_volume_in  = 0.0;
    double total_extrusion_in = 0.0;
    double layer_height_modal = 0.2;
    double extrusion_width_modal = 0.4;
};

bool read_file(const std::string &path, std::vector<std::string> &out_lines)
{
    std::ifstream in(path, std::ios::binary);
    if (!in)
        return false;
    out_lines.clear();
    std::string line;
    while (std::getline(in, line)) {
        if (!line.empty() && line.back() == '\r')
            line.pop_back();
        out_lines.emplace_back(std::move(line));
        line.clear();
    }
    return true;
}

bool write_file(const std::string &path, const std::vector<std::string> &lines)
{
    std::ofstream out(path, std::ios::binary | std::ios::trunc);
    if (!out)
        return false;
    for (const std::string &l : lines) {
        out.write(l.data(), static_cast<std::streamsize>(l.size()));
        out.put('\n');
    }
    return static_cast<bool>(out);
}

bool starts_with(const std::string &s, const char *prefix)
{
    const size_t n = std::char_traits<char>::length(prefix);
    return s.size() >= n && s.compare(0, n, prefix) == 0;
}

bool parse_axis(const std::string &raw, size_t &pos, char letter, double &value)
{
    if (pos >= raw.size() || std::toupper(static_cast<unsigned char>(raw[pos])) != letter)
        return false;
    char *parse_end = nullptr;
    value = std::strtod(raw.c_str() + pos + 1, &parse_end);
    if (parse_end == raw.c_str() + pos + 1)
        return false;
    pos = static_cast<size_t>(parse_end - raw.c_str());
    return true;
}

void parse_line(Line &out, FeatureType &current_feature)
{
    out.is_g1 = false;
    out.has_x = out.has_y = out.has_z = out.has_e = out.has_f = false;
    out.is_tool_change = out.is_retract = out.is_unretract = false;
    out.is_extrusion = out.is_travel = out.is_layer_change = false;
    out.tool = -1;
    out.feature = current_feature;

    const std::string &raw = out.raw;
    if (raw.empty())
        return;

    // Comments — track feature transitions from upstream slicer markers.
    if (raw[0] == ';') {
        if (raw.find(";TYPE:") != std::string::npos) {
            if      (raw.find(";TYPE:Outer wall")     != std::string::npos) current_feature = FeatureType::OuterWall;
            else if (raw.find(";TYPE:Inner wall")     != std::string::npos) current_feature = FeatureType::InnerWall;
            else if (raw.find(";TYPE:Solid infill")   != std::string::npos) current_feature = FeatureType::SolidInfill;
            else if (raw.find(";TYPE:Sparse infill")  != std::string::npos) current_feature = FeatureType::SparseInfill;
            else if (raw.find(";TYPE:Top surface")    != std::string::npos) current_feature = FeatureType::TopSurface;
            else if (raw.find(";TYPE:Bottom surface") != std::string::npos) current_feature = FeatureType::BottomSurface;
            else if (raw.find(";TYPE:Bridge")         != std::string::npos) current_feature = FeatureType::Bridge;
            else if (raw.find(";TYPE:Support")        != std::string::npos) current_feature = FeatureType::Support;
            else if (raw.find(";TYPE:Wipe tower")     != std::string::npos) current_feature = FeatureType::WipeTower;
            else if (raw.find(";TYPE:Custom")         != std::string::npos) current_feature = FeatureType::Custom;
        }
        if (raw.find("CP TOOLCHANGE START") != std::string::npos ||
            raw.find("WIPE_START")          != std::string::npos)
            current_feature = FeatureType::WipeTower;
        if (raw.find(";LAYER_CHANGE") != std::string::npos ||
            raw.find(";LAYER:")       != std::string::npos)
            out.is_layer_change = true;
        out.feature = current_feature;
        return;
    }

    // Tool change.
    if ((raw[0] == 'T' || raw[0] == 't') && raw.size() > 1 &&
        std::isdigit(static_cast<unsigned char>(raw[1]))) {
        out.is_tool_change = true;
        out.tool           = std::atoi(raw.c_str() + 1);
        out.feature        = current_feature;
        return;
    }

    // G1 motion line.
    if (raw.size() >= 2 && (raw[0] == 'G' || raw[0] == 'g') && raw[1] == '1' &&
        (raw.size() == 2 || raw[2] == ' ' || raw[2] == '\t')) {
        out.is_g1 = true;
        size_t pos = 2;
        while (pos < raw.size()) {
            while (pos < raw.size() && (raw[pos] == ' ' || raw[pos] == '\t'))
                ++pos;
            if (pos >= raw.size() || raw[pos] == ';')
                break;
            char c   = static_cast<char>(std::toupper(static_cast<unsigned char>(raw[pos])));
            double v = 0.0;
            size_t before = pos;
            if (parse_axis(raw, pos, c, v)) {
                switch (c) {
                    case 'X': out.x = v; out.has_x = true; break;
                    case 'Y': out.y = v; out.has_y = true; break;
                    case 'Z': out.z = v; out.has_z = true; break;
                    case 'E': out.e = v; out.has_e = true; break;
                    case 'F': out.f = v; out.has_f = true; break;
                    default: break;
                }
            } else if (pos == before) {
                ++pos; // avoid infinite loop on malformed input
            }
        }
        out.feature = current_feature;
        return;
    }
}

// Walk the lines, fill modal X/Y/Z and classify each G1 as travel/retract/
// unretract/extrusion. Detects M82/M83. Updates absolute-E counters.
void reconstruct_modal(Parsed &p)
{
    bool   abs_mode  = false;
    bool   any_m82   = false;
    bool   any_m83   = false;
    double mx = 0, my = 0, mz = 0;
    double abs_e    = 0;

    p.retract_count_in   = 0;
    p.retract_volume_in  = 0.0;
    p.total_extrusion_in = 0.0;

    for (Line &l : p.lines) {
        const std::string &r = l.raw;
        if (starts_with(r, "M82")) { abs_mode = true;  any_m82 = true; continue; }
        if (starts_with(r, "M83")) { abs_mode = false; any_m83 = true; continue; }
        if (starts_with(r, "G92")) {
            if (r.find('E') != std::string::npos || r.find('e') != std::string::npos)
                abs_e = 0;
            continue;
        }
        if (!l.is_g1)
            continue;

        const double prev_x = mx;
        const double prev_y = my;
        if (l.has_x) mx = l.x;
        if (l.has_y) my = l.y;
        if (l.has_z) mz = l.z;
        l.delta_x   = mx - prev_x;
        l.delta_y   = my - prev_y;
        l.length_xy = std::hypot(l.delta_x, l.delta_y);

        double e_inc = 0.0;
        if (l.has_e) {
            if (abs_mode) {
                e_inc = l.e - abs_e;
                abs_e = l.e;
            } else {
                e_inc = l.e;
            }
        }
        if (e_inc > 0.0) {
            if (l.length_xy < 1e-6) {
                l.is_unretract = true;
            } else {
                l.is_extrusion = true;
                p.total_extrusion_in += e_inc;
            }
        } else if (e_inc < 0.0) {
            l.is_retract = true;
            ++p.retract_count_in;
            p.retract_volume_in += -e_inc;
        } else if (l.length_xy > 1e-6) {
            l.is_travel = true;
        }
        l.z = mz;
    }

    p.absolute_e  = any_m82 && !any_m83;
    p.m83_present = any_m83;
}

// Replace the value of the first 'E' or 'e' token in raw with new_value.
void rewrite_e_token(std::string &raw, double new_value)
{
    size_t pe = raw.find('E');
    if (pe == std::string::npos)
        pe = raw.find('e');
    if (pe == std::string::npos)
        return;
    size_t end = pe + 1;
    while (end < raw.size() && (std::isdigit(static_cast<unsigned char>(raw[end])) ||
                                 raw[end] == '.' || raw[end] == '-' || raw[end] == '+'))
        ++end;
    char buf[64];
    std::snprintf(buf, sizeof(buf), "%.5f", new_value);
    raw.replace(pe + 1, end - (pe + 1), buf);
}

// Convert the entire file to relative (M83) extrusion.
bool convert_to_m83(Parsed &p)
{
    bool   modified = false;
    bool   abs_mode = false;
    double abs_e    = 0.0;

    for (Line &l : p.lines) {
        std::string &r = l.raw;
        if (starts_with(r, "M82")) {
            r        = "M83 ; LeanSpectrum: converted from M82";
            abs_mode = true;
            modified = true;
            continue;
        }
        if (starts_with(r, "M83")) {
            abs_mode = false;
            continue;
        }
        if (starts_with(r, "G92") && (r.find('E') != std::string::npos || r.find('e') != std::string::npos)) {
            abs_e = 0.0;
            continue;
        }
        if (!l.is_g1 || !l.has_e || !abs_mode)
            continue;
        const double delta = l.e - abs_e;
        abs_e              = l.e;
        rewrite_e_token(r, delta);
        l.e       = delta;
        modified  = true;
    }
    return modified;
}

double material_flow_limit(const Settings &s)
{
    // Conservative: smallest of common materials. A future iteration takes
    // the active filament_type into account.
    const double lo = std::min({s.flow_limits.pla, s.flow_limits.petg,
                                s.flow_limits.abs, s.flow_limits.nylon});
    return s.flow_limits.safety * lo;
}

double compute_max_flow(const Parsed &p)
{
    double q_max = 0.0;
    for (const Line &l : p.lines) {
        if (!l.is_extrusion || !l.has_f)
            continue;
        const double q = p.layer_height_modal * p.extrusion_width_modal *
                         (l.f / 60.0);
        q_max = std::max(q_max, q);
    }
    return q_max;
}

void verification_pass(Parsed &p, Stats &stats, const Settings &s)
{
    // I1 — M83 enforcement.
    if (p.absolute_e && s.force_m83) {
        if (convert_to_m83(p)) {
            stats.converted_to_m83 = true;
            stats.notes.emplace_back("Converted G-code from M82 absolute to M83 relative extrusion.");
            reconstruct_modal(p);
        }
    }

    // I5 — volumetric flow.
    const double q_max = compute_max_flow(p);
    stats.max_flow_mm3s = q_max;
    const double q_cap = material_flow_limit(s);
    if (q_max > q_cap) {
        char buf[160];
        std::snprintf(buf, sizeof(buf),
                      "Volumetric flow %.2f mm^3/s exceeds %.2f mm^3/s safety cap; "
                      "aggressive E-scaling passes will be skipped.",
                      q_max, q_cap);
        stats.notes.emplace_back(buf);
        stats.verification_ok = false;
    }
}

// Pass 1 — remove no-op tool changes AND their orphan wipe-tower block.
//
// A "no-op swap" is a `T<n>` that re-selects the currently active extruder.
// Just commenting out the T<n> line leaves the surrounding wipe-tower
// segment in place — the head still purges plastic, undoing the whole
// point of the optimisation. The refined version walks outward from the
// removed T to find the enclosing `CP TOOLCHANGE START..END` block (or
// `WIPE_TOWER_START..END` equivalent) and comments out every line in it.
//
// Tracks two stats:
//   stats.swaps_removed   — how many T<n> lines we neutralised
//   stats.extrusion_saved_mm — total positive-E in the removed wipe blocks
size_t pass_noop_swaps(Parsed &p, Stats &stats)
{
    auto comment_out = [](Line &l, const char *tag) {
        // Preserve original text in the marker for debugging.
        std::string trimmed = l.raw;
        if (trimmed.size() > 80) trimmed.resize(80);
        l.raw = std::string("; LeanSpectrum: removed ") + tag + " (" + trimmed + ")";
        l.is_tool_change = false;
        l.is_retract     = false;
        l.is_unretract   = false;
        l.is_extrusion   = false;
        l.is_travel      = false;
    };

    size_t swap_count = 0;
    double saved_mm   = 0.0;
    int    current    = -1;
    for (size_t i = 0; i < p.lines.size(); ++i) {
        Line &l = p.lines[i];
        if (!l.is_tool_change)
            continue;
        if (current == -1) {
            current = l.tool;
            continue;
        }
        if (l.tool != current) {
            current = l.tool;
            continue;
        }

        // Found a no-op. Look outward for the enclosing wipe-tower block.
        // Search backward up to 80 lines for a START marker (typical
        // wipe-tower epilogue prologue is a few dozen lines).
        size_t block_start = i;
        size_t back_limit  = (i > 80) ? (i - 80) : 0;
        for (size_t b = i; b-- > back_limit; ) {
            if (line_starts_toolchange_block(p.lines[b].raw)) {
                block_start = b;
                break;
            }
        }
        // Forward to END.
        size_t block_end   = i;
        size_t fwd_limit   = std::min(p.lines.size(), i + 200);
        for (size_t f = i + 1; f < fwd_limit; ++f) {
            if (line_ends_toolchange_block(p.lines[f].raw)) {
                block_end = f;
                break;
            }
        }

        if (block_start < i && block_end > i) {
            // Sum positive E inside the block before neutralising.
            for (size_t k = block_start + 1; k < block_end; ++k) {
                const Line &lk = p.lines[k];
                if (lk.is_g1 && lk.has_e && lk.e > 0.0 && lk.is_extrusion)
                    saved_mm += lk.e;
            }
            for (size_t k = block_start; k <= block_end; ++k)
                comment_out(p.lines[k], "no-op wipe block");
        } else {
            // No surrounding block found — just neutralise the T<n> line.
            comment_out(l, "no-op tool change");
        }
        ++swap_count;
    }
    stats.swaps_removed      += swap_count;
    stats.extrusion_saved_mm += saved_mm;
    return swap_count;
}

double angle_between(double ax, double ay, double bx, double by)
{
    const double na = std::hypot(ax, ay);
    const double nb = std::hypot(bx, by);
    if (na < 1e-9 || nb < 1e-9)
        return 0.0;
    double c = (ax * bx + ay * by) / (na * nb);
    c = std::clamp(c, -1.0, 1.0);
    return std::acos(c);
}

void median_filter(std::vector<double> &v, int window)
{
    if (window <= 1 || v.size() < static_cast<size_t>(window))
        return;
    const int half = window / 2;
    std::vector<double> out(v.size());
    std::vector<double> w; w.reserve(static_cast<size_t>(window));
    for (size_t i = 0; i < v.size(); ++i) {
        w.clear();
        for (int k = -half; k <= half; ++k) {
            const int j = static_cast<int>(i) + k;
            if (j < 0 || j >= static_cast<int>(v.size()))
                continue;
            w.push_back(v[j]);
        }
        std::nth_element(w.begin(), w.begin() + w.size() / 2, w.end());
        out[i] = w[w.size() / 2];
    }
    v.swap(out);
}

// Pass 4 — curvature-aware adaptive E scaling.
//
// First-layer guard: segments at Z within one modal layer-height of the
// build plate are excluded entirely. The first layer is the only one
// where reduced extrusion would meaningfully hurt adhesion, and the
// paper's caps were derived for non-first layers.
//
// Mass-conservation guard: after the rewrite, we check that the
// realised extrusion ratio (Σ E_new / Σ E_old over the touched
// segments) lies within [1 - max_red - tol, 1]. If it doesn't, that
// almost certainly indicates a parser bug — we rollback the segment-
// level E rewrites and report verification_ok = false rather than
// silently shipping an over- or under-extruded file.
size_t pass_curvature_lh(Parsed &p, Stats &stats, const Settings &s)
{
    if (!stats.verification_ok)
        return 0;

    const double first_layer_z =
        p.layer_height_modal > 1e-6 ? p.layer_height_modal + 1e-3 : 0.5;

    std::vector<size_t> idx;
    idx.reserve(p.lines.size() / 8);
    for (size_t i = 0; i < p.lines.size(); ++i) {
        const Line &l = p.lines[i];
        if (!l.is_extrusion || l.length_xy <= 1e-6)
            continue;
        // Skip the first layer to preserve bed adhesion. We use the modal Z
        // populated by reconstruct_modal — `l.z == 0` means no Z move has
        // been observed yet (synthetic tests, or pre-first-Z preamble),
        // which we deliberately don't filter so unit tests stay simple.
        if (l.z > 0.0 && l.z <= first_layer_z)
            continue;
        idx.push_back(i);
    }
    if (idx.size() < 3)
        return 0;

    const double low_rad  = s.curvature_low_deg  * M_PI / 180.0;
    const double high_rad = s.curvature_high_deg * M_PI / 180.0;
    const double max_red  = std::clamp(s.curvature_max_pct, 0, 100) / 100.0;

    std::vector<double> ratios(idx.size(), 1.0);
    for (size_t k = 1; k + 1 < idx.size(); ++k) {
        const Line &prev = p.lines[idx[k - 1]];
        const Line &cur  = p.lines[idx[k]];

        const double kappa = angle_between(prev.delta_x, prev.delta_y,
                                           cur.delta_x,  cur.delta_y);
        const double cap   = feature_cap(cur.feature);
        const double red   = std::min(max_red, cap);

        double r;
        if (kappa >= high_rad)        r = 1.0;
        else if (kappa <= low_rad)    r = 1.0 - red;
        else {
            const double t = (kappa - low_rad) / (high_rad - low_rad);
            r = (1.0 - red) + t * red;
        }
        ratios[k] = r;
    }
    ratios.front() = 1.0;
    ratios.back()  = 1.0;

    median_filter(ratios, s.curvature_filter_window);

    // First pass: snapshot the original E values for the segments we'll
    // touch, so we can rollback if the conservation check fails.
    struct OriginalE { size_t idx; double e; std::string raw; };
    std::vector<OriginalE> originals;
    originals.reserve(idx.size());

    size_t scaled = 0;
    double saved   = 0.0;
    double e_total = 0.0;
    for (size_t k = 0; k < idx.size(); ++k) {
        Line &l = p.lines[idx[k]];
        if (!l.has_e || l.e <= 0.0)
            continue;
        e_total += l.e;
        const double r = ratios[k];
        if (r >= 0.999)
            continue;
        originals.push_back({ idx[k], l.e, l.raw });
        const double e_new = l.e * r;
        saved   += (l.e - e_new);
        l.e      = e_new;
        l.e_ratio = r;
        rewrite_e_token(l.raw, e_new);
        ++scaled;
    }

    // Conservation check: saved must be no more than max_red * e_total
    // (with a 1% slack for floating-point and rounding from the median
    // filter). A larger reduction means the parser scaled something it
    // shouldn't have — roll back the segment-level rewrites and reject.
    if (e_total > 0.0) {
        const double observed_red = saved / e_total;
        if (observed_red > max_red + 0.01) {
            for (const OriginalE &o : originals) {
                Line &l = p.lines[o.idx];
                l.e     = o.e;
                l.raw   = o.raw;
                l.e_ratio = 1.0;
            }
            stats.verification_ok = false;
            stats.notes.emplace_back(
                "pass4: mass-conservation guard tripped (observed " +
                std::to_string(observed_red) + " > cap " +
                std::to_string(max_red + 0.01) + "); changes reverted");
            return 0;
        }
    }

    stats.segments_scaled    += scaled;
    stats.extrusion_saved_mm += saved;
    return scaled;
}

// ---------------------------------------------------------------------------
// Pass 2 — shrink wipe-tower purges based on extruder idle time.
// See doc/filament-economy/PASS_2_SHRINK_PURGE.md for the full design.
//
// The wipe tower brackets every tool change in Snapmaker_Orca / Bambu-derived
// G-code with explicit markers:
//
//     ; CP TOOLCHANGE START
//     T<n>
//     ... motion + extrusion (this is the purge we want to shrink) ...
//     ; CP TOOLCHANGE END
//
// The slicer sized that purge assuming the target nozzle had cooled for the
// worst-case idle. When FullSpectrum alternates filaments layer-by-layer,
// each tool's actual idle time is far shorter and the purge is oversized.
// We multiply every positive E inside the block by a ratio that grows from
// `min_ratio` (recent use, lots of savings) to 1.0 (long idle, no change).
//
// Retracts (negative E) and unretracts (positive E with zero XY motion) are
// intentionally left untouched: they must balance to keep the extruder
// primed correctly.
// ---------------------------------------------------------------------------

// Estimate wall-clock time consumed by a single G1 motion line using the
// modal feedrate carried alongside it (mm/min). We do not model
// acceleration; the resulting clock is rough but consistent across the file,
// which is all Pass 2's ratio computation needs.
double estimate_line_duration_s(const Line &l, double modal_feedrate_mm_min)
{
    const double f = (l.has_f && l.f > 0.0) ? l.f : modal_feedrate_mm_min;
    if (f <= 0.0)
        return 0.0;
    // Distance traveled in mm. For pure E moves use |delta_e| as a proxy.
    double dist = l.length_xy;
    if (dist < 1e-6 && l.has_e && std::fabs(l.e) > 1e-6)
        dist = std::fabs(l.e);
    if (dist < 1e-6)
        return 0.0;
    return dist * 60.0 / f; // mm / (mm/min) -> seconds
}

bool line_starts_toolchange_block(const std::string &raw)
{
    return raw.find("CP TOOLCHANGE START") != std::string::npos
        || raw.find("WIPE_TOWER_START")    != std::string::npos
        || raw.find("WIPE_START")          != std::string::npos;
}

bool line_ends_toolchange_block(const std::string &raw)
{
    return raw.find("CP TOOLCHANGE END") != std::string::npos
        || raw.find("WIPE_TOWER_END")    != std::string::npos
        || raw.find("WIPE_END")          != std::string::npos;
}

constexpr size_t kMaxExtruders = 16;

size_t pass_shrink_purge(Parsed &p, Stats &stats, int max_pct)
{
    if (!stats.verification_ok)
        return 0;
    // Absolute-extrusion files are not supported in this pass — scaling per
    // segment would desynchronise the cumulative counter. Pass 5 normally
    // converts to M83 first; if that did not run, log and skip.
    if (p.absolute_e) {
        stats.notes.emplace_back("Pass 2 skipped: file still uses M82 absolute extrusion.");
        return 0;
    }

    const double saturation_s = 600.0; // 10 min idle = treat as fully cooled.
    const double clamp_pct    = std::clamp(max_pct, 0, 100);
    const double min_ratio    = 1.0 - clamp_pct / 100.0;
    if (clamp_pct == 0)
        return 0;

    // Phase 1 - walk the file once to build:
    //   per_line_clock[i] = estimated wall-clock when line i is reached
    //   ex_last_use[t]    = clock at which tool t last extruded
    std::vector<double> per_line_clock(p.lines.size(), 0.0);
    std::array<double, kMaxExtruders> ex_last_use;
    ex_last_use.fill(-std::numeric_limits<double>::infinity());
    double clock_s    = 0.0;
    int    active_tool = 0;
    double modal_f    = 1200.0;
    for (size_t i = 0; i < p.lines.size(); ++i) {
        const Line &l = p.lines[i];
        per_line_clock[i] = clock_s;
        if (l.is_tool_change && l.tool >= 0 &&
            static_cast<size_t>(l.tool) < kMaxExtruders)
        {
            active_tool = l.tool;
            continue;
        }
        if (!l.is_g1)
            continue;
        if (l.has_f && l.f > 0.0)
            modal_f = l.f;
        clock_s += estimate_line_duration_s(l, modal_f);
        if (l.is_extrusion && static_cast<size_t>(active_tool) < kMaxExtruders)
            ex_last_use[active_tool] = clock_s;
    }

    // Phase 2 - find every TOOLCHANGE block and scale the E values inside.
    size_t blocks_shrunk = 0;
    double total_saved   = 0.0;
    for (size_t i = 0; i < p.lines.size(); ++i) {
        if (!line_starts_toolchange_block(p.lines[i].raw))
            continue;
        // Find the matching END marker.
        size_t end = i + 1;
        while (end < p.lines.size() && !line_ends_toolchange_block(p.lines[end].raw))
            ++end;
        if (end >= p.lines.size())
            continue;

        // Identify target tool from the first T<n> line inside the block.
        int target = -1;
        for (size_t k = i + 1; k < end; ++k) {
            if (p.lines[k].is_tool_change && p.lines[k].tool >= 0) {
                target = p.lines[k].tool;
                break;
            }
        }
        if (target < 0 || static_cast<size_t>(target) >= kMaxExtruders) {
            i = end;
            continue;
        }

        // Idle time = now - last_use.
        const double now  = per_line_clock[i];
        const double last = ex_last_use[target];
        const double idle = (std::isfinite(last)) ? std::max(0.0, now - last)
                                                  : std::numeric_limits<double>::infinity();
        const double sat  = std::clamp(idle / saturation_s, 0.0, 1.0);
        const double r    = min_ratio + sat * (1.0 - min_ratio);
        // r close to 1.0 -> nothing to gain; skip the rewrite to keep the
        // file diff minimal.
        if (r >= 0.999) {
            i = end;
            continue;
        }

        // Rewrite every positive E inside the block. Retracts (negative E)
        // and pure unretracts (positive E with no XY motion) are kept.
        size_t scaled_here = 0;
        double saved_here  = 0.0;
        for (size_t k = i + 1; k < end; ++k) {
            Line &l = p.lines[k];
            if (!l.is_g1 || !l.has_e || l.e <= 0.0 || !l.is_extrusion)
                continue;
            const double e_new = l.e * r;
            saved_here += (l.e - e_new);
            l.e = e_new;
            rewrite_e_token(l.raw, e_new);
            ++scaled_here;
        }
        if (scaled_here > 0) {
            ++blocks_shrunk;
            total_saved += saved_here;
        }
        i = end;
    }
    stats.purges_shrunk      += blocks_shrunk;
    stats.extrusion_saved_mm += total_saved;
    return blocks_shrunk;
}

// ---------------------------------------------------------------------------
// Pass 3 — collapse redundant back-to-back retract / un-retract pairs.
// See doc/filament-economy/PASS_3_MERGE_TRAVEL.md for the broader design.
//
// This v0.1 implementation handles only the safest sub-case: a retract that
// is immediately followed by an un-retract of the same magnitude, with no
// XY motion in between. Such pairs are functionally a no-op (they prime
// the filament back to where it started) and are common when two close
// extrusion segments use the same tool but the slicer emitted a retract
// "just in case". Removing them saves a handful of milliseconds and a tiny
// amount of nozzle ooze per occurrence.
//
// Out of scope for this pass:
//   - travel merging (collapsing two consecutive G1 travels into one
//     diagonal) — needs the gap-distance and gap-time analysis from the
//     design doc, and is much more sensitive to seam quality;
//   - retract pairs around tool changes — touches Pass 1's removed swaps;
//   - retract pairs separated by other extrusion lines — those represent
//     real geometry intent.
// ---------------------------------------------------------------------------

size_t pass_merge_travel(Parsed &p, Stats &stats)
{
    if (!stats.verification_ok)
        return 0;

    size_t removed = 0;
    for (size_t i = 0; i + 1 < p.lines.size(); ++i) {
        const Line &a = p.lines[i];
        if (!a.is_retract || !a.has_e)
            continue;

        // Find the next G1 line, skipping comments / blank lines / M codes
        // that don't perturb the toolhead state.
        size_t j = i + 1;
        bool   abort = false;
        while (j < p.lines.size()) {
            const Line &lj = p.lines[j];
            if (lj.is_g1) break;
            // A custom G-code or M-code between retract and unretract may
            // have side effects (temp change, fan speed, beep, …). Bail out.
            if (!lj.raw.empty() && lj.raw[0] != ';') { abort = true; break; }
            ++j;
        }
        if (abort || j >= p.lines.size())
            continue;
        const Line &b = p.lines[j];
        if (!b.is_unretract || !b.has_e)
            continue;
        // Conservative: require zero XY motion between i and j (inclusive
        // of both endpoints — a retract usually carries no XY, an
        // unretract by definition carries none).
        bool any_xy = (a.length_xy > 1e-6) || (b.length_xy > 1e-6);
        for (size_t k = i + 1; !any_xy && k < j; ++k) {
            if (p.lines[k].length_xy > 1e-6) { any_xy = true; }
        }
        if (any_xy)
            continue;
        // Magnitudes must cancel within tolerance.
        if (std::fabs(a.e + b.e) > 1e-4)
            continue;

        char note[96];
        std::snprintf(note, sizeof(note),
                      "; LeanSpectrum: collapsed redundant retract %+.3f / un-retract %+.3f",
                      a.e, b.e);
        p.lines[i].raw = note;
        p.lines[j].raw = "; LeanSpectrum: (un-retract removed by collapse above)";
        p.lines[i].is_retract   = false;
        p.lines[j].is_unretract = false;
        removed += 2;
        i = j;
    }
    stats.lines_removed += removed;
    return removed;
}

} // namespace

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

Stats process(const std::string &gcode_path, const Settings &settings)
{
    Stats stats;
    if (!settings.enable) {
        BOOST_LOG_TRIVIAL(info) << "FilamentEconomy: disabled, skipping " << gcode_path;
        return stats;
    }

    std::vector<std::string> raw;
    if (!read_file(gcode_path, raw)) {
        BOOST_LOG_TRIVIAL(error) << "FilamentEconomy: cannot read " << gcode_path;
        return stats;
    }

    Parsed p;
    p.lines.reserve(raw.size());
    FeatureType cur_feature = FeatureType::Unknown;
    for (std::string &s : raw) {
        Line l;
        l.raw = std::move(s);
        parse_line(l, cur_feature);
        p.lines.emplace_back(std::move(l));
    }

    reconstruct_modal(p);

    BOOST_LOG_TRIVIAL(info)
        << "FilamentEconomy: " << p.lines.size() << " lines parsed; "
        << "absolute_e=" << p.absolute_e
        << " retracts=" << p.retract_count_in
        << " ext_total=" << p.total_extrusion_in << " mm";

    // Pass 5 — verification gate. Sets verification_ok in stats.
    verification_pass(p, stats, settings);

    bool changed = stats.converted_to_m83;

    if (stats.verification_ok) {
        if (settings.remove_noop_swaps && pass_noop_swaps(p, stats) > 0)
            changed = true;
        if (settings.curvature_lh   && pass_curvature_lh(p, stats, settings) > 0)
            changed = true;
        if (settings.shrink_purge   && pass_shrink_purge(p, stats, settings.shrink_purge_pct) > 0)
            changed = true;
        if (settings.merge_travel   && pass_merge_travel(p, stats) > 0)
            changed = true;
    }

    if (changed) {
        std::vector<std::string> out;
        out.reserve(p.lines.size());
        for (const Line &l : p.lines)
            out.emplace_back(l.raw);
        if (!write_file(gcode_path, out)) {
            BOOST_LOG_TRIVIAL(error) << "FilamentEconomy: cannot write " << gcode_path;
            stats.notes.emplace_back("Failed to write optimised G-code; original file kept.");
            stats.modified = false;
            return stats;
        }
        stats.modified = true;
        BOOST_LOG_TRIVIAL(info)
            << "FilamentEconomy: modified file (swaps=" << stats.swaps_removed
            << " segments_scaled=" << stats.segments_scaled
            << " saved_mm=" << stats.extrusion_saved_mm
            << " m83_conv=" << stats.converted_to_m83 << ")";
    }

    return stats;
}

} // namespace FilamentEconomy
} // namespace Slic3r
