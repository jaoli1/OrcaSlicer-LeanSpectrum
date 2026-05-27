#ifndef slic3r_AutoProfile_hpp_
#define slic3r_AutoProfile_hpp_

#include <string>
#include <vector>

#include "PrintConfig.hpp"

// LeanSpectrum auto-profile generator.
//
// Maps a single high-level user intent ("draft", "quality", "strength", …)
// onto a curated bundle of print-settings overrides that the slicer then
// applies on top of the active profile. The point is to give non-expert
// users a one-click way to get a reasonable result on the Snapmaker U1
// without having to learn 200 individual settings.
//
// Design notes:
// - Each intent is a small table of "key → value" overrides expressed in
//   the same units PrintConfig uses (mm, %, mm/s, layer counts).
// - Intent overrides come first; filament-aware overrides (cooling, retract,
//   max flow) come second and refine the intent for the active filament's
//   polymer family. This lets a single intent like "Quality" produce
//   sensible settings for PLA, PETG, ABS, and TPU without per-material
//   intent tables.
// - The module only writes values back to a DynamicPrintConfig — it does
//   NOT mutate preset bundles or UI state directly. The caller (Plater)
//   does the GUI plumbing.
//
// See doc/leanspectrum/AUTO_PROFILE.md (to be added) for the rationale
// behind each value choice and the cross-reference to the OrcaSlicer wiki
// material temperature / calibration pages.

namespace Slic3r {
namespace AutoProfile {

enum class Intent : int {
    Draft       = 0, // fastest print, lowest visual fidelity, lowest filament
    Standard    = 1, // balanced default
    HighQuality = 2, // small layers, smooth surfaces, scarf seam
    Strength    = 3, // many walls, dense infill, slower
    Decorative  = 4, // medium layers, low infill, scarf seam, optimised for looks
};

// Human-readable intent labels for the UI. Index matches the enum value.
const char *intent_label(Intent intent);
const char *intent_description(Intent intent);

// Polymer family detected from the active filament_type. Used by the
// material-overrides layer.
enum class Polymer : int {
    PLA    = 0,
    PETG   = 1,
    ABS    = 2, // includes ASA
    PC     = 3,
    PA     = 4, // Nylon
    TPU    = 5,
    HIPS   = 6,
    PP     = 7,
    Unknown = 8,
};

Polymer polymer_from_type(const std::string &filament_type);

// Apply the (intent, polymer) bundle to the given config in place.
// Returns a list of human-readable change notes ("layer_height -> 0.12 mm",
// "wall_loops -> 3", …) so the caller can show the user what changed.
//
// Only keys that the bundle explicitly sets are touched; anything else in
// the config is left as-is.
std::vector<std::string> apply(DynamicPrintConfig &config,
                               Intent              intent,
                               Polymer             polymer);

// Convenience overload: derives Polymer from the config's filament_type
// option (first filament if multi-material).
std::vector<std::string> apply(DynamicPrintConfig &config, Intent intent);

} // namespace AutoProfile
} // namespace Slic3r

#endif /* slic3r_AutoProfile_hpp_ */
