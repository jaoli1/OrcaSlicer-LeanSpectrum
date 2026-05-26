#include "FilamentEconomy.hpp"

#include <algorithm>
#include <cstdio>
#include <fstream>
#include <regex>
#include <sstream>
#include <string>
#include <vector>

#include <boost/log/trivial.hpp>

#include "../PrintConfig.hpp"

namespace Slic3r {
namespace FilamentEconomy {

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

Settings Settings::from_config(const DynamicPrintConfig &config)
{
    Settings s;

    if (const ConfigOptionBool *opt = config.option<ConfigOptionBool>("filament_economy_enable"))
        s.enable = opt->value;

    if (const ConfigOptionBool *opt = config.option<ConfigOptionBool>("filament_economy_remove_noop_swaps"))
        s.remove_noop_swaps = opt->value;

    if (const ConfigOptionBool *opt = config.option<ConfigOptionBool>("filament_economy_shrink_purge"))
        s.shrink_purge = opt->value;

    if (const ConfigOptionInt *opt = config.option<ConfigOptionInt>("filament_economy_shrink_purge_pct"))
        s.shrink_purge_pct = std::clamp(opt->value, 0, 100);

    if (const ConfigOptionBool *opt = config.option<ConfigOptionBool>("filament_economy_merge_travel"))
        s.merge_travel = opt->value;

    return s;
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

namespace {

// A tool-change event located in the input G-code by line index.
struct ToolChange
{
    size_t line_idx       = 0;        // Index of the T<n> line in the input buffer
    int    tool           = -1;       // Target tool number (T<n>)
    size_t block_start    = 0;        // First line of the swap block (purge prologue)
    size_t block_end      = 0;        // First line *after* the swap block (exclusive)
};

// Parsed view of the G-code split into lines, plus all tool-change events.
struct Parsed
{
    std::vector<std::string> lines;
    std::vector<ToolChange>  tool_changes;
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

// Match "T0", "T12", possibly followed by spaces or a comment.
const std::regex re_tool_change(R"(^\s*T(\d+)\s*(;.*)?$)");

void index_tool_changes(Parsed &p)
{
    p.tool_changes.clear();
    std::smatch m;
    for (size_t i = 0; i < p.lines.size(); ++i) {
        if (std::regex_match(p.lines[i], m, re_tool_change)) {
            ToolChange tc;
            tc.line_idx    = i;
            tc.tool        = std::stoi(m[1].str());
            tc.block_start = i;     // refined in a later pass
            tc.block_end   = i + 1; // refined in a later pass
            p.tool_changes.emplace_back(tc);
        }
    }
}

} // namespace

// ---------------------------------------------------------------------------
// Passes (skeleton — to be implemented)
// ---------------------------------------------------------------------------

// Pass 1: detect no-op swaps and mark them for removal.
// A no-op swap is a T<n> that points to the same physical extruder as the
// currently active one. Returns the number of swaps that would be removed.
static size_t pass_noop_swaps(Parsed &p, Stats &stats)
{
    if (p.tool_changes.size() < 2)
        return 0;

    size_t removed = 0;
    int    current = p.tool_changes.front().tool;

    // Iterate from the second tool change onwards; if it matches the current
    // tool, mark the line for deletion by clearing it and skip the swap block.
    for (size_t i = 1; i < p.tool_changes.size(); ++i) {
        ToolChange &tc = p.tool_changes[i];
        if (tc.tool == current) {
            // Mark the T<n> line as a comment so the rewriter drops it later.
            p.lines[tc.line_idx] = "; LeanSpectrum: removed no-op T" + std::to_string(tc.tool);
            ++removed;
        } else {
            current = tc.tool;
        }
    }

    stats.swaps_removed += removed;
    return removed;
}

// Pass 2: shrink purge volumes based on recent same-tool activity.
// Skeleton — not yet implemented.
static size_t pass_shrink_purge(Parsed &p, Stats &stats, int /*max_pct*/)
{
    (void)p;
    (void)stats;
    // TODO(leanspectrum): parse "; WIPE_START" / "; WIPE_END" markers and
    // shrink E values inside.
    return 0;
}

// Pass 3: merge redundant travel + retract pairs around kept swaps.
// Skeleton — not yet implemented.
static size_t pass_merge_travel(Parsed &p, Stats &stats)
{
    (void)p;
    (void)stats;
    // TODO(leanspectrum): collapse "; TRAVEL" sequences around kept swaps.
    return 0;
}

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

    Parsed parsed;
    if (!read_file(gcode_path, parsed.lines)) {
        BOOST_LOG_TRIVIAL(error) << "FilamentEconomy: cannot read " << gcode_path;
        return stats;
    }

    index_tool_changes(parsed);

    BOOST_LOG_TRIVIAL(info)
        << "FilamentEconomy: " << parsed.lines.size() << " lines, "
        << parsed.tool_changes.size() << " tool changes detected";

    if (parsed.tool_changes.size() < 2) {
        // Single-material or no swaps — nothing to optimise.
        return stats;
    }

    bool changed = false;

    if (settings.remove_noop_swaps)
        if (pass_noop_swaps(parsed, stats) > 0)
            changed = true;

    if (settings.shrink_purge)
        if (pass_shrink_purge(parsed, stats, settings.shrink_purge_pct) > 0)
            changed = true;

    if (settings.merge_travel)
        if (pass_merge_travel(parsed, stats) > 0)
            changed = true;

    if (changed) {
        if (!write_file(gcode_path, parsed.lines)) {
            BOOST_LOG_TRIVIAL(error) << "FilamentEconomy: cannot write " << gcode_path;
            return stats;
        }
        stats.modified = true;
        BOOST_LOG_TRIVIAL(info)
            << "FilamentEconomy: removed " << stats.swaps_removed << " no-op swaps, "
            << stats.purges_shrunk << " purges shrunk, "
            << stats.lines_removed << " lines removed";
    }

    return stats;
}

} // namespace FilamentEconomy
} // namespace Slic3r
