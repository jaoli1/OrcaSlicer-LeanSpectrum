//! SDS parser.
//!
//! Splits the document into the standardised 16-section structure (ISO
//! 11014-1 / GHS) and pulls the fields LeanSpectrum needs from the
//! relevant sections.

use once_cell::sync::Lazy;
use regex::Regex;

use crate::{polymer, ExtractedFilament};

static SECTION_HEADING: Lazy<Regex> = Lazy::new(|| {
    // Match lines like:
    //   "SECTION 9: PHYSICAL AND CHEMICAL PROPERTIES"
    //   "9. Propriétés physiques et chimiques"
    //   "9.Propriétés..." (no space — eSUN SDS does this)
    //   "Section 1 - Identification"
    //   "Section 3 –Composition" (en-dash, SUNLU)
    //   "Section 6 —Accidental Release" (em-dash)
    // The separator class includes ASCII : . - plus Unicode en-dash (\u{2013})
    // and em-dash (\u{2014}); the regex crate accepts these as literal chars.
    Regex::new(r"(?im)^\s*(?:section\s+)?(\d{1,2})\s*[:.\-\u{2013}\u{2014}]?\s*([A-Za-zÀ-ÿ][^\n]{2,80})$").unwrap()
});

static URL_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"https?://[A-Za-z0-9._~:/?#\[\]@!$&'()*+,;=%-]+").unwrap()
});

static BARE_DOMAIN_RX: Lazy<Regex> = Lazy::new(|| {
    // Catches "www.example.com" or "example.com/path" without a scheme.
    // Added: .pl (ROSA3D / Polish vendors), and a few common EU TLDs.
    Regex::new(r"\b(?:www\.)?[a-zA-Z0-9-]+\.(?:com|net|org|io|eu|fr|de|cn|uk|us|info|co|pl|it|es|be|ch|at|se|no|dk|pt|nl|jp|kr|ca|au|cz|hu|ro|tr|biz)\b(?:/[A-Za-z0-9._/-]*)?").unwrap()
});

static DATE_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:20\d{2}-\d{2}-\d{2}|\d{2}/\d{2}/20\d{2}|\d{2}\.\d{2}\.20\d{2}|20\d{2}\.\d{2}\.\d{2})").unwrap()
});

static TEMP_RANGE_RX: Lazy<Regex> = Lazy::new(|| {
    // Trailing "°C" is now optional; the plausibility check in the caller
    // ((50, 500) °C) filters spurious matches. Accepts "180-200",
    // "180 - 200", "180 to 200", "180 à 220".
    Regex::new(r"(?i)(\d{2,3}(?:\.\d+)?)\s*(?:-|to|–|à|au)\s*(\d{2,3}(?:\.\d+)?)\s*(?:°\s*c)?").unwrap()
});

static SINGLE_TEMP_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(\d{2,3}(?:\.\d+)?)\s*(?:°\s*c)?").unwrap()
});

static DENSITY_VALUE_RX: Lazy<Regex> = Lazy::new(|| {
    // First plausible density (0.5..1.7 g/cm^3) in the scanned window.
    Regex::new(r"(0\.[5-9]\d?|1\.[0-7]\d?)").unwrap()
});

static PRODUCT_NAME_LINE_RX: Lazy<Regex> = Lazy::new(|| {
    // Accepts:
    //   "Product Name: <value>"        (colon + space, common)
    //   "Product Name:<value>"         (colon, no space — eSun SDS)
    //   "1.1 Product identification  <value>"  (subsection + double space)
    //   "Trade Name: <value>"
    //   "Nom du produit : <value>"
    // [\s:\-]+ in place of \s*[:\-]?\s+ so the separator can be any combination
    // of space, colon, or dash with no surrounding whitespace requirement.
    Regex::new(r"(?im)^\s*(?:\d+\.\d+\s+)?(?:product\s*(?:name|identification|identifier)|trade\s*name|nom\s+du\s+produit|nom\s+commercial|nom\s+du\s+m[ée]lange)[\s:\-]+(\S.{1,80}?)\s*$").unwrap()
});

static MANUFACTURER_SUFFIX_RX: Lazy<Regex> = Lazy::new(|| {
    // Polish "Sp. z o.o" with optional spaces and dots between each char,
    // alongside the global legal-form set already supported.
    Regex::new(r"(?i)\b(?:Co[.,;\s]{0,3}Ltd|GmbH|S\.A\.S?|S\.L\.|S\.R\.L|Inc\.?|LLC|Corp\.?|Limited|B\.V\.|N\.V\.|Pty\.?\s*Ltd|AG|KG|Oy|AB|A/S|Sp\.?\s*z\.?\s*o\.?\s*o\.?|s\.r\.o\.?)\b").unwrap()
});

#[derive(Debug, Default)]
struct Sections<'a> {
    by_number: [Option<&'a str>; 17], // 1..=16, index 0 unused
}

fn split_sections(text: &str) -> Sections<'_> {
    let mut sections = Sections::default();
    let bounds: Vec<(usize, usize)> = SECTION_HEADING
        .captures_iter(text)
        .filter_map(|c| {
            let n: usize = c.get(1)?.as_str().parse().ok()?;
            if (1..=16).contains(&n) {
                Some((n, c.get(0)?.start()))
            } else {
                None
            }
        })
        .collect();

    for (i, &(n, start)) in bounds.iter().enumerate() {
        let end = bounds.get(i + 1).map(|&(_, e)| e).unwrap_or(text.len());
        sections.by_number[n] = Some(&text[start..end]);
    }
    sections
}

fn parse_temp_range_with_floor(text: &str, floor: f64, ceiling: f64) -> (Option<f64>, Option<f64>) {
    if let Some(c) = TEMP_RANGE_RX.captures(text) {
        let lo: f64 = c.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
        let hi: f64 = c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
        if hi >= lo && lo >= floor && hi <= ceiling {
            return (Some(lo), Some(hi));
        }
    }
    if let Some(c) = SINGLE_TEMP_RX.captures(text) {
        if let Some(v) = c.get(1).and_then(|m| m.as_str().parse::<f64>().ok()) {
            if (floor..=ceiling).contains(&v) {
                return (Some(v), Some(v));
            }
        }
    }
    (None, None)
}

/// Default floor (50 °C) matches melting / decomposition / glass-transition
/// expectations. Bed-temp / print-temp callers use a lower floor.
fn parse_temp_range(text: &str) -> (Option<f64>, Option<f64>) {
    parse_temp_range_with_floor(text, 50.0, 500.0)
}

static DENSITY_VALUE_WITH_UNIT_AFTER_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(0\.[5-9]\d?|1\.[0-7]\d?)\s*(?:g\s*/\s*cm\d*|kg\s*/\s*m\d*)").unwrap()
});
static DENSITY_VALUE_WITH_UNIT_BEFORE_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:g\s*/\s*cm\d*|kg\s*/\s*m\d*)\s*[^\d\n]{0,40}(0\.[5-9]\d?|1\.[0-7]\d?)").unwrap()
});

fn density_pull(window: &str, rx: &Regex) -> Option<f64> {
    rx.captures(window)
        .and_then(|c| c.get(1).and_then(|m| m.as_str().parse().ok()))
}

fn parse_density(text: &str) -> Option<f64> {
    let lower = text.to_ascii_lowercase();
    let idx = lower.find("density").or_else(|| lower.find("densit"))?;
    // Forward window stays at 200 chars (most SDS print density on the
    // same line as the label). Backward window is wider (400 chars) for
    // reverse-column TDS layouts where the value sits several lines
    // above the label.
    let forward = &text[idx..text.len().min(idx + 200)];
    let before_start = idx.saturating_sub(400);
    let backward = &text[before_start..idx];

    // Prefer unit-aware matches (g/cm, kg/m). They reject false friends
    // like "1.5 mm" (thickness) or "1.51 ohm-cm" (resistivity) that share
    // the plausible-density numeric range.
    density_pull(forward,  &DENSITY_VALUE_WITH_UNIT_AFTER_RX)
        .or_else(|| density_pull(forward,  &DENSITY_VALUE_WITH_UNIT_BEFORE_RX))
        .or_else(|| density_pull(backward, &DENSITY_VALUE_WITH_UNIT_AFTER_RX))
        .or_else(|| density_pull(backward, &DENSITY_VALUE_WITH_UNIT_BEFORE_RX))
        // Fallback: bare value scan (only when no unit is anywhere nearby).
        .or_else(|| density_pull(forward,  &DENSITY_VALUE_RX))
        .or_else(|| density_pull(backward, &DENSITY_VALUE_RX))
}

fn next_non_empty_line<'a>(lines: &mut std::str::Lines<'a>) -> Option<&'a str> {
    lines.by_ref().map(|l| l.trim()).find(|l| !l.is_empty())
}

/// Trim trailing generic markers that don't belong in a profile name:
/// "Filament", "1.75mm", "2.85mm", "Spool", "Refill", etc.
/// If stripping leaves a string shorter than 4 chars (e.g. just a polymer
/// abbreviation like "ABS" or "PLA"), keep the original so the saved
/// profile carries the more recognisable "ABS filament" form instead of
/// the generic "ABS".
fn strip_trailing_noise(s: &str) -> String {
    static TRAIL: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\s*(?:filament|1[,.]75\s*mm|2[,.]85\s*mm|3\s*mm|spool|refill|\d+\s*kg|1kg)+\s*$").unwrap()
    });
    let cleaned = TRAIL.replace(s, "").trim().to_string();
    if cleaned.is_empty() || cleaned.len() < 4 {
        return s.to_string();
    }
    cleaned
}

fn parse_section_1(text: &str, out: &mut ExtractedFilament) {
    // Product name via labelled regex (preferred). Strip trailing
    // "filament" / "1.75mm" noise so the saved profile carries a clean
    // name.
    if let Some(c) = PRODUCT_NAME_LINE_RX.captures(text) {
        if let Some(m) = c.get(1) {
            let v = m.as_str().trim().to_string();
            if !v.is_empty() {
                out.product_name = Some(strip_trailing_noise(&v));
            }
        }
    }

    // Manufacturer extraction. Three patterns we accept, in order:
    //   1. A line containing one of the known legal suffixes (Co Ltd, GmbH,
    //      Sp. z o.o, …). Highest precedence — most likely the full name.
    //   2. A "Supplier:" / "Manufacturer:" / "Fabricant:" labelled line with
    //      a non-empty value AFTER the colon (same line). No suffix required
    //      because real-world supplier values often lack the suffix word.
    //   3. The same labelled line but with the value on the NEXT non-empty
    //      line.
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if MANUFACTURER_SUFFIX_RX.is_match(line) {
            // Strip a leading "<Label>:" prefix when the value carrying the
            // legal suffix sits after a colon ("Manufacture/Supplier : Zhuhai
            // SUNLU Industrial Co., Ltd.").
            let trimmed = line.trim();
            let value = if let Some((_, after)) = trimmed.split_once(':') {
                let after = after.trim();
                if !after.is_empty() && MANUFACTURER_SUFFIX_RX.is_match(after) {
                    after.to_string()
                } else {
                    trimmed.to_string()
                }
            } else {
                trimmed.to_string()
            };
            out.manufacturer = Some(value);
            break;
        }
        let trimmed = line.trim();
        let lower_trim = trimmed.to_ascii_lowercase();
        // Only treat lines that START with a known label as field labels.
        // Subsection headers like "1.3 Details of the supplier of …" must
        // not be confused with the actual "Supplier:" line below them.
        let is_label_line =
            lower_trim.starts_with("manufactur") ||
            lower_trim.starts_with("supplier")   ||
            lower_trim.starts_with("fabricant")  ||
            lower_trim.starts_with("fournisseur");
        if is_label_line {
            if let Some((_, after)) = trimmed.split_once(':') {
                let after = after.trim();
                if !after.is_empty() {
                    out.manufacturer = Some(after.to_string());
                    break;
                }
            }
            if let Some(next) = next_non_empty_line(&mut lines) {
                out.manufacturer = Some(next.to_string());
                break;
            }
        }
    }

    // URL: scheme-aware first, then bare-domain fallback.
    if let Some(m) = URL_RX.find(text) {
        out.manufacturer_url = Some(m.as_str().trim_end_matches(|c: char| !c.is_alphanumeric()).to_string());
    } else if let Some(m) = BARE_DOMAIN_RX.find(text) {
        out.manufacturer_url = Some(format!("https://{}",
            m.as_str().trim_start_matches("www.").trim_end_matches(|c: char| !c.is_alphanumeric())
        ));
    }
}

pub fn parse(text: &str) -> ExtractedFilament {
    let sections = split_sections(text);

    let mut out = ExtractedFilament::default();
    out.polymer = polymer::detect(text);

    if let Some(s1) = sections.by_number[1] {
        parse_section_1(s1, &mut out);
    }

    if let Some(s9) = sections.by_number[9] {
        out.density_g_cm3 = parse_density(s9);

        let scan = |label_patterns: &[&str]| -> Option<(f64, f64)> {
            for pat in label_patterns {
                if let Some(idx) = s9.to_ascii_lowercase().find(&pat.to_ascii_lowercase()) {
                    let snippet = &s9[idx..s9.len().min(idx + 160)];
                    let (lo, hi) = parse_temp_range(snippet);
                    if lo.is_some() || hi.is_some() {
                        return Some((lo.unwrap_or(hi.unwrap_or(0.0)), hi.unwrap_or(lo.unwrap_or(0.0))));
                    }
                }
            }
            None
        };

        if let Some((lo, hi)) = scan(&["melting", "fusion"]) {
            out.melt_temp_min_c = Some(lo);
            out.melt_temp_max_c = Some(hi);
        }
        if let Some((lo, _hi)) = scan(&["decomposition", "décomposition"]) {
            out.decomposition_c = Some(lo);
        }
        if let Some((lo, _hi)) = scan(&["glass transition", "transition vitreuse",
                                        "vicat softening", "vicat"]) {
            out.glass_transition_c = Some(lo);
        }

        // SUNLU and other modern vendor SDSes carry TDS-style recommendations
        // inside Section 9 ("Print Temp 210-235", "Bed Temp 60-80"). Pick them
        // up when present; downstream consumers prefer measured values over
        // polymer-family defaults.
        let scan_low = |label_patterns: &[&str], floor: f64, ceiling: f64| -> Option<(f64, f64)> {
            for pat in label_patterns {
                if let Some(idx) = s9.to_ascii_lowercase().find(&pat.to_ascii_lowercase()) {
                    let snippet = &s9[idx..s9.len().min(idx + 160)];
                    let (lo, hi) = parse_temp_range_with_floor(snippet, floor, ceiling);
                    if lo.is_some() || hi.is_some() {
                        return Some((lo.unwrap_or(hi.unwrap_or(0.0)), hi.unwrap_or(lo.unwrap_or(0.0))));
                    }
                }
            }
            None
        };
        if let Some((lo, hi)) = scan_low(&["print temp", "nozzle temp", "extruder temp",
                                            "printing temp", "extrusion temp"],
                                          120.0, 450.0) {
            out.nozzle_temp_min_c = Some(lo);
            out.nozzle_temp_max_c = Some(hi);
            out.nozzle_temp_recommended_c = Some((lo + hi) / 2.0);
        }
        if let Some((lo, hi)) = scan_low(&["bed temp", "platform temp",
                                            "heated bed", "build plate temp",
                                            "hot plate temp"],
                                          25.0, 200.0) {
            out.bed_temp_min_c = Some(lo);
            out.bed_temp_max_c = Some(hi);
        }
    }

    // Density fallback to polymer default.
    if out.density_g_cm3.is_none() {
        out.density_g_cm3 = out.polymer.and_then(|p| p.default_density_g_cm3());
        if out.density_g_cm3.is_some() {
            out.estimated_fields.push("density_g_cm3".into());
        }
    }

    if let Some(s16) = sections.by_number[16] {
        out.revision_date = DATE_RX.find(s16).map(|m| m.as_str().to_string());
    }
    if out.revision_date.is_none() {
        out.revision_date = DATE_RX.find(text).map(|m| m.as_str().to_string());
    }

    out.language = detect_language(text);
    out.needs_review = out.polymer.is_none() ||
        (out.melt_temp_max_c.is_none() && out.decomposition_c.is_none());
    out
}

fn detect_language(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    let fr_hits = ["fiche", "données", "sécurité", "composition", "produit", "fabricant"]
        .iter().filter(|w| lower.contains(*w)).count();
    let en_hits = ["safety", "data", "sheet", "composition", "product", "manufacturer"]
        .iter().filter(|w| lower.contains(*w)).count();
    if fr_hits > en_hits + 1 { Some("fr".into()) }
    else if en_hits > 0     { Some("en".into()) }
    else                    { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polymer::Polymer;

    #[test]
    fn empty_input_yields_empty_result() {
        let r = parse("");
        assert!(r.product_name.is_none());
        assert!(r.polymer.is_none());
        assert!(r.needs_review);
    }

    #[test]
    fn detects_section_9_melting_range_with_unit() {
        let sds = "SECTION 9: Physical and Chemical Properties\nMelting point: 175 to 195 °C\n";
        let r = parse(sds);
        assert_eq!(r.melt_temp_min_c, Some(175.0));
        assert_eq!(r.melt_temp_max_c, Some(195.0));
    }

    /// Regression test built from a real-world eSUN ABS MSDS.
    /// Covers seven previously-broken cases at once:
    ///  - "7.Handling" / "9.Physical" with no space after the section number
    ///  - Polymer detected via CAS 9003-56-9
    ///  - Section 9 "Melting point: 180-200" with no °C suffix
    ///  - Section 9 "Density at 25 : 1.05G/cm3" with intermediate digits
    ///  - "Co;Ltd" manufacturer suffix (semicolon variant)
    ///  - "Manufacture/Supplier:" label followed by the company name on the
    ///    next line
    ///  - Product name on a "Product Name:ABS filament" line
    ///  - Bare-domain manufacturer URL (no http scheme)
    #[test]
    fn parses_esun_style_sds_layout() {
        let sds = r#"
                       Material Safety data sheet

1. Identification

    Product Name:ABS filament
    Manufacture/Supplier:
    Shenzhen eSUN Industrial Co;Ltd
    More Information:
    www.brightcn.net

2. Composition

Chemical Character:                CAS NO.    Content%
  Ingredient Name                  9003-56-9  97%
  Acrylonitrile-butadiene-styrene             3%

9. Physical and Chemical Properties

Form: Solid
Melting point: 180-200
Decomposition Temperature: Not determined
Density at 25 : 1.05G/cm3
"#;
        let r = parse(sds);
        assert_eq!(r.polymer, Some(Polymer::Abs));
        assert_eq!(r.product_name.as_deref(), Some("ABS filament"));
        assert!(r.manufacturer.as_deref().unwrap_or("").contains("eSUN"));
        assert_eq!(r.manufacturer_url.as_deref(), Some("https://brightcn.net"));
        assert_eq!(r.melt_temp_min_c, Some(180.0));
        assert_eq!(r.melt_temp_max_c, Some(200.0));
        assert_eq!(r.density_g_cm3, Some(1.05));
    }

    #[test]
    fn french_label_product_name() {
        let sds = "1. Identification\nNom du produit : eFlex TPU bleu\n";
        let r = parse(sds);
        assert_eq!(r.product_name.as_deref(), Some("eFlex TPU bleu"));
    }

    /// ROSA3D MSDS layout — labels and values are on the same line with
    /// many spaces between them, subsection headings precede the actual
    /// "Supplier:" line, the URL is a bare .pl domain, and the product
    /// name carries trailing "Filament" / "1.75mm" noise.
    #[test]
    fn parses_rosa3d_msds_layout() {
        let sds = r#"
MATERIAL SAFETY DATA SHEET
          PLA Plus ProSpeed

Made on: 18/02/2021         Updated on:

SECTION 1: Product and Company identification

1.1 Product identification  PLA Plus ProSpeed Filament
                            FILAMENT 3D PLA Plus ProSpeed 1,75mm
  Product Name:
 Trade Name:

1.2 Relevant identified uses of the substance or mixture and uses advised against

Identified uses:            Thermal processing of 3D printing

1.3 Details of the supplier of the safety data sheet

Supplier:                   Przedsibiorstwo Handlowo-Produkcyjne
                            ,,Rosa" Alicja Sakowicz-Soldatke
                            ul. Hipolitowska 102, 05-074 Halinów-Hipolitów
                            tel.: +48 22 783 62 62, www.rosa3d.pl

SECTION 3: Composition/information on ingredients

PLA (Polylactide Resin) - >85% CAS: 9051-89-2

SECTION 9: Physical and chemical properties

Density: Not determined
Decomposition temperature: Not determined
Melting point: Not determined
"#;
        let r = parse(sds);
        assert_eq!(r.polymer, Some(Polymer::Pla));
        // strip_trailing_noise removes "Filament" from the captured name.
        assert_eq!(r.product_name.as_deref(), Some("PLA Plus ProSpeed"));
        // Subsection header "1.3 Details of the supplier..." must NOT win
        // over the "Supplier:" line below it.
        assert_eq!(
            r.manufacturer.as_deref(),
            Some("Przedsibiorstwo Handlowo-Produkcyjne"),
        );
        // .pl TLD is supported and the bare-domain URL is promoted to https.
        assert_eq!(r.manufacturer_url.as_deref(), Some("https://rosa3d.pl"));
    }

    /// SUNLU SDS layout. Notable real-world quirks all in a single fixture:
    ///   - "Section 3 –Composition" uses an en-dash, not "-".
    ///   - "Manufacture/Supplier : Zhuhai SUNLU Industrial Co., Ltd." mixes
    ///     a colon-labelled prefix with a legal-suffix-bearing value.
    ///   - Section 9 carries TDS-style "Print Temp" and "Bed Temp" values
    ///     directly, not just melt/decomposition.
    #[test]
    fn parses_sunlu_sds_layout() {
        let sds = r#"
Safety Data Sheet(SDS)

Product name: PETG Filament         According to GHS
Revision Date: 2021.5.15

Section 1 - Identification of the substance/preparation and of the company/undertaking

Product identifier

Product name: PETG filament

Details of the supplier of the safety data sheet

Manufacture/Supplier : Zhuhai SUNLU Industrial Co., Ltd.

Address:                  Room 501C, Building 2 No.35 Jinzhou Road
Tel:                                                 (086) 0756 3385639
E-mail :                  jk@sunlugw.com

Section 3 –Composition/Information on Ingredients

Ingredient Name    CAS No.                         EC No.  Content (%)
   PETG          25640-14-6                            --      100

Section 9 - Physical and Chemical
Properties Information on basic physical
and chemical properties
Form                                      Solid
Melting Range (°C)                        No data
Decomposition Temp (°C)                   No data
Print Temp (°C)                           220-250
Bed Temp(°C)                              60-80
Density(g/cm3)                            1.23
"#;
        let r = parse(sds);
        assert_eq!(r.polymer, Some(Polymer::Petg));
        // Manufacturer extracted with the "Manufacture/Supplier :" prefix
        // stripped via the new suffix-match branch.
        assert_eq!(r.manufacturer.as_deref(), Some("Zhuhai SUNLU Industrial Co., Ltd."));
        // Section 3 must be detected through the en-dash separator.
        // Print/Bed values come from Section 9's vendor-extended fields.
        assert_eq!(r.nozzle_temp_min_c, Some(220.0));
        assert_eq!(r.nozzle_temp_max_c, Some(250.0));
        assert_eq!(r.bed_temp_min_c,    Some(60.0));
        assert_eq!(r.bed_temp_max_c,    Some(80.0));
        assert_eq!(r.density_g_cm3,     Some(1.23));
    }
}
