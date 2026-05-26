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
    Regex::new(r"(?im)^\s*(?:section\s+)?(\d{1,2})\s*[:.\-]?\s*([A-Za-zÀ-ÿ][^\n]{2,80})$").unwrap()
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
    // "Product Name: <value>", "1.1 Product identification  <value>",
    // "Trade Name: <value>", "Nom du produit : <value>".
    Regex::new(r"(?im)^\s*(?:\d+\.\d+\s+)?(?:product\s*(?:name|identification|identifier)|trade\s*name|nom\s+du\s+produit|nom\s+commercial|nom\s+du\s+m[ée]lange)\s*[:\-]?\s+(\S.{1,80}?)\s*$").unwrap()
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

fn parse_temp_range(text: &str) -> (Option<f64>, Option<f64>) {
    if let Some(c) = TEMP_RANGE_RX.captures(text) {
        let lo: f64 = c.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
        let hi: f64 = c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
        if hi >= lo && lo >= 50.0 && hi <= 500.0 {
            return (Some(lo), Some(hi));
        }
    }
    if let Some(c) = SINGLE_TEMP_RX.captures(text) {
        if let Some(v) = c.get(1).and_then(|m| m.as_str().parse::<f64>().ok()) {
            if (50.0..=500.0).contains(&v) {
                return (Some(v), Some(v));
            }
        }
    }
    (None, None)
}

fn parse_density(text: &str) -> Option<f64> {
    // Scan for "density" / "densité" in the section and pull the first
    // plausible density value within a 200-char window.
    let lower = text.to_ascii_lowercase();
    let idx = lower.find("density").or_else(|| lower.find("densit"))?;
    let window = &text[idx..text.len().min(idx + 200)];
    DENSITY_VALUE_RX.captures(window)
        .and_then(|c| c.get(1).and_then(|m| m.as_str().parse().ok()))
}

fn next_non_empty_line<'a>(lines: &mut std::str::Lines<'a>) -> Option<&'a str> {
    lines.by_ref().map(|l| l.trim()).find(|l| !l.is_empty())
}

/// Trim trailing generic markers that don't belong in a profile name:
/// "Filament", "1.75mm", "2.85mm", "Spool", "Refill", etc.
fn strip_trailing_noise(s: &str) -> String {
    static TRAIL: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\s*(?:filament|1[,.]75\s*mm|2[,.]85\s*mm|3\s*mm|spool|refill|\d+\s*kg|1kg)+\s*$").unwrap()
    });
    let cleaned = TRAIL.replace(s, "").trim().to_string();
    if cleaned.is_empty() { s.to_string() } else { cleaned }
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
            out.manufacturer = Some(line.trim().to_string());
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
}
