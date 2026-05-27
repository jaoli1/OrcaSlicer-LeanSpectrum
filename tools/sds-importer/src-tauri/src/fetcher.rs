//! Optional manufacturer URL fetcher.
//!
//! Given a homepage URL extracted from the SDS, look for a single link
//! that points to a TDS PDF and return its extracted text.

use std::time::Duration;

use crate::{Error, Result};

const MAX_PAGE_BYTES: usize = 5 * 1024 * 1024;
const MAX_PDF_BYTES:  usize = 5 * 1024 * 1024;
const TIMEOUT:        Duration = Duration::from_secs(8);

/// Return the text content of the manufacturer's TDS PDF if exactly one
/// plausible link is found on the homepage. Returns `Ok(None)` if no
/// match was found (no crawl).
pub fn try_fetch_tds(url: &str) -> Result<Option<String>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .user_agent("LeanSpectrum-SDS-Importer/0.1 (+https://github.com/jaoli1/OrcaSlicer-LeanSpectrum)")
        .build()
        .map_err(|e| Error::Fetch(e.to_string()))?;

    let resp = client.get(url).send().map_err(|e| Error::Fetch(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(Error::Fetch(format!("status {}", resp.status())));
    }
    let bytes = resp.bytes().map_err(|e| Error::Fetch(e.to_string()))?;
    if bytes.len() > MAX_PAGE_BYTES {
        return Err(Error::Fetch("manufacturer page too large".into()));
    }
    let html = String::from_utf8_lossy(&bytes);

    let document = scraper::Html::parse_document(&html);
    let selector = scraper::Selector::parse("a[href]").unwrap();

    let mut candidate: Option<String> = None;
    for el in document.select(&selector) {
        let href = match el.value().attr("href") { Some(h) => h, None => continue };
        let lower_href = href.to_ascii_lowercase();
        if !lower_href.ends_with(".pdf") { continue; }
        let anchor = el.text().collect::<String>().to_ascii_lowercase();
        let likely = ["tds", "technical", "datasheet", "fiche technique"]
            .iter().any(|w| anchor.contains(w) || lower_href.contains(w));
        if likely {
            candidate = Some(absolutise(url, href));
            break;
        }
    }
    let url = match candidate { Some(u) => u, None => return Ok(None) };

    let resp = client.get(&url).send().map_err(|e| Error::Fetch(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(Error::Fetch(format!("TDS download status {}", resp.status())));
    }
    let bytes = resp.bytes().map_err(|e| Error::Fetch(e.to_string()))?;
    if bytes.len() > MAX_PDF_BYTES {
        return Err(Error::Fetch("TDS PDF too large".into()));
    }

    let text = pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| Error::Pdf(e.to_string()))?;
    Ok(Some(text))
}

fn absolutise(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    let cut = if href.starts_with('/') {
        // root-relative — strip path from base
        if let Some(scheme_end) = base.find("://") {
            let after = &base[scheme_end + 3..];
            if let Some(slash) = after.find('/') {
                return format!("{}{}", &base[..scheme_end + 3 + slash], href);
            }
        }
        base.to_string()
    } else {
        base.trim_end_matches('/').to_string()
    };
    format!("{}/{}", cut.trim_end_matches('/'), href.trim_start_matches('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_url_unchanged() {
        assert_eq!(absolutise("https://example.com", "https://other.com/x.pdf"), "https://other.com/x.pdf");
    }
    #[test]
    fn root_relative_joined() {
        assert_eq!(absolutise("https://example.com/a/b", "/docs/x.pdf"), "https://example.com/docs/x.pdf");
    }
}
