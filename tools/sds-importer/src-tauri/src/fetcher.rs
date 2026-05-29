//! Optional manufacturer URL fetcher.
//!
//! Given a homepage URL extracted from the SDS, look for a single link
//! that points to a TDS PDF and return its extracted text.

use std::net::{IpAddr, ToSocketAddrs};
use std::time::Duration;

use crate::{Error, Result};

const MAX_PAGE_BYTES: usize = 5 * 1024 * 1024;
const MAX_PDF_BYTES:  usize = 5 * 1024 * 1024;
const TIMEOUT:        Duration = Duration::from_secs(8);

/// Return the text content of the manufacturer's TDS PDF if exactly one
/// plausible link is found on the homepage. Returns `Ok(None)` if no
/// match was found (no crawl).
pub fn try_fetch_tds(url: &str) -> Result<Option<String>> {
    // SSRF guard: refuse non-web schemes and any host that resolves to a
    // loopback / private / link-local / cloud-metadata address.
    assert_public_url(url)?;

    let client = reqwest::blocking::Client::builder()
        .timeout(TIMEOUT)
        .user_agent("LeanSpectrum-SDS-Importer/0.1 (+https://github.com/jaoli1/OrcaSlicer-LeanSpectrum)")
        // Cap redirects and re-check each hop so a redirect can't smuggle us
        // onto a private address.
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() > 3 {
                attempt.error("too many redirects")
            } else if assert_public_url(attempt.url().as_str()).is_err() {
                attempt.error("redirect to a non-public address")
            } else {
                attempt.follow()
            }
        }))
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
    let selector = scraper::Selector::parse("a[href]")
        .map_err(|e| Error::Other(format!("selector parse failed: {e}")))?;

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
    // The candidate link came from fetched HTML — re-check before following it.
    assert_public_url(&url)?;

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

/// Reject SSRF targets: non-web schemes, and hosts that resolve to a
/// loopback / private / link-local / unspecified address. DNS is resolved so
/// a public hostname pointing at an internal IP is also caught.
fn assert_public_url(raw: &str) -> Result<()> {
    let url = reqwest::Url::parse(raw).map_err(|e| Error::Fetch(format!("bad URL: {e}")))?;
    match url.scheme() {
        "http" | "https" => {}
        other => return Err(Error::Fetch(format!("refused non-web scheme: {other}"))),
    }
    let host = url.host_str().ok_or_else(|| Error::Fetch("URL has no host".into()))?;
    let port = url.port_or_known_default().unwrap_or(443);
    let mut any = false;
    for addr in (host, port)
        .to_socket_addrs()
        .map_err(|e| Error::Fetch(format!("DNS resolution failed: {e}")))?
    {
        any = true;
        if is_blocked_ip(&addr.ip()) {
            return Err(Error::Fetch(
                "refused a private / loopback / link-local address".into(),
            ));
        }
    }
    if !any {
        return Err(Error::Fetch("host did not resolve".into()));
    }
    Ok(())
}

fn is_blocked_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()   // 169.254/16 — incl. the cloud metadata IP
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_documentation()
                // CGNAT 100.64.0.0/10
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xc0) == 64)
        }
        IpAddr::V6(v6) => {
            v6.is_loopback()
                || v6.is_unspecified()
                || (v6.segments()[0] & 0xfe00) == 0xfc00 // unique-local fc00::/7
                || (v6.segments()[0] & 0xffc0) == 0xfe80 // link-local fe80::/10
        }
    }
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
    #[test]
    fn ssrf_blocks_loopback_private_and_schemes() {
        assert!(assert_public_url("http://127.0.0.1/x.pdf").is_err());
        assert!(assert_public_url("http://localhost/x").is_err());
        assert!(assert_public_url("http://169.254.169.254/latest/meta-data").is_err());
        assert!(assert_public_url("http://10.0.0.5/").is_err());
        assert!(assert_public_url("file:///etc/passwd").is_err());
        assert!(assert_public_url("ftp://example.com/x").is_err());
    }
    #[test]
    fn blocked_ip_classification() {
        let b = |s: &str| is_blocked_ip(&s.parse::<IpAddr>().unwrap());
        assert!(b("127.0.0.1"));
        assert!(b("192.168.1.1"));
        assert!(b("10.1.2.3"));
        assert!(b("169.254.169.254"));
        assert!(b("::1"));
        assert!(!b("8.8.8.8"));
        assert!(!b("1.1.1.1"));
    }
}
