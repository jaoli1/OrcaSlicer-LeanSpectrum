//! TDS heuristic parser.
//!
//! Unlike SDS, TDS layouts are vendor-specific. We rely on label keyword
//! matching followed by numeric range extraction in a small window.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::{polymer, ExtractedFilament};

static RANGE_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(\d{2,3}(?:\.\d+)?)\s*(?:-|to|–|à|au)\s*(\d{2,3}(?:\.\d+)?)").unwrap()
});

static SPEED_UNIT_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)mm\s*/\s*s").unwrap()
});

static DENSITY_VALUE_RX: Lazy<Regex> = Lazy::new(|| {
    // Match a plausible density value (0.5 .. 1.8 g/cm^3) anywhere within the
    // configurable window around the label. Many vendor data sheets use a
    // tabular layout with tens of whitespace characters between the label and
    // the value.
    Regex::new(r"(0\.[5-9]\d?|1\.[0-7]\d?)\s*(?:g/cm|kg/m)?").unwrap()
});

static MANUFACTURER_RX: Lazy<Regex> = Lazy::new(|| {
    // Captures common company suffixes worldwide. The "Co...Ltd" form is
    // intentionally permissive — pdftotext often collapses "Co., Ltd" into
    // weird shapes like "Co,.Ltd" or "Co.Ltd".
    Regex::new(r"(?i)([^\n]{2,80}(?:Co[.,\s]{0,3}Ltd|GmbH|S\.A\.|S\.A\.S|S\.L\.|S\.R\.L|Inc\.?|LLC|Corp\.?|Limited|B\.V\.|N\.V\.|Pty\.?\s*Ltd|AG|KG|Oy|AB|AS|sp\.?z\.?o\.?o)[^\n]*)").unwrap()
});

static PRODUCT_LINE_RX: Lazy<Regex> = Lazy::new(|| {
    // After a heading like "TDS" or "Technical Data Sheet" the next line is
    // usually the short product name. We don't require an exact format.
    Regex::new(r"(?im)^\s*(PLA\+?|PETG\+?|ABS\+?|ASA|PC\+?|TPU\+?|HIPS|PP|PA\s?\d+|Nylon\s?\d*|[A-Z][A-Z0-9\-]+\s*(?:PLA|PETG|ABS|TPU|PA))\s*$").unwrap()
});

pub fn looks_like_tds(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let signals = [
        "technical data sheet",
        "fiche technique",
        "print temperature",
        "nozzle temperature",
        "bed temperature",
        "recommended settings",
        "paramètres recommandés",
        "printing parameters",
        "physical properties of materials",
    ];
    signals.iter().filter(|p| lower.contains(*p)).count() >= 1
}

/// Find the first plausible range in `window` that matches the expected unit
/// class. `expect_c=true` keeps temperatures only — values with a `%` or
/// `mm/s` suffix immediately after the range are rejected because they are
/// almost certainly fan speeds or print speeds.
fn first_range_with_unit_check(window: &str, expect_c: bool) -> Option<(f64, f64)> {
    let mut last: Option<usize> = None;
    for c in RANGE_RX.captures_iter(window) {
        let m = c.get(0)?;
        // Skip captures that have already been considered (RANGE_RX is global).
        if let Some(prev) = last { if m.start() <= prev { continue; } }
        last = Some(m.end());

        let lo: f64 = c.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
        let hi: f64 = c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
        let plausible = if expect_c {
            lo >= 30.0 && hi <= 350.0 && hi >= lo
        } else {
            lo >= 1.0 && hi <= 1000.0 && hi >= lo
        };
        if !plausible { continue; }

        // Look at the 6 chars after the match for a disqualifying unit.
        let tail_start = m.end();
        let tail = &window[tail_start..window.len().min(tail_start + 8)];
        let tail_low = tail.to_ascii_lowercase();
        if expect_c {
            if tail_low.contains('%') || tail_low.contains("mm/s") || tail_low.contains("mm /s") {
                continue;
            }
        } else {
            if tail_low.contains('°') {
                continue;
            }
        }
        return Some((lo, hi));
    }
    None
}

fn scan_range_after(text: &str, labels: &[&str], expect_c: bool) -> (Option<f64>, Option<f64>) {
    scan_range_with_hint(text, labels, expect_c, None)
}

/// Like `scan_range_after` but with an optional `expected_min` plausibility
/// floor. If the forward window produces a range whose lower bound is BELOW
/// the floor (e.g. PLA nozzle at 40-60 °C, which is obviously the bed), the
/// function falls back to a backward window scan. This handles vendor TDS
/// where pdftotext extracted the value column ahead of the label column.
fn scan_range_with_hint(
    text: &str,
    labels: &[&str],
    expect_c: bool,
    expected_min: Option<f64>,
) -> (Option<f64>, Option<f64>) {
    let lower = text.to_ascii_lowercase();
    for label in labels {
        if let Some(idx) = lower.find(&label.to_ascii_lowercase()) {
            let forward = &text[idx + label.len()..text.len().min(idx + label.len() + 200)];
            let forward_result = first_range_with_unit_check(forward, expect_c);

            let try_backward = match forward_result {
                None => !RANGE_RX.is_match(forward),
                Some((lo, _)) => expected_min.map(|m| lo < m).unwrap_or(false),
            };

            if try_backward {
                let before_start = idx.saturating_sub(200);
                let backward = &text[before_start..idx];
                if let Some((lo, hi)) = first_range_with_unit_check(backward, expect_c) {
                    if expected_min.map(|m| lo >= m).unwrap_or(true) {
                        return (Some(lo), Some(hi));
                    }
                }
            }

            if let Some((lo, hi)) = forward_result {
                if expected_min.map(|m| lo >= m).unwrap_or(true) {
                    return (Some(lo), Some(hi));
                }
            }
        }
    }
    (None, None)
}

fn scan_density(text: &str) -> Option<f64> {
    let lower = text.to_ascii_lowercase();
    let idx = lower.find("density").or_else(|| lower.find("densit"))?;
    // Some sheets put the value 50-150 chars after the label.
    let window = &text[idx..text.len().min(idx + 200)];
    DENSITY_VALUE_RX.captures(window)
        .and_then(|c| c.get(1).and_then(|m| m.as_str().parse().ok()))
}

fn scan_manufacturer(text: &str) -> Option<String> {
    // Prefer the first match within the first ~1000 chars (header area).
    let head = &text[..text.len().min(1500)];
    MANUFACTURER_RX.captures(head)
        .and_then(|c| c.get(1).map(|m| m.as_str().trim().to_string()))
}

// Words that look like a region / city / generic legal-form rather than a
// brand. Used to skip them when picking a one-word brand from the manufacturer
// line. Lowercase. Add to this list if a real-world TDS breaks the heuristic.
const NON_BRAND_WORDS: &[&str] = &[
    "shenzhen", "beijing", "shanghai", "guangzhou", "hangzhou", "ningbo", "dongguan",
    "tokyo", "osaka", "seoul",
    "london", "paris", "berlin", "munich", "milano", "milan", "madrid", "barcelona",
    "new", "north", "south", "east", "west",
    "the", "and", "of",
    "technology", "technologies", "industries", "industrial", "international",
    "company", "corporation", "co", "co.", "co,", "ltd", "limited", "gmbh", "inc",
    "filament", "filaments", "materials",
];

fn pick_brand(manufacturer: &str) -> Option<String> {
    manufacturer
        .split_whitespace()
        .find(|w| {
            let lower = w.to_ascii_lowercase();
            let trimmed = lower.trim_end_matches(|c: char| !c.is_alphanumeric());
            !NON_BRAND_WORDS.contains(&trimmed)
                && trimmed.chars().any(|c| c.is_alphabetic())
                && trimmed.len() >= 2
        })
        .map(|w| w.trim_end_matches(|c: char| !c.is_alphanumeric()).to_string())
}

fn scan_product_name(text: &str, manufacturer: Option<&str>) -> Option<String> {
    let head = &text[..text.len().min(800)];
    if let Some(c) = PRODUCT_LINE_RX.captures(head) {
        if let Some(m) = c.get(1) {
            let candidate = m.as_str().trim().to_string();
            if candidate.len() <= 40 {
                if let Some(mfr) = manufacturer {
                    if let Some(brand) = pick_brand(mfr) {
                        return Some(format!("{} {}", brand, candidate));
                    }
                }
                return Some(candidate);
            }
        }
    }
    None
}

static WORD_BOUNDED_NUM_RX: Lazy<Regex> = Lazy::new(|| {
    // Word-bounded number so "152" from a standards code like "ASTM D1525"
    // does not get picked up as a temperature.
    Regex::new(r"\b(\d{2,3}(?:\.\d+)?)\b").unwrap()
});

fn scan_glass_transition(text: &str) -> Option<f64> {
    let lower = text.to_ascii_lowercase();
    // Vicat softening temperature is a good T_g proxy for amorphous-ish
    // polymers like PLA. Heat distortion temp is a weaker proxy but better
    // than nothing.
    for label in ["glass transition", "transition vitreuse",
                  "vicat softening", "vicat", "heat distortion"] {
        if let Some(idx) = lower.find(label) {
            let window = &text[idx..text.len().min(idx + 200)];
            for m in WORD_BOUNDED_NUM_RX.find_iter(window) {
                if let Ok(v) = m.as_str().parse::<f64>() {
                    if (30.0..200.0).contains(&v) {
                        // Skip plain integers that are obviously test-method
                        // codes (e.g. "1525" -> "152" was the original
                        // failure mode; with \b we're safe, but also skip if
                        // the value is immediately preceded by "D" or "ISO"
                        // to avoid astm-style designators).
                        let prefix_start = idx + m.start().saturating_sub(8);
                        let prefix = &text[prefix_start..idx + m.start()].to_ascii_lowercase();
                        if prefix.ends_with("d") || prefix.ends_with("iso ")
                           || prefix.ends_with("astm ") || prefix.ends_with("gb/t ")
                        {
                            continue;
                        }
                        return Some(v);
                    }
                }
            }
        }
    }
    None
}

fn scan_print_speed(text: &str) -> (Option<f64>, Option<f64>) {
    let lower = text.to_ascii_lowercase();
    // Accept "print speed", "printing speed", "vitesse d'impression",
    // "vitesse" alone.
    let labels = ["printing speed", "print speed", "vitesse d'impression", "vitesse"];
    for label in labels {
        if let Some(idx) = lower.find(label) {
            let window = &text[idx..text.len().min(idx + 200)];
            if SPEED_UNIT_RX.is_match(window)
                || lower.get(idx + label.len()..idx + label.len() + 4).map(|s| s.contains("mm")).unwrap_or(false)
            {
                if let Some(c) = RANGE_RX.captures(window) {
                    let lo: f64 = c.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
                    let hi: f64 = c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
                    if lo >= 1.0 && hi <= 1000.0 && hi >= lo {
                        return (Some(lo), Some(hi));
                    }
                }
            }
        }
    }
    (None, None)
}

pub fn parse(text: &str) -> ExtractedFilament {
    let mut out = ExtractedFilament::default();
    out.polymer = polymer::detect(text);
    out.density_g_cm3 = scan_density(text)
        .or_else(|| out.polymer.and_then(|p| p.default_density_g_cm3()));
    out.manufacturer = scan_manufacturer(text);
    out.product_name = scan_product_name(text, out.manufacturer.as_deref());
    out.glass_transition_c = scan_glass_transition(text);

    // Polymer-aware sanity floors. These come from the family's default
    // nozzle/bed ranges (see polymer.rs). With them, a forward scan that
    // returns 40-60 for a PLA nozzle is recognised as the bed value
    // (probably a reverse-column table) and we retry backward.
    let nozzle_min = out.polymer.and_then(|p| p.default_nozzle_range_c())
        .map(|(lo, _)| (lo - 30.0).max(120.0)); // 30 °C below family minimum, floored at 120
    let bed_max = out.polymer.and_then(|p| p.default_bed_range_c())
        .map(|(_, hi)| hi + 20.0);

    let (n_lo, n_hi) = scan_range_with_hint(
        text,
        &["nozzle temperature", "print temperature", "extruder temperature",
          "bottom printing temperature", "température buse", "température d'impression",
          "printing temperature", "3d printing temperature"],
        true,
        nozzle_min,
    );
    out.nozzle_temp_min_c = n_lo;
    out.nozzle_temp_max_c = n_hi;
    if let (Some(lo), Some(hi)) = (n_lo, n_hi) {
        out.nozzle_temp_recommended_c = Some((lo + hi) / 2.0);
    }

    let (b_lo, b_hi) = scan_range_after(
        text,
        &["bed temperature", "heated bed", "platform temperature",
          "température plateau", "plateau chauffant", "base plate"],
        true,
    );
    // Reject implausible bed values that look more like a nozzle reading.
    out.bed_temp_min_c = b_lo.filter(|&v| bed_max.map(|m| v <= m).unwrap_or(true));
    out.bed_temp_max_c = b_hi.filter(|&v| bed_max.map(|m| v <= m + 10.0).unwrap_or(true));

    let (s_lo, s_hi) = scan_print_speed(text);
    out.print_speed_min_mm_s = s_lo;
    out.print_speed_max_mm_s = s_hi;

    // Cooling fan boolean.
    let lower = text.to_ascii_lowercase();
    if lower.contains("cooling fan: yes") || lower.contains("ventilateur: oui") {
        out.fan_enabled = Some(true);
    } else if lower.contains("cooling fan: no") || lower.contains("ventilateur: non") {
        out.fan_enabled = Some(false);
    }

    out.needs_review = out.polymer.is_none() || out.nozzle_temp_recommended_c.is_none();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polymer::Polymer;

    #[test]
    fn detects_nozzle_range() {
        let tds = "Recommended Settings\nNozzle temperature: 215-235 °C\nBed temperature: 60-70 °C\nPrint speed 40-80 mm/s\n";
        let r = parse(tds);
        assert_eq!(r.nozzle_temp_min_c, Some(215.0));
        assert_eq!(r.nozzle_temp_max_c, Some(235.0));
        assert_eq!(r.bed_temp_min_c,    Some(60.0));
        assert_eq!(r.print_speed_min_mm_s, Some(40.0));
    }

    #[test]
    fn looks_like_tds_positive() {
        assert!(looks_like_tds("Technical Data Sheet — recommended settings"));
    }

    /// Regression test built from a real-world Eryone PLA+ TDS layout
    /// (tabular, lots of whitespace between labels and values, "Printing
    /// speed" with the -ing form).
    #[test]
    fn parses_tabular_tds_layout() {
        let tds = r#"
                                                                                              Shenzhen Eryone Technology Co,.Ltd
                                                                                                                    version 1.0

     Technical Data Sheet (TDS)

         PLA+

Part I: Suggests Printing Parameters

          Parameter                                         Set up
    Nozzle temperature                                  190-220

       Bed temperature                                     55-70
          Bed material                        glass, PEI, spring steel plate

Bottom printing temperature                             190-220

         Printing speed                                30-100mm/s
       Drying conditions                            65-7512H

Part II: Physical Properties of Materials

Property                                   Testing Method  Unit Typical Value

Density(g/cm at 21.5 C ASTM D792 (ISO 1183, GB/T 1033) g/cm                   1.23

Vicat Softening Temperature( C) ASTM D1525 (ISO 306 GB/T 1633)                54
"#;
        assert!(looks_like_tds(tds));
        let r = parse(tds);
        assert_eq!(r.polymer, Some(Polymer::Pla));
        assert_eq!(r.nozzle_temp_min_c, Some(190.0));
        assert_eq!(r.nozzle_temp_max_c, Some(220.0));
        assert_eq!(r.bed_temp_min_c,    Some(55.0));
        assert_eq!(r.bed_temp_max_c,    Some(70.0));
        assert_eq!(r.print_speed_min_mm_s, Some(30.0));
        assert_eq!(r.print_speed_max_mm_s, Some(100.0));
        assert_eq!(r.density_g_cm3,     Some(1.23));
        assert_eq!(r.glass_transition_c,Some(54.0));
        assert!(r.manufacturer.as_deref().unwrap_or("").contains("Eryone"));
        // pick_brand should skip "Shenzhen" and use "Eryone" as the brand.
        assert!(r.product_name.as_deref().unwrap_or("").starts_with("Eryone"));
        assert!(r.product_name.as_deref().unwrap_or("").contains("PLA"));
        assert!(!r.needs_review);
    }

    #[test]
    fn pick_brand_skips_city_and_legal_form() {
        assert_eq!(pick_brand("Shenzhen Eryone Technology Co.,Ltd").as_deref(), Some("Eryone"));
        assert_eq!(pick_brand("Berlin Polymaker GmbH").as_deref(),               Some("Polymaker"));
        assert_eq!(pick_brand("FormFutura B.V.").as_deref(),                     Some("FormFutura"));
    }

    /// ROSA3D PLA Speed TDS layout — pdftotext extracted the VALUE column
    /// ahead of the PARAMETER column, so the forward window for the nozzle
    /// label "3D printing temperature" carries 40-60 (the bed value), and
    /// 220-250 (the real nozzle value) sits BEFORE the label. The
    /// polymer-aware sanity floor catches this and triggers a backward
    /// scan.
    #[test]
    fn parses_rosa3d_reverse_column_layout() {
        let tds = r#"
                                         TECHNICAL DATA SHEET

                                                            FILAMENT 3D PLA Speed Matt

RECOMMENDED PRINTING PARAMETERS                                      VALUE
                                                                    220-250
                          PARAMETER
              3D printing temperature [C]                            40-60
                                                                     70-100
                        Heated bed [C]
                        Cooling fan [%]                                 no
                       Closed chamber                                 50/4
                  Drying conditions [C/h]
"#;
        let r = parse(tds);
        assert_eq!(r.polymer, Some(Polymer::Pla));
        // Forward scan finds 40-60 first; PLA expected_min (160) rejects it
        // and the backward scan returns 220-250.
        assert_eq!(r.nozzle_temp_min_c, Some(220.0));
        assert_eq!(r.nozzle_temp_max_c, Some(250.0));
        // Bed is found by the conservative backward scan (forward window
        // is empty of ranges) and accepted because 40-60 is within the PLA
        // bed window.
        assert_eq!(r.bed_temp_min_c, Some(40.0));
        assert_eq!(r.bed_temp_max_c, Some(60.0));
    }
}
