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
