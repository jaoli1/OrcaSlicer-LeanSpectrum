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
    //   "Section 1 - Identification"
    Regex::new(r"(?im)^\s*(?:section\s+)?(\d{1,2})\s*[:.\-]?\s*([A-Za-zÀ-ÿ][^\n]{2,80})$").unwrap()
});

static URL_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"https?://[A-Za-z0-9._~:/?#\[\]@!$&'()*+,;=%-]+").unwrap()
});

static DATE_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(20\d{2}-\d{2}-\d{2}|\d{2}/\d{2}/20\d{2})\b").unwrap()
});

static TEMP_RANGE_RX: Lazy<Regex> = Lazy::new(|| {
    // Catches values like "200-220 °C", "190 to 220 ºC", "approx. 210°C"
    Regex::new(r"(?i)(\d{2,3})\s*(?:-|to|–|à|au)\s*(\d{2,3})\s*°?\s*c").unwrap()
});

static SINGLE_TEMP_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(\d{2,3}(?:\.\d+)?)\s*°?\s*c").unwrap()
});

static DENSITY_RX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:density|densit[ée])[^\d]{0,30}(\d\.\d{1,3})").unwrap()
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

fn first_match(rx: &Regex, hay: &str) -> Option<String> {
    rx.captures(hay).and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn parse_temp_range(text: &str) -> (Option<f64>, Option<f64>) {
    if let Some(c) = TEMP_RANGE_RX.captures(text) {
        let lo: f64 = c.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
        let hi: f64 = c.get(2).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
        if hi >= lo && lo > 50.0 && hi < 500.0 {
            return (Some(lo), Some(hi));
        }
    }
    if let Some(s) = first_match(&SINGLE_TEMP_RX, text) {
        if let Ok(v) = s.parse::<f64>() {
            if (50.0..500.0).contains(&v) {
                return (Some(v), Some(v));
            }
        }
    }
    (None, None)
}

pub fn parse(text: &str) -> ExtractedFilament {
    let sections = split_sections(text);

    let mut out = ExtractedFilament::default();
    out.polymer = polymer::detect(text);
    if let Some(density) = first_match(&DENSITY_RX, text).and_then(|s| s.parse().ok()) {
        out.density_g_cm3 = Some(density);
    } else {
        out.density_g_cm3 = out.polymer.and_then(|p| p.default_density_g_cm3());
        if out.density_g_cm3.is_some() {
            out.estimated_fields.push("density_g_cm3".into());
        }
    }

    if let Some(s1) = sections.by_number[1] {
        if let Some(url) = URL_RX.find(s1) {
            out.manufacturer_url = Some(url.as_str().trim_end_matches(|c: char| !c.is_alphanumeric()).to_string());
        }
        // Product name: heuristic — first non-empty line after the section
        // heading that is longer than 3 chars and looks like a name.
        out.product_name = s1
            .lines()
            .skip(1)
            .map(|l| l.trim())
            .find(|l| l.len() > 3 && l.chars().any(|c| c.is_alphabetic()))
            .map(|l| l.to_string());
        out.manufacturer = s1
            .lines()
            .map(|l| l.trim())
            .find(|l| l.to_ascii_lowercase().contains("manufactur") || l.to_ascii_lowercase().contains("fabricant"))
            .map(|l| l.to_string());
    }

    if let Some(s9) = sections.by_number[9] {
        // Section 9 — physical & chemical properties. We hunt for
        // "melting", "decomposition", "glass transition" labels.
        let scan = |label_patterns: &[&str]| -> Option<(f64, f64)> {
            for pat in label_patterns {
                if let Some(idx) = s9.to_ascii_lowercase().find(&pat.to_ascii_lowercase()) {
                    // Look at the next 120 chars for the temperature.
                    let snippet = &s9[idx..s9.len().min(idx + 120)];
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
        if let Some((lo, _hi)) = scan(&["glass transition", "transition vitreuse"]) {
            out.glass_transition_c = Some(lo);
        }
    }

    if let Some(s16) = sections.by_number[16] {
        out.revision_date = DATE_RX.find(s16).map(|m| m.as_str().to_string());
    }

    out.language = detect_language(text);
    out.needs_review = out.polymer.is_none() || (out.melt_temp_max_c.is_none() && out.decomposition_c.is_none());
    out
}

fn detect_language(text: &str) -> Option<String> {
    let fr_hits = ["fiche", "données", "sécurité", "composition", "produit", "fabricant"]
        .iter().filter(|w| text.to_ascii_lowercase().contains(*w)).count();
    let en_hits = ["safety", "data", "sheet", "composition", "product", "manufacturer"]
        .iter().filter(|w| text.to_ascii_lowercase().contains(*w)).count();
    if fr_hits > en_hits + 1 { Some("fr".into()) }
    else if en_hits > 0     { Some("en".into()) }
    else                    { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_yields_empty_result() {
        let r = parse("");
        assert!(r.product_name.is_none());
        assert!(r.polymer.is_none());
        assert!(r.needs_review);
    }

    #[test]
    fn detects_section_9_melting_range() {
        let sds = "SECTION 9: Physical and Chemical Properties\nMelting point: 175 to 195 °C\nDensity 1.24\n";
        let r = parse(sds);
        assert_eq!(r.melt_temp_min_c, Some(175.0));
        assert_eq!(r.melt_temp_max_c, Some(195.0));
        assert_eq!(r.density_g_cm3,   Some(1.24));
    }
}
