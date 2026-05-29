// v0.8.1 — the Tauri commands that drove this module (`crawl_catalog`,
// `import_from_urls` in lib.rs) were removed from `invoke_handler!` because
// they were unused by the frontend AND reachable from any XSS in the WebView
// without going through `fetcher::assert_public_url`. Module retained for
// future reintroduction — re-route through `assert_public_url` first if you do.
#![allow(dead_code)]

//! Catalog crawler: vendor URL -> list of SDS / TDS PDF candidates.
//!
//! Given a single HTML page URL, the crawler downloads it, extracts every
//! `<a href="*.pdf">` link (no recursion), and classifies each link by
//! document type and likely material based on the anchor text and the URL
//! itself.
//!
//! The crawler is intentionally one-page-deep. Vendor catalogs that
//! require JavaScript rendering (Shopify storefronts, vendor SPAs) are
//! handled when their HTML carries the PDF links statically — which is
//! the case for atome3d.com (ROSA3D), sunlu.com product pages,
//! eel3dshop.com and most plentymarkets-hosted vendors.

use std::time::Duration;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const MAX_PAGE_BYTES: usize = 8 * 1024 * 1024;
const MAX_LINKS:      usize = 200;
const TIMEOUT:        Duration = Duration::from_secs(10);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum DocType {
    Sds,
    Tds,
    Certificate,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub url:               String,
    pub anchor_text:       String,
    pub doc_type:          DocType,
    /// Best-effort polymer family inference from anchor text + URL.
    pub guessed_polymer:   Option<String>,
    /// Free-form name guess ("PLA Plus", "PETG Standard", "ePLA Chameleon").
    pub guessed_product:   Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrawlResult {
    pub source_url: String,
    pub entries:    Vec<CatalogEntry>,
    pub skipped:    Vec<String>, // links we discarded with a reason
}

static USER_AGENT: &str = "LeanSpectrum-SDS-Importer/0.1 (+https://github.com/jaoli1/OrcaSlicer-LeanSpectrum)";

/// Build a blocking HTTP client tuned for one-shot static pages.
fn http_client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|e| Error::Fetch(e.to_string()))
}

/// Discover SDS/TDS PDF candidates on a single vendor page.
pub fn crawl_vendor_page(url: &str) -> Result<CrawlResult> {
    let client = http_client()?;
    let resp = client.get(url).send().map_err(|e| Error::Fetch(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(Error::Fetch(format!("status {}", resp.status())));
    }
    let bytes = resp.bytes().map_err(|e| Error::Fetch(e.to_string()))?;
    if bytes.len() > MAX_PAGE_BYTES {
        return Err(Error::Fetch("vendor page too large".into()));
    }
    let html = String::from_utf8_lossy(&bytes).into_owned();
    extract_catalog(url, &html)
}

/// HTML-only extraction. Separated from `crawl_vendor_page` for testing.
pub fn extract_catalog(base_url: &str, html: &str) -> Result<CrawlResult> {
    let document = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse("a[href]").map_err(|e| Error::Other(format!("selector: {e}")))?;

    let mut entries: Vec<CatalogEntry> = Vec::new();
    let mut skipped: Vec<String>       = Vec::new();
    let mut seen_urls: std::collections::HashSet<String> = std::collections::HashSet::new();

    for el in document.select(&selector) {
        if entries.len() >= MAX_LINKS { break; }

        let href = match el.value().attr("href") { Some(h) => h.to_string(), None => continue };
        let lower_href = href.to_ascii_lowercase();
        // Quick filter: PDF only.
        if !lower_href.contains(".pdf") { continue; }

        let abs = absolutise(base_url, &href);
        // De-dup on absolute URL minus query string.
        let canonical = strip_query(&abs);
        if !seen_urls.insert(canonical.clone()) { continue; }

        let anchor_text: String = el.text().collect::<String>().trim().to_string();
        let context = format!("{} {}", anchor_text.to_ascii_lowercase(), lower_href);

        let doc_type        = classify_doc_type(&context);
        let guessed_polymer = classify_polymer(&context);
        let guessed_product = guess_product(&anchor_text);

        // Skip obviously off-topic docs.
        if doc_type == DocType::Certificate
            && !context.contains("filament")
            && !context.contains("resin")
        {
            skipped.push(format!("certificate (non-filament): {anchor_text}"));
            continue;
        }

        entries.push(CatalogEntry {
            url: abs,
            anchor_text,
            doc_type,
            guessed_polymer,
            guessed_product,
        });
    }

    Ok(CrawlResult {
        source_url: base_url.to_string(),
        entries,
        skipped,
    })
}

fn classify_doc_type(context_lower: &str) -> DocType {
    // Order matters: more specific wins.
    if context_lower.contains("msds") || context_lower.contains("safety data") ||
       context_lower.contains("fds")  || context_lower.contains("fiche de s") ||
       context_lower.contains("ghs")  || context_lower.contains("_sds")       ||
       context_lower.contains("-sds") || context_lower.contains("sds.pdf")    ||
       context_lower.contains("sicherheit") /* DE */
    {
        return DocType::Sds;
    }
    if context_lower.contains("tds")            ||
       context_lower.contains("technical data") ||
       context_lower.contains("datasheet")      ||
       context_lower.contains("fiche technique")||
       context_lower.contains("_tds")           ||
       context_lower.contains("-tds")           ||
       context_lower.contains("tds.pdf")
    {
        return DocType::Tds;
    }
    if context_lower.contains("certificat") ||
       context_lower.contains("iso 9001")   ||
       context_lower.contains("iso9001")    ||
       context_lower.contains("quality mana")
    {
        return DocType::Certificate;
    }
    DocType::Unknown
}

static POLYMER_HINT_RX: Lazy<Regex> = Lazy::new(|| {
    // Order matters: 'pa-15cf' must beat 'pa', 'pla+' must beat 'pla'.
    Regex::new(r"(?i)\b(pla\+?(?:\s*plus)?|petg\+?|abs\+?|asa|pc-?abs|pc|tpu|pa-?\d+(?:cf)?|nylon\s?\d*|hips|peek|pp|silk)\b").unwrap()
});

fn classify_polymer(context_lower: &str) -> Option<String> {
    POLYMER_HINT_RX.captures(context_lower)
        .and_then(|c| c.get(1).map(|m| canonicalise_polymer(m.as_str())))
}

fn canonicalise_polymer(s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    match lower.as_str() {
        "pla" | "pla+" | "pla plus" => "PLA".into(),
        "petg" | "petg+"            => "PETG".into(),
        "abs"  | "abs+"             => "ABS".into(),
        "asa"                       => "ASA".into(),
        "pc-abs" | "pcabs"          => "PC-ABS".into(),
        "pc"                        => "PC".into(),
        "tpu"                       => "TPU".into(),
        "silk"                      => "PLA".into(),  // silk filaments are PLA-based
        "hips"                      => "HIPS".into(),
        "pp"                        => "PP".into(),
        "peek"                      => "PEEK".into(),
        s if s.starts_with("pa") || s.starts_with("nylon") => {
            // PA6, PA12, PA-15CF...
            s.to_ascii_uppercase().replace(' ', "")
        }
        other => other.to_ascii_uppercase(),
    }
}

fn guess_product(anchor: &str) -> Option<String> {
    let cleaned = anchor.trim();
    if cleaned.is_empty() || cleaned.len() > 80 {
        return None;
    }
    // Strip obvious doc-type tokens to leave the product part.
    static DOC_NOISE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?i)\b(SDS|MSDS|TDS|FDS|GHS|CLP|SGH|Safety\s+Data\s+Sheet|Technical\s+Data\s+Sheet|Fiche\s+(?:de\s+s\S+|technique)|Datasheet)\b").unwrap()
    });
    let cleaned = DOC_NOISE.replace_all(cleaned, "").to_string();
    let cleaned = cleaned.trim_matches(|c: char| !c.is_alphanumeric()).trim().to_string();
    if cleaned.is_empty() { None } else { Some(cleaned) }
}

fn strip_query(url: &str) -> String {
    match url.find('?') {
        Some(i) => url[..i].to_string(),
        None    => url.to_string(),
    }
}

fn absolutise(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    if href.starts_with("//") {
        // Protocol-relative URL.
        let scheme = if base.starts_with("http://") { "http:" } else { "https:" };
        return format!("{scheme}{href}");
    }
    if href.starts_with('/') {
        if let Some(scheme_end) = base.find("://") {
            let after = &base[scheme_end + 3..];
            if let Some(slash) = after.find('/') {
                return format!("{}{}", &base[..scheme_end + 3 + slash], href);
            }
            return format!("{}{}", base.trim_end_matches('/'), href);
        }
        return base.to_string();
    }
    // Path-relative — append to base after trimming trailing slash.
    let cut = base.trim_end_matches('/').to_string();
    format!("{}/{}", cut, href.trim_start_matches('/'))
}

/// Download a single PDF from URL into a temporary file and return the
/// path. Caller is responsible for removing the file when done.
pub fn download_to_temp(url: &str) -> Result<std::path::PathBuf> {
    let client = http_client()?;
    let resp = client.get(url).send().map_err(|e| Error::Fetch(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(Error::Fetch(format!("status {}", resp.status())));
    }
    let bytes = resp.bytes().map_err(|e| Error::Fetch(e.to_string()))?;
    if bytes.len() > 10 * 1024 * 1024 {
        return Err(Error::Fetch("PDF too large (>10 MB)".into()));
    }
    let id = uuid_like();
    let tmp = std::env::temp_dir().join(format!("leanspectrum-{id}.pdf"));
    std::fs::write(&tmp, &bytes).map_err(|e| Error::Io(e))?;
    Ok(tmp)
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("{:016x}-{:04x}", nanos, std::process::id())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_sds_and_tds_anchor_text() {
        assert_eq!(classify_doc_type("safety data sheet pla"), DocType::Sds);
        assert_eq!(classify_doc_type("fiche de sécurité pla"), DocType::Sds);
        assert_eq!(classify_doc_type("technical data sheet petg"), DocType::Tds);
        assert_eq!(classify_doc_type("rosa3d_msds-petg-en.pdf"), DocType::Sds);
        assert_eq!(classify_doc_type("filament_3d_pla_speed.pdf"), DocType::Unknown);
    }

    #[test]
    fn classifies_polymer_hints() {
        assert_eq!(classify_polymer("pla speed matt").as_deref(),  Some("PLA"));
        assert_eq!(classify_polymer("petg standard").as_deref(),   Some("PETG"));
        assert_eq!(classify_polymer("pa-15cf carbon").as_deref(),  Some("PA-15CF"));
        assert_eq!(classify_polymer("tpu flex 85a").as_deref(),    Some("TPU"));
    }

    #[test]
    fn absolutises_various_href_forms() {
        assert_eq!(absolutise("https://example.com/a/", "/x.pdf"),
                   "https://example.com/x.pdf");
        assert_eq!(absolutise("https://example.com/a/", "x.pdf"),
                   "https://example.com/a/x.pdf");
        assert_eq!(absolutise("https://example.com/a/", "//cdn.com/x.pdf"),
                   "https://cdn.com/x.pdf");
        assert_eq!(absolutise("https://example.com/a/",
                              "https://other.com/x.pdf"),
                   "https://other.com/x.pdf");
    }

    #[test]
    fn de_duplicates_and_strips_query_strings() {
        let html = r#"
            <a href="/a/pla-sds.pdf?v=1">PLA SDS</a>
            <a href="/a/pla-sds.pdf?v=2">PLA SDS (mirror)</a>
            <a href="/a/petg-tds.pdf">PETG TDS</a>
        "#;
        let r = extract_catalog("https://vendor.example", html).unwrap();
        // 2 unique URLs after stripping query string.
        assert_eq!(r.entries.len(), 2);
    }

    #[test]
    fn extracts_atome3d_style_links() {
        let html = r#"
            <a href="https://cdn.shopify.com/s/files/1/files/ROSA3D_MSDS-PLA-Plus-ProSpeed-EN.pdf">ROSA3D MSDS PLA Plus ProSpeed</a>
            <a href="https://cdn.shopify.com/s/files/1/files/filament_3d_pla_speed.pdf">FILAMENT 3D PLA Speed Matt</a>
        "#;
        let r = extract_catalog("https://atome3d.example", html).unwrap();
        assert_eq!(r.entries.len(), 2);
        assert_eq!(r.entries[0].doc_type, DocType::Sds);
        assert_eq!(r.entries[0].guessed_polymer.as_deref(), Some("PLA"));
        // The second link has no SDS/TDS keyword in its anchor or URL — classified Unknown.
        assert_eq!(r.entries[1].doc_type, DocType::Unknown);
        assert_eq!(r.entries[1].guessed_polymer.as_deref(), Some("PLA"));
    }

    #[test]
    fn drops_iso9001_certificates() {
        let html = r#"
            <a href="/iso-9001-certificate.pdf">ISO 9001 Quality Management</a>
            <a href="/pla-sds.pdf">PLA SDS</a>
        "#;
        let r = extract_catalog("https://vendor.example", html).unwrap();
        assert_eq!(r.entries.len(), 1);
        assert!(r.skipped.iter().any(|s| s.contains("certificate")));
    }
}
