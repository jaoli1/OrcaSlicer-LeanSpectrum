//! TDS heuristic parser.
//!
//! Unlike SDS, TDS layouts are vendor-specific. We rely on label keyword
//! matching followed by numeric range extraction in a small window.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::text_utils::safe_slice;
use crate::{polymer, ExtractedFilament};

static RANGE_RX: Lazy<Regex> = Lazy::new(|| {
    // An optional inline temperature unit (°C / ℃ / ℉ / °) is allowed
    // BETWEEN the first number and the separator. Vendor TDS such as the
    // Eryone PLA+ write ranges as "190℃-220℃" where the unit clings to
    // each number; without skipping it the first number's trailing unit
    // blocks the dash match and the whole range is lost. Both the raw
    // glyph (℃) and the normalized form (°C) are accepted so the regex is
    // robust whether or not normalize_unicode ran first.
    Regex::new(r"(?i)(\d{2,3}(?:\.\d+)?)\s*(?:°\s*[cf]|℃|℉|°)?\s*(?:-|to|–|à|au)\s*(\d{2,3}(?:\.\d+)?)").unwrap()
});

static SPEED_UNIT_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)mm\s*/\s*s").unwrap()
});

static DENSITY_VALUE_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(0\.[5-9]\d?|1\.[0-7]\d?)").unwrap()
});
static DENSITY_VALUE_WITH_UNIT_AFTER_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(0\.[5-9]\d?|1\.[0-7]\d?)\s*(?:g\s*/\s*cm\d*|kg\s*/\s*m\d*)").unwrap()
});
static DENSITY_VALUE_WITH_UNIT_BEFORE_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:g\s*/\s*cm\d*|kg\s*/\s*m\d*)\s*[^\d\n]{0,40}(0\.[5-9]\d?|1\.[0-7]\d?)").unwrap()
});

static MANUFACTURER_RX: Lazy<Regex> = Lazy::new(|| {
    // Captures common company suffixes worldwide. Safeguards against
    // false positives:
    //   1. The distinctive multi-char / punctuated forms (Co Ltd, GmbH,
    //      Inc, LLC, Limited, B.V., …) may be GLUED to the preceding word
    //      because pdf-extract frequently drops the space — the Eryone
    //      TDS extracts as "Shenzhen Eryone TechnologyCo,.Ltd". A leading
    //      \b would reject that, so it is omitted for these forms.
    //   2. The ambiguous 2-letter forms (AG, KG, Oy, AB) DO keep a leading
    //      \b so "AG" doesn't match inside "drAG" / "KG" inside "10KG".
    //   3. The suffix must be at the END of the line (optional trailing
    //      period + whitespace). Real company-name lines end with the
    //      legal form; false friends like "190/2.16 kg g/10 min" have more
    //      text after.
    Regex::new(r"(?im)^\s*([^\n]{2,80}(?:Co[.,;\s]{0,3}Ltd|GmbH|S\.A\.S?|S\.L\.|S\.R\.L|Inc\.?|LLC|Corp\.?|Limited|B\.V\.|N\.V\.|Pty\.?\s*Ltd|A/S|Sp\.?\s*z\.?\s*o\.?\s*o\.?|s\.r\.o\.?|\b(?:AG|KG|Oy|AB))\b\.?)\s*$").unwrap()
});

static PRODUCT_LINE_RX: Lazy<Regex> = Lazy::new(|| {
    // After a heading like "TDS" or "Technical Data Sheet" the next line is
    // usually the short product name. We don't require an exact format.
    Regex::new(r"(?im)^\s*(PLA\+?|PETG\+?|ABS\+?|ASA|PC\+?|TPU\+?|HIPS|PP|PA\s?\d+|Nylon\s?\d*|[A-Z][A-Z0-9\-]+\s*(?:PLA|PETG|ABS|TPU|PA))\s*$").unwrap()
});

static LABELLED_PRODUCT_NAME_RX: Lazy<Regex> = Lazy::new(|| {
    // "Product Name: <value>" / "PRODUCT NAME:  <value>" / "Nom du produit : ..."
    Regex::new(r"(?im)^\s*(?:\d+\.\d+\s+)?(?:product\s*(?:name|identification|identifier)|trade\s*name|nom\s+du\s+produit|nom\s+commercial)\s*[:\-]?\s+(\S.{1,80}?)\s*$").unwrap()
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
        let tail = safe_slice(window, tail_start, tail_start + 8);
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
            let forward = safe_slice(text, idx + label.len(), idx + label.len() + 200);
            let forward_result = first_range_with_unit_check(forward, expect_c);

            let try_backward = match forward_result {
                None => !RANGE_RX.is_match(forward),
                Some((lo, _)) => expected_min.map(|m| lo < m).unwrap_or(false),
            };

            if try_backward {
                let before_start = idx.saturating_sub(200);
                let backward = safe_slice(text, before_start, idx);
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

fn density_pull(window: &str, rx: &Regex) -> Option<f64> {
    rx.captures(window)
        .and_then(|c| c.get(1).and_then(|m| m.as_str().parse().ok()))
}

fn scan_density(text: &str) -> Option<f64> {
    let lower = text.to_ascii_lowercase();
    let idx = lower.find("density").or_else(|| lower.find("densit"))?;
    // 200 forward / 400 backward. The wider backward window covers SUNLU's
    // official TDS where the 1.23 value sits ~270 chars above the label
    // because pdftotext extracts the data column ahead of the property
    // column.
    let forward = safe_slice(text, idx, idx + 200);
    let before_start = idx.saturating_sub(400);
    let backward = safe_slice(text, before_start, idx);

    // Prefer unit-aware matches (g/cm, kg/m). Rejects false-friends like
    // "1.5 mm" (thickness) and "1.51 ohm-cm" (resistivity) that share the
    // plausible-density numeric range.
    density_pull(forward,  &DENSITY_VALUE_WITH_UNIT_AFTER_RX)
        .or_else(|| density_pull(forward,  &DENSITY_VALUE_WITH_UNIT_BEFORE_RX))
        .or_else(|| density_pull(backward, &DENSITY_VALUE_WITH_UNIT_AFTER_RX))
        .or_else(|| density_pull(backward, &DENSITY_VALUE_WITH_UNIT_BEFORE_RX))
        // Last resort: bare value scan (only when no unit was found in the
        // surrounding window at all).
        .or_else(|| density_pull(forward,  &DENSITY_VALUE_RX))
        .or_else(|| density_pull(backward, &DENSITY_VALUE_RX))
}

fn scan_manufacturer(text: &str) -> Option<String> {
    // Prefer the first match within the first ~1000 chars (header area).
    let head = safe_slice(text, 0, 1500);
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

/// Trim trailing generic markers that don't belong in a profile name.
/// Mirror of the SDS-side helper: if stripping leaves less than 4 chars,
/// keep the original (e.g. don't reduce "PLA filament" to "PLA").
fn strip_trailing_noise(s: &str) -> String {
    static TRAIL: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\s*(?:filament|1[,.]75\s*mm|2[,.]85\s*mm|3\s*mm|spool|refill|matt|\d+\s*kg|1kg|3d)+\s*$").unwrap()
    });
    let cleaned = TRAIL.replace(s, "").trim().to_string();
    if cleaned.is_empty() || cleaned.len() < 4 {
        return s.to_string();
    }
    cleaned
}

fn scan_product_name(text: &str, manufacturer: Option<&str>) -> Option<String> {
    let head = safe_slice(text, 0, 1200);

    // Labelled form ("Product Name: PLA", "PRODUCT NAME:  FILAMENT 3D PLA Speed Matt")
    // takes precedence — it carries the explicit value.
    if let Some(c) = LABELLED_PRODUCT_NAME_RX.captures(head) {
        if let Some(m) = c.get(1) {
            let raw = m.as_str().trim().to_string();
            let cleaned = strip_trailing_noise(&raw);
            if !cleaned.is_empty() && cleaned.len() <= 60 {
                if let Some(mfr) = manufacturer {
                    if let Some(brand) = pick_brand(mfr) {
                        if !cleaned.to_ascii_lowercase().contains(&brand.to_ascii_lowercase()) {
                            return Some(format!("{} {}", brand, cleaned));
                        }
                    }
                }
                return Some(cleaned);
            }
        }
    }

    // Fallback: a short polymer-name-only line ("PLA+", "PETG" on its own row).
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
            let window = safe_slice(text, idx, idx + 200);
            for m in WORD_BOUNDED_NUM_RX.find_iter(window) {
                if let Ok(v) = m.as_str().parse::<f64>() {
                    if (30.0..200.0).contains(&v) {
                        // Skip plain integers that are obviously test-method
                        // codes (e.g. "1525" -> "152" was the original
                        // failure mode; with \b we're safe, but also skip if
                        // the value is immediately preceded by "D" or "ISO"
                        // to avoid astm-style designators).
                        let prefix_start = idx + m.start().saturating_sub(8);
                        let prefix = safe_slice(text, prefix_start, idx + m.start()).to_ascii_lowercase();
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
            let window = safe_slice(text, idx, idx + 200);
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

    // Label list order matters slightly: more specific phrases first so the
    // `find` lookup picks the longer one when both are present in the text.
    // "nozzle temp" and "plate temp" are intentional substrings — they match
    // both the abbreviated form ("Nozzle Temp.") and the long one
    // ("Nozzle temperature").
    let (n_lo, n_hi) = scan_range_with_hint(
        text,
        &["nozzle temperature", "extruder temperature", "print temperature",
          "bottom printing temperature", "température buse", "température d'impression",
          "printing temperature", "3d printing temperature",
          "nozzle temp", "extruder temp", "print temp", "printing temp"],
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
          "température plateau", "plateau chauffant", "base plate",
          "plate temp", "bed temp", "platform temp", "hot plate temp"],
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

    /// Official SUNLU PLA TDS — uses a "Product Name: PLA" labelled line
    /// (not a polymer-only line) and the density value sits BEFORE the
    /// "Density" label because pdftotext extracted the data column ahead
    /// of the property column.
    #[test]
    fn parses_sunlu_official_tds_layout() {
        let tds = r#"
TECHNICAL DATA SHEET ISO                                 Number                     SL-TE-WI-077

Product Name: PLA

Properties                               Test Method    Test Condition S.I. Units Typical Values

Thermal                                                              %             54
(X-Y) Heat Distortion (HDT)                                                        164
Glass Transition (Tg)                       ISO 306                              %  54
Melting Temperature                         ISO 294
@5%Decomposition Temp.                   ISO 11359-2                          0.1-0.3
Vicat Softening Temp.                                                          101
Moulding Shrinkage                         ISO 1133
Coefficient of Thermal Exp.                ISO 1183     190/2.16 kg         g/10 min    6.9
                                          IEC 60093          23              g/cm3      1.23
Others                                    IEC 60250
                                                             1 kHz          ohm-cm      1.51
Melt Mass-flow Rate                          UL 94                                       HB
Density                                                     1.5 mm            Class
Volume Resistivity

Recommended Printing Parameters

Parameters                                    Range

                                 Temperature                                                    Speed

Nozzle Temp.                                                                                    mm/s
                                 200-210                                                        50-100
                                 210-240                                                        100-200

Plate Temp.                                   60-70

Plate Material                   Textured PEI Build Plate

Cooling Fan                          Open/Close

Drying Temp.                                  50
"#;
        let r = parse(tds);
        assert_eq!(r.polymer, Some(Polymer::Pla));
        // Labelled "Product Name: PLA" form is now accepted in TDS scan.
        assert_eq!(r.product_name.as_deref(), Some("PLA"));
        // Density 1.23 sits BEFORE the "Density" label; backward fallback wins.
        assert_eq!(r.density_g_cm3, Some(1.23));
        // First Nozzle Temp row (200-210) is fine — second row (210-240)
        // is the high-speed variant we choose not to chase.
        assert_eq!(r.nozzle_temp_min_c, Some(200.0));
        assert_eq!(r.nozzle_temp_max_c, Some(210.0));
        assert_eq!(r.bed_temp_min_c,    Some(60.0));
        assert_eq!(r.bed_temp_max_c,    Some(70.0));
    }

    /// Regression test for the v0.1.7 → v0.1.8 fix: the ERYONE PLA+ TDS
    /// contains 9× U+2103 ℃ (3-byte UTF-8) plus U+00B0 ° (2-byte UTF-8) and
    /// fullwidth ） (U+FF09). The old &text[idx..idx+200] byte-slices panic
    /// when the +200 offset lands inside a multi-byte character, which
    /// killed the Tauri worker thread and closed the window silently.
    /// safe_slice() must accept every byte position without panicking, and
    /// the parser must extract the ranges as usual despite the multi-byte
    /// characters in the surrounding text.
    ///
    /// Layout note: the trailing ℃ unit reflects what real vendor PDFs
    /// look like AFTER pdftotext extraction (vendors put the degree
    /// glyph after the number, not between min and max). The RANGE_RX
    /// regex requires an ASCII `-` or `to` / `–` between the two
    /// numbers, so the test stays close to the wire format.
    #[test]
    fn parses_raw_unicode_temperature_tds_without_panic() {
        let tds = "Technical Data Sheet (TDS)\n\
                   PLA+\n\
                   Nozzle temperature 190-220 ℃\n\
                   Bed temperature 55-70 ℃\n\
                   Filament density 1.23 g/cm³ at 21.5°C）\n";
        assert!(looks_like_tds(tds));
        // The parse itself must not panic with "byte index N is not a char
        // boundary" on any byte position inside ℃ / ° / ） / ³.
        let r = parse(tds);
        assert_eq!(r.polymer, Some(Polymer::Pla));
        assert_eq!(r.nozzle_temp_min_c, Some(190.0));
        assert_eq!(r.nozzle_temp_max_c, Some(220.0));
        assert_eq!(r.bed_temp_min_c,    Some(55.0));
        assert_eq!(r.bed_temp_max_c,    Some(70.0));
        assert_eq!(r.density_g_cm3,     Some(1.23));
    }

    /// Full end-to-end regression on the EXACT text that `pdf-extract`
    /// produces from the real ERYONE-PLA-plus_TDS.pdf (captured via the
    /// pdf::dump_pdf_text helper). This is the file that shipped a broken
    /// profile in v0.1.8: nozzle temperature was missing because the unit
    /// glyph sits BETWEEN the numbers ("190℃-220℃"), and the manufacturer
    /// was Unknown because pdf-extract glued "TechnologyCo,.Ltd".
    ///
    /// The text is fed through normalize_unicode first, exactly as lib.rs
    /// does before calling parse(), so this exercises the real production
    /// path.
    #[test]
    fn parses_real_eryone_pdf_extract_text() {
        let raw = "aasdadsd\n 1\n Shenzhen Eryone TechnologyCo,.Ltd\n\n\
version 1.0\n08/2024\n\nTechnical DataSheet (TDS)\n\nPLA+\n\n\
The Eryone PLA+ filament is a printing material known for its exceptional toughness.\n\n\
Part I: Suggests Printing Parameters\n\n\
Parameter Set up\n\n\
Nozzle temperature 190℃-220℃\n\n\
Bed temperature 55-70℃\n\n\
Bed material glass, PEI, spring steel plate\n\n\
Bottom printing temperature 190℃-220℃\n\n\
Sealed printing Open Printing/closed printing\n\n\
Printing speed 30-100mm/s\n\n\
Drying conditions 65℃-75℃，12H\n\n\
Part II: Physical Properties of Materials\n\n\
Property Testing Method Unit Typical Value\n\n\
Density(g/cm³ at 21.5 ° C） ASTM D792 (ISO 1183, GB/T 1033) g/cm³ 1.23\n\n\
Vicat Softening Temperature(° C) ASTM D1525 (ISO 306 GB/T 1633) ℃ 54\n";
        // Production path: normalize the unicode, THEN parse.
        let text = crate::text_utils::normalize_unicode(raw);
        assert!(looks_like_tds(&text));
        let r = parse(&text);

        assert_eq!(r.polymer, Some(Polymer::Pla));
        // Nozzle 190-220 — the v0.1.8 miss. Unit glyph between the numbers
        // must no longer block the range.
        assert_eq!(r.nozzle_temp_min_c, Some(190.0));
        assert_eq!(r.nozzle_temp_max_c, Some(220.0));
        // Bed 55-70 (worked in v0.1.8, must still work).
        assert_eq!(r.bed_temp_min_c, Some(55.0));
        assert_eq!(r.bed_temp_max_c, Some(70.0));
        // Print speed 30-100 mm/s.
        assert_eq!(r.print_speed_min_mm_s, Some(30.0));
        assert_eq!(r.print_speed_max_mm_s, Some(100.0));
        // Density 1.23 g/cm³.
        assert_eq!(r.density_g_cm3, Some(1.23));
        // Vicat softening 54 °C as a glass-transition proxy.
        assert_eq!(r.glass_transition_c, Some(54.0));
        // Manufacturer detected despite the glued "TechnologyCo,.Ltd".
        assert!(
            r.manufacturer.as_deref().unwrap_or("").contains("Eryone"),
            "manufacturer was {:?}", r.manufacturer,
        );
        // Brand prefix applied to the bare "PLA+" product line.
        assert!(
            r.product_name.as_deref().unwrap_or("").starts_with("Eryone"),
            "product_name was {:?}", r.product_name,
        );
        assert!(r.product_name.as_deref().unwrap_or("").contains("PLA"));
        // With nozzle + polymer present, the profile is no longer flagged
        // for review.
        assert!(!r.needs_review);
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
