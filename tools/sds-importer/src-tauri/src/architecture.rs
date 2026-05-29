//! Printer multi-material **architecture** (v0.7.0).
//!
//! Classifies each catalogue model so the generated process can pick the right
//! purge-tower lever and so the UI can show an "I use an AMS / CFS / MMU"
//! checkbox only where it makes sense. Sourced from OFFICIAL manufacturer sites
//! (Bambu, Creality, Prusa, Snapmaker, Anycubic, Flashforge, Qidi, Elegoo,
//! Raise3D, Sovol, RatRig, Volumic, Lulzbot, WonderMaker) cross-checked against
//! the shipped slicer profiles' nozzle/tool counts. Anything not listed defaults
//! to `Single` (a single-nozzle printer with no multi-material option → it never
//! builds a purge tower, so the tower keys are harmless no-ops).
//!
//! Why this matters (proven in the slicer source, `Print.cpp` ~3125 / ~3347):
//!   • `MultiNozzle` (tool-changer / IDEX): each nozzle keeps its own colour, so
//!     there is NO inter-colour purge through a shared nozzle. The slicer lays
//!     `prime_volume` per tool; `flush_multiplier`/`flush_volumes` are ignored.
//!   • `AmsCapable` (single nozzle + optional AMS/CFS/MMU/ACE…): the nozzle must
//!     purge the previous colour on every change → `flush_multiplier` is the
//!     lever. Multi-material only when the user actually has the add-on.

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    /// Single nozzle, no multi-material option → never a purge tower.
    Single,
    /// ≥2 independent nozzles / tool-changer / IDEX — inherently multi-material.
    /// Lever: `prime_volume` (per-tool prime); `flush_multiplier` is ignored.
    MultiNozzle,
    /// Single nozzle + an OPTIONAL multi-material system (AMS/CFS/MMU/ACE/…).
    /// Multi-material only if the user enables it (the UI checkbox); then the
    /// lever is `flush_multiplier` + the wipe tower.
    AmsCapable,
}

/// Models with ≥2 nozzles / IDEX / tool-changer (manufacturer-confirmed).
/// NB: only the Snapmaker "*Dual*" / Artisan variants are dual — the plain
/// A250/A350 (+BKit/QSKit) are single-extruder modules and stay `Single`.
const MULTI_NOZZLE: &[&str] = &[
    "Snapmaker U1", "Snapmaker J1", "Snapmaker Artisan",
    "Snapmaker A250 Dual", "Snapmaker A250 Dual BKit",
    "Snapmaker A250 Dual QS+B Kit", "Snapmaker A250 Dual QSKit",
    "Snapmaker A350 Dual", "Snapmaker A350 Dual BKit",
    "Snapmaker A350 Dual QS+B Kit", "Snapmaker A350 Dual QSKit",
    "Prusa XL", "Prusa XL 5T",
    "Raise3D Pro3", "Raise3D Pro3 Plus",
    "Lulzbot Taz Pro Dual",
    "Sovol SV02",
    "Elegoo Neptune 2D",
    "RatRig V-Core 4 IDEX 300", "RatRig V-Core 4 IDEX 300 COPY MODE", "RatRig V-Core 4 IDEX 300 MIRROR MODE",
    "RatRig V-Core 4 IDEX 400", "RatRig V-Core 4 IDEX 400 COPY MODE", "RatRig V-Core 4 IDEX 400 MIRROR MODE",
    "RatRig V-Core 4 IDEX 500", "RatRig V-Core 4 IDEX 500 COPY MODE", "RatRig V-Core 4 IDEX 500 MIRROR MODE",
    "EXO42 IDRE", "EXO65 IDRE", "SH65 IDRE",
    "WonderMaker ZR Ultra", "WonderMaker ZR Ultra S",
    "TiQ2", "TiQ8",
    "Z-Bolt S300 Dual", "Z-Bolt S400 Dual", "Z-Bolt S600 Dual",
    "Z-Bolt S800 Dual", "Z-Bolt S1000 Dual",
];

/// Single-nozzle models that support an OPTIONAL multi-material system, with the
/// system's display name. Multi-material only when the user enables it.
const AMS_CAPABLE: &[(&str, &str)] = &[
    ("Bambu Lab X1", "AMS"), ("Bambu Lab X1 Carbon", "AMS"), ("Bambu Lab X1E", "AMS"),
    ("Bambu Lab P1P", "AMS"), ("Bambu Lab P1S", "AMS"),
    ("Bambu Lab A1", "AMS lite"), ("Bambu Lab A1 mini", "AMS lite"),
    ("Creality K2 Plus", "CFS"), ("Creality K1", "CFS"), ("Creality K1 Max", "CFS"),
    ("Creality K1C", "CFS"), ("Creality K1 SE", "CFS"),
    ("Anycubic Kobra 3", "ACE Pro"), ("Anycubic Kobra S1", "ACE Pro"),
    ("Flashforge AD5X", "IFS"),
    ("Qidi Q2", "QIDI Box"),
    ("Elegoo Centauri Carbon", "CANVAS"),
    ("Prusa MK4S", "MMU3"), ("Prusa MK4S HF", "MMU3"), ("Prusa MK4", "MMU3"),
    ("Prusa MK3.5", "MMU3"), ("Prusa MK3S", "MMU3"),
    ("Prusa CORE One", "INDX"), ("Prusa CORE One HF", "INDX"),
];

/// Classify a catalogue model name → (architecture, optional MM-system label).
/// Unknown models default to `Single`.
pub fn classify(model: &str) -> (Architecture, Option<&'static str>) {
    if MULTI_NOZZLE.contains(&model) {
        return (Architecture::MultiNozzle, None);
    }
    if let Some((_, system)) = AMS_CAPABLE.iter().find(|(m, _)| *m == model) {
        return (Architecture::AmsCapable, Some(system));
    }
    (Architecture::Single, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_changers_are_multi_nozzle() {
        for m in ["Snapmaker U1", "Snapmaker J1", "Prusa XL 5T", "Sovol SV02",
                  "RatRig V-Core 4 IDEX 400", "EXO65 IDRE"] {
            assert_eq!(classify(m).0, Architecture::MultiNozzle, "{m}");
        }
    }

    #[test]
    fn ams_systems_are_labelled() {
        assert_eq!(classify("Bambu Lab X1 Carbon"), (Architecture::AmsCapable, Some("AMS")));
        assert_eq!(classify("Bambu Lab A1"), (Architecture::AmsCapable, Some("AMS lite")));
        assert_eq!(classify("Creality K2 Plus"), (Architecture::AmsCapable, Some("CFS")));
        assert_eq!(classify("Prusa MK4S"), (Architecture::AmsCapable, Some("MMU3")));
        assert_eq!(classify("Qidi Q2"), (Architecture::AmsCapable, Some("QIDI Box")));
    }

    #[test]
    fn unknown_and_single_extruder_default_to_single() {
        for m in ["Creality Ender-3", "Snapmaker A350", "Prusa MINI",
                  "WonderMaker ZR", "Anycubic Kobra 2 Max", "Elegoo Centauri"] {
            assert_eq!(classify(m), (Architecture::Single, None), "{m}");
        }
    }
}
