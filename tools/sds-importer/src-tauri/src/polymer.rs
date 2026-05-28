//! Polymer-family detection from extracted SDS / TDS text.
//!
//! Detection is based on a small table of identifiers per polymer:
//! CAS Registry numbers (factual, publicly catalogued) and common
//! abbreviations / IUPAC names. A single match is enough; additives are
//! ignored.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Polymer {
    Pla,
    Petg,
    Abs,
    Asa,
    Pc,
    Tpu,
    NylonPa6,
    NylonPa12,
    Hips,
    Pp,
    Other,
}

impl Polymer {
    pub fn as_str(&self) -> &'static str {
        match self {
            Polymer::Pla       => "PLA",
            Polymer::Petg      => "PETG",
            Polymer::Abs       => "ABS",
            Polymer::Asa       => "ASA",
            Polymer::Pc        => "PC",
            Polymer::Tpu       => "TPU",
            Polymer::NylonPa6  => "PA6",
            Polymer::NylonPa12 => "PA12",
            Polymer::Hips      => "HIPS",
            Polymer::Pp        => "PP",
            Polymer::Other     => "Other",
        }
    }

    /// Map a filament database `base_type` string (e.g. "PLA", "PETG", "PA12",
    /// "PC-ABS", "TPE") to the closest polymer family. Flexible families collapse
    /// to TPU, polyamides to PA6/PA12, and anything we have no FDM profile for
    /// (resin, PEEK, PEI, …) falls back to `Other` — the generator then leans on
    /// the data sheet's own temperatures plus a generic parent.
    pub fn from_base_type(s: &str) -> Polymer {
        match s.trim().to_ascii_uppercase().as_str() {
            "PLA" | "PLA+" | "PLA-CF" | "PLGA" | "PLCL" | "PCL" | "PDO" => Polymer::Pla,
            "PETG" | "PET-G" | "PCTG" | "PET" | "RPET" | "PBT" => Polymer::Petg,
            "ABS" | "PC-ABS" | "ABS-GF" => Polymer::Abs,
            "ASA" => Polymer::Asa,
            "PC" => Polymer::Pc,
            "TPU" | "TPE" | "TPC" | "TPS" | "TPI" | "SEBS" | "SBS" | "SBC" | "PEBA" | "EVA" | "OBC" => Polymer::Tpu,
            "PA12" | "NYLON12" => Polymer::NylonPa12,
            "PA" | "PA6" | "NYLON" | "PPA" | "PAHT" | "PA-CF" => Polymer::NylonPa6,
            "HIPS" | "PS" => Polymer::Hips,
            "PP" => Polymer::Pp,
            _ => Polymer::Other,
        }
    }

    /// Name of the stock OrcaSlicer-family "Generic …" filament to inherit from
    /// when the chosen printer is NOT the Snapmaker U1 (those generic profiles
    /// ship across the OrcaSlicer family and are broadly printer-compatible).
    /// HIPS/PP have no "Generic" leaf in the shipped set, so they fall back to
    /// the nearest thermal sibling (HIPS≈ABS, PP≈PETG); `Other`→Generic PLA.
    pub fn orca_generic_parent(&self) -> &'static str {
        match self {
            Polymer::Pla       => "Generic PLA",
            Polymer::Petg      => "Generic PETG",
            Polymer::Abs       => "Generic ABS",
            Polymer::Asa       => "Generic ASA",
            Polymer::Pc        => "Generic PC",
            Polymer::Tpu       => "Generic TPU",
            Polymer::NylonPa6  => "Generic PA",
            Polymer::NylonPa12 => "Generic PA",
            Polymer::Hips      => "Generic ABS",
            Polymer::Pp        => "Generic PETG",
            Polymer::Other     => "Generic PLA",
        }
    }

    /// Conservative default printing temperatures used when no TDS data is
    /// available. The values come from public consumer-grade filament data
    /// and are intentionally a starting point — users are expected to tune.
    pub fn default_nozzle_range_c(&self) -> Option<(f64, f64)> {
        Some(match self {
            Polymer::Pla       => (190.0, 220.0),
            Polymer::Petg      => (220.0, 250.0),
            Polymer::Abs       => (230.0, 260.0),
            Polymer::Asa       => (240.0, 265.0),
            Polymer::Pc        => (260.0, 290.0),
            Polymer::Tpu       => (210.0, 240.0),
            Polymer::NylonPa6  => (240.0, 270.0),
            Polymer::NylonPa12 => (250.0, 280.0),
            Polymer::Hips      => (220.0, 250.0),
            Polymer::Pp        => (220.0, 245.0),
            Polymer::Other     => return None,
        })
    }

    pub fn default_bed_range_c(&self) -> Option<(f64, f64)> {
        Some(match self {
            Polymer::Pla       => (50.0, 60.0),
            Polymer::Petg      => (70.0, 85.0),
            Polymer::Abs       => (95.0, 110.0),
            Polymer::Asa       => (95.0, 110.0),
            Polymer::Pc        => (105.0, 120.0),
            Polymer::Tpu       => (40.0, 60.0),
            Polymer::NylonPa6  => (70.0, 90.0),
            Polymer::NylonPa12 => (60.0, 80.0),
            Polymer::Hips      => (90.0, 110.0),
            Polymer::Pp        => (90.0, 110.0),
            Polymer::Other     => return None,
        })
    }

    pub fn default_density_g_cm3(&self) -> Option<f64> {
        Some(match self {
            Polymer::Pla       => 1.24,
            Polymer::Petg      => 1.27,
            Polymer::Abs       => 1.04,
            Polymer::Asa       => 1.07,
            Polymer::Pc        => 1.20,
            Polymer::Tpu       => 1.21,
            Polymer::NylonPa6  => 1.13,
            Polymer::NylonPa12 => 1.01,
            Polymer::Hips      => 1.04,
            Polymer::Pp        => 0.90,
            Polymer::Other     => return None,
        })
    }

    /// Conservative default maximum volumetric speed (mm³/s) for the polymer
    /// family. These are the values that most affect print success when a
    /// vendor TDS does not provide an explicit flow limit. Tuned for a
    /// standard 0.4 mm nozzle on a non-volcano hotend; high-flow setups
    /// (volcano, CHT) can typically run 1.5-2× higher.
    pub fn default_max_flow_mm3_s(&self) -> Option<f64> {
        Some(match self {
            Polymer::Pla       => 12.0,
            Polymer::Petg      =>  9.0,
            Polymer::Abs       =>  9.0,
            Polymer::Asa       =>  9.0,
            Polymer::Pc        =>  7.0,
            Polymer::Tpu       =>  4.0,
            Polymer::NylonPa6  =>  8.0,
            Polymer::NylonPa12 =>  8.0,
            Polymer::Hips      =>  9.0,
            Polymer::Pp        =>  7.0,
            Polymer::Other     => return None,
        })
    }

    /// Recommended scarf-joint seam settings per polymer family. These are
    /// the OrcaSlicer fields that hide the Z-seam line; values come from
    /// the OrcaSlicer wiki defaults + community guides
    /// (orcaslicer.com/wiki/print_settings/quality/quality_settings_seam,
    ///  Obico OrcaSlicer Seam Settings guide).
    ///
    /// PLA: the friendliest material for scarf seams — defaults work as-is.
    /// PETG: needs slower scarf speed to limit stringing across the ramp.
    /// ABS/ASA/PC: similar speed cap to PETG, slightly tighter angle
    ///   threshold so the conditional scarf only fires on smoother sections.
    /// TPU: skip the scarf entirely (rubbery polymer doesn't ramp cleanly);
    ///   fall back to aligned seam + manual paint.
    /// Nylon: same as PETG family for processing reasons.
    pub fn default_scarf_settings(&self) -> ScarfSettings {
        match self {
            Polymer::Pla => ScarfSettings {
                enable_scarf:         true,
                scarf_joint_speed_pct: 50,   // % of outer wall speed
                scarf_length_mm:      20.0,
                scarf_steps:          10,
                scarf_flow_ratio_pct: 100,
                scarf_angle_deg:      155,
                seam_position:        "back".into(),
            },
            Polymer::Petg | Polymer::Hips => ScarfSettings {
                enable_scarf:         true,
                scarf_joint_speed_pct: 40,
                scarf_length_mm:      20.0,
                scarf_steps:          12,
                scarf_flow_ratio_pct: 100,
                scarf_angle_deg:      150,
                seam_position:        "back".into(),
            },
            Polymer::Abs | Polymer::Asa | Polymer::Pc => ScarfSettings {
                enable_scarf:         true,
                scarf_joint_speed_pct: 40,
                scarf_length_mm:      18.0,
                scarf_steps:          12,
                scarf_flow_ratio_pct: 100,
                scarf_angle_deg:      150,
                seam_position:        "back".into(),
            },
            Polymer::NylonPa6 | Polymer::NylonPa12 | Polymer::Pp => ScarfSettings {
                enable_scarf:         true,
                scarf_joint_speed_pct: 35,
                scarf_length_mm:      18.0,
                scarf_steps:          12,
                scarf_flow_ratio_pct: 100,
                scarf_angle_deg:      150,
                seam_position:        "back".into(),
            },
            Polymer::Tpu => ScarfSettings {
                enable_scarf:         false,
                scarf_joint_speed_pct: 0,
                scarf_length_mm:      0.0,
                scarf_steps:          0,
                scarf_flow_ratio_pct: 100,
                scarf_angle_deg:      0,
                seam_position:        "aligned".into(),
            },
            Polymer::Other => ScarfSettings::default(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScarfSettings {
    pub enable_scarf:         bool,
    pub scarf_joint_speed_pct: u32,   // % of outer wall speed
    pub scarf_length_mm:      f64,
    pub scarf_steps:          u32,
    pub scarf_flow_ratio_pct: u32,
    pub scarf_angle_deg:      u32,
    pub seam_position:        String, // "back" | "aligned" | "nearest" | "random"
}

impl Default for ScarfSettings {
    fn default() -> Self {
        ScarfSettings {
            enable_scarf:         true,
            scarf_joint_speed_pct: 50,
            scarf_length_mm:      20.0,
            scarf_steps:          10,
            scarf_flow_ratio_pct: 100,
            scarf_angle_deg:      155,
            seam_position:        "back".into(),
        }
    }
}

struct Signature {
    polymer:        Polymer,
    cas_numbers:    &'static [&'static str],
    name_patterns:  &'static [&'static str],
}

const SIGNATURES: &[Signature] = &[
    Signature {
        polymer: Polymer::Pla,
        cas_numbers: &["9051-89-2", "26100-51-6", "33135-50-1"],
        name_patterns: &["polylact", "PLA", "acide polylactique"],
    },
    Signature {
        polymer: Polymer::Petg,
        cas_numbers: &["25640-14-6", "30965-26-5"],
        name_patterns: &["PETG", "PET-G", "polyethylene terephthalate glycol"],
    },
    Signature {
        polymer: Polymer::Abs,
        cas_numbers: &["9003-56-9"],
        name_patterns: &["acrylonitrile butadiene styrene", "ABS"],
    },
    Signature {
        polymer: Polymer::Asa,
        cas_numbers: &["26299-47-8"],
        name_patterns: &["acrylonitrile styrene acrylate", "ASA"],
    },
    Signature {
        polymer: Polymer::Pc,
        cas_numbers: &["24936-68-3", "25037-45-0"],
        name_patterns: &["polycarbonate", "PC "],
    },
    Signature {
        polymer: Polymer::Tpu,
        cas_numbers: &["75880-72-1", "9009-54-5"],
        name_patterns: &["thermoplastic polyurethane", "TPU"],
    },
    Signature {
        polymer: Polymer::NylonPa6,
        cas_numbers: &["32131-17-2", "25038-54-4"],
        name_patterns: &["polyamide 6", "polyamide-6", "PA6 ", "PA 6", "nylon 6"],
    },
    Signature {
        polymer: Polymer::NylonPa12,
        cas_numbers: &["32954-72-4", "25038-74-8"],
        name_patterns: &["polyamide 12", "polyamide-12", "PA12", "PA 12", "nylon 12"],
    },
    Signature {
        polymer: Polymer::Hips,
        cas_numbers: &["9003-55-8"],
        name_patterns: &["high impact polystyrene", "HIPS"],
    },
    Signature {
        polymer: Polymer::Pp,
        cas_numbers: &["9003-07-0"],
        name_patterns: &["polypropylene", "polypropylène", "PP "],
    },
];

/// Detect the polymer family from a piece of SDS / TDS text.
/// Returns `Some(Polymer::Other)` only when explicit "thermoplastic" or
/// "polymer" wording is detected without a more specific family.
pub fn detect(text: &str) -> Option<Polymer> {
    let lower = text.to_ascii_lowercase();
    for sig in SIGNATURES {
        for cas in sig.cas_numbers {
            if lower.contains(&cas.to_ascii_lowercase()) {
                return Some(sig.polymer);
            }
        }
        for pat in sig.name_patterns {
            if lower.contains(&pat.to_ascii_lowercase()) {
                return Some(sig.polymer);
            }
        }
    }
    if lower.contains("thermoplastic") || lower.contains("polymer") {
        return Some(Polymer::Other);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_pla_by_cas() {
        let txt = "Composition: polylactic acid CAS 9051-89-2.";
        assert_eq!(detect(txt), Some(Polymer::Pla));
    }

    #[test]
    fn detects_petg_by_name() {
        let txt = "Material: PETG copolyester.";
        assert_eq!(detect(txt), Some(Polymer::Petg));
    }

    #[test]
    fn returns_none_when_no_clue() {
        assert_eq!(detect("hello world"), None);
    }
}
