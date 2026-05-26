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
    Regex::new(r"\b(?:www\.)?[a-zA-Z0-9-]+\.(?:com|net|org|io|eu|fr|de|cn|uk|us|info|co)\b(?:/[A-Za-z0-9._/-]*)?").unwrap()
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
    // "Product Name: <value>" or "Nom du produit : <value>"
    Regex::new(r"(?im)^\s*(?:product\s*name|nom\s+du\s+produit|nom\s+commercial)\s*[:\-]\s*(.+?)\s*$").unwrap()
});

static MANUFACTURER_SUFFIX_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:Co[.,;\s]{0,3}Ltd|GmbH|S\.A\.|S\.A\.S|S\.L\.|S\.R\.L|Inc\.?|LLC|Corp\.?|Limited|B\.V\.|N\.V\.|Pty\.?\s*Ltd|AG|KG|Oy|AB|AS|sp\.?z\.?o\.?o)\b").unwrap()
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

fn parse_section_1(text: &str, out: &mut ExtractedFilament) {
    // Product name via labelled regex (preferred).
    if let Some(c) = PRODUCT_NAME_LINE_RX.captures(text) {
        if let Some(m) = c.get(1) {
            let v = m.as_str().trim().to_string();
            if !v.is_empty() {
                out.product_name = Some(v);
            }
        }
    }

    // Manufacturer: lines that contain a company-suffix anywhere, OR the
    // line immediately after a "Manufacture..." / "Fabricant" label.
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if MANUFACTURER_SUFFIX_RX.is_match(line) {
            out.manufacturer = Some(line.trim().to_string());
            break;
        }
        let lower = line.to_ascii_lowercase();
        if lower.contains("manufactur") || lower.contains("fabricant")
            || lower.contains("supplier")  || lower.contains("fournisseur")
        {
            // Strip any value after a colon on the same line.
            if let Some((_, after)) = line.split_once(':') {
                let after = after.trim();
                if !after.is_empty() && MANUFACTURER_SUFFIX_RX.is_match(after) {
                    out.manufacturer = Some(after.to_string());
                    break;
                }
            }
            // Otherwise look at the next non-empty line.
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
}
