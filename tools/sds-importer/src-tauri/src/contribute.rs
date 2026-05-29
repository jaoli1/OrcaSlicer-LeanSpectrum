//! v0.4.0 — community contribution (opt-in, anonymous).
//!
//! When the user leaves the "share this sheet" checkbox ticked, the single-PDF
//! flow POSTs the *manufacturer FACTS only* of a freshly-imported filament to
//! the Maison Drabiec server, which appends them to a moderation queue. This
//! grows the shared filament database from real sheets, without ever sending
//! the PDF, file paths, or anything personal / machine-specific.
//!
//! Hard rules enforced here:
//!   * a strict key whitelist — only the manufacturer facts below are ever
//!     serialized (see [`build_payload`]); nothing else from the
//!     [`ExtractedFilament`] (source files, estimated-field flags, …) leaks,
//!   * the destination is gated through [`crate::update::is_trusted`] so a
//!     mistyped / tampered constant cannot exfiltrate data elsewhere,
//!   * ANY failure (offline, 4xx, timeout, untrusted URL) is non-fatal: the
//!     import already succeeded, so a share problem must never surface as an
//!     error. Callers get a short human reason for the log only.

use std::time::Duration;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::{update, ExtractedFilament};

/// Contribution endpoint — append-only moderation queue on the Maison Drabiec
/// HTTPS server. Must stay under [`update::is_trusted`]'s trusted prefix.
const ENDPOINT_URL: &str = "https://slicer.maisondrabiec.fr/o-8e1ff3fc4498/contribute";
/// Shared write token for the queue (server also rate-limits by IP hash).
const TOKEN: &str = "8cac4fdb94244b836e91761d54de4159b467020fe3cc8ca1";

/// One colour entry in the payload (kept tiny + factual: a name and a hex).
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ColorFact {
    pub name: String,
    pub hex: String,
}

/// The exact, whitelisted JSON body the server accepts. Building a typed struct
/// (rather than hand-rolling a `json!`) guarantees no stray field from the
/// `ExtractedFilament` can ever be serialized — the wire shape is *only* these
/// keys. Numeric facts are `Option` so absent values are simply omitted.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ContributionPayload {
    pub brand: String,
    pub label: String,
    pub base_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub density: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diameter: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nozzle_min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nozzle_max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bed_min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bed_max: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_temp: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dry_time: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturer_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision_date: Option<String>,
    pub app_version: String,
    pub submission_id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub colors: Vec<ColorFact>,
}

/// The keys the server whitelist accepts — the single source of truth shared by
/// the builder's documentation and the unit test that asserts no extra key ever
/// reaches the wire.
pub const ALLOWED_KEYS: &[&str] = &[
    "brand",
    "label",
    "base_type",
    "density",
    "diameter",
    "nozzle_min",
    "nozzle_max",
    "bed_min",
    "bed_max",
    "dry_temp",
    "dry_time",
    "manufacturer_url",
    "revision_date",
    "app_version",
    "submission_id",
    "colors",
];

/// A best-effort unique, non-cryptographic submission id as a hex string. No
/// new dependency: we fold the high-resolution clock with an atomic counter and
/// the running app version, which is plenty to de-duplicate retries in the
/// server queue (the id is *not* a security token).
fn random_submission_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    // Mix the two with a cheap splitmix64-style avalanche so consecutive calls
    // don't produce near-identical prefixes.
    let mut x = nanos ^ seq.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x = x.wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    format!("{x:016x}{nanos:016x}")
}

/// Build the whitelisted payload from an extracted filament. Pure (no I/O, no
/// network) so it is unit-testable: brand←manufacturer, label←product_name,
/// base_type←polymer, plus the manufacturer printing window / dry / url /
/// revision facts when present. `app_version` is the compiled crate version and
/// `submission_id` is freshly generated.
pub fn build_payload(ef: &ExtractedFilament) -> ContributionPayload {
    ContributionPayload {
        brand: ef.manufacturer.clone().unwrap_or_default(),
        label: ef.product_name.clone().unwrap_or_default(),
        base_type: ef
            .polymer
            .map(|p| p.as_str().to_string())
            .unwrap_or_else(|| "Other".to_string()),
        density: ef.density_g_cm3,
        // Diameter is rarely present on the ExtractedFilament; included for
        // completeness — omitted from the body when None.
        diameter: None,
        nozzle_min: ef.nozzle_temp_min_c,
        nozzle_max: ef.nozzle_temp_max_c,
        bed_min: ef.bed_temp_min_c,
        bed_max: ef.bed_temp_max_c,
        // The ExtractedFilament has no drying fields today; left None so the
        // keys are simply absent (the server treats missing facts as unknown).
        dry_temp: None,
        dry_time: None,
        manufacturer_url: ef.manufacturer_url.clone(),
        revision_date: ef.revision_date.clone(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        submission_id: random_submission_id(),
        // Colours are not trivially available from a single SDS/TDS import, so
        // none are attached here (the DB-driven path is where colours live).
        colors: Vec::new(),
    }
}

/// Defence-in-depth: assert the serialized JSON object only ever carries
/// whitelisted keys. Returns the offending key if any slips through. This makes
/// the "facts only / no PII" guarantee a checked invariant, not just a comment.
fn extra_key(payload: &ContributionPayload) -> Option<String> {
    let value = serde_json::to_value(payload).ok()?;
    let obj: &Map<String, Value> = value.as_object()?;
    obj.keys()
        .find(|k| !ALLOWED_KEYS.contains(&k.as_str()))
        .cloned()
}

/// Submit one freshly-imported filament's manufacturer facts to the community
/// queue. NON-FATAL by contract: returns `Ok(())` on a successful 204, and an
/// `Err(reason)` only so the caller can log *why* it was skipped — the caller
/// must swallow that and never fail the import.
pub fn submit(ef: &ExtractedFilament) -> crate::Result<()> {
    // Gate: the endpoint must live on the trusted Maison Drabiec server.
    if !update::is_trusted(ENDPOINT_URL) {
        return Err(crate::Error::Other(
            "refused: contribution endpoint is not on the Maison Drabiec server".into(),
        ));
    }

    let payload = build_payload(ef);

    // Never send a contribution with no identifying facts at all.
    if payload.brand.trim().is_empty() && payload.label.trim().is_empty() {
        return Err(crate::Error::Other("nothing to share (no brand/label)".into()));
    }
    // Belt-and-braces whitelist check before anything touches the network.
    if let Some(bad) = extra_key(&payload) {
        return Err(crate::Error::Other(format!(
            "internal: non-whitelisted key '{bad}' — not sending"
        )));
    }

    // Serialize ourselves and send as an explicit JSON body. `reqwest`'s
    // `.json()` helper needs the `json` feature, which this crate doesn't
    // enable — `to_string` + an explicit Content-Type keeps the dependency
    // surface unchanged.
    let body = serde_json::to_string(&payload)
        .map_err(|e| crate::Error::Other(e.to_string()))?;

    let client = reqwest::blocking::Client::builder()
        .user_agent("OptimisateurMD-Contribute")
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| crate::Error::Other(e.to_string()))?;

    let resp = client
        .post(ENDPOINT_URL)
        .header("X-MD-Token", TOKEN)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send()
        .map_err(|e| crate::Error::Other(e.to_string()))?;

    let status = resp.status();
    if status.as_u16() == 204 || status.is_success() {
        Ok(())
    } else {
        Err(crate::Error::Other(format!("server rejected (HTTP {status})")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polymer::Polymer;

    fn sample() -> ExtractedFilament {
        ExtractedFilament {
            product_name: Some("PolyTerra PLA".into()),
            manufacturer: Some("Polymaker".into()),
            manufacturer_url: Some("https://polymaker.com".into()),
            revision_date: Some("2024-01-01".into()),
            polymer: Some(Polymer::Pla),
            density_g_cm3: Some(1.31),
            nozzle_temp_min_c: Some(190.0),
            nozzle_temp_max_c: Some(230.0),
            bed_temp_min_c: Some(25.0),
            bed_temp_max_c: Some(60.0),
            // Fields that must NOT leak into the payload:
            source_files: vec!["C:/secret/path/polyterra.pdf".into()],
            estimated_fields: vec!["nozzle_temp_min_c".into()],
            ..Default::default()
        }
    }

    #[test]
    fn payload_maps_facts_from_extracted() {
        let p = build_payload(&sample());
        assert_eq!(p.brand, "Polymaker");
        assert_eq!(p.label, "PolyTerra PLA");
        assert_eq!(p.base_type, "PLA");
        assert_eq!(p.density, Some(1.31));
        assert_eq!(p.nozzle_min, Some(190.0));
        assert_eq!(p.nozzle_max, Some(230.0));
        assert_eq!(p.bed_min, Some(25.0));
        assert_eq!(p.bed_max, Some(60.0));
        assert_eq!(p.manufacturer_url.as_deref(), Some("https://polymaker.com"));
        assert_eq!(p.revision_date.as_deref(), Some("2024-01-01"));
        assert_eq!(p.app_version, env!("CARGO_PKG_VERSION"));
        assert!(!p.submission_id.is_empty());
    }

    #[test]
    fn payload_serializes_only_whitelisted_keys() {
        // A fully-populated payload's JSON object must contain no key outside
        // the server whitelist — this is the "facts only, no PII" guarantee.
        let p = build_payload(&sample());
        let value = serde_json::to_value(&p).unwrap();
        let obj = value.as_object().unwrap();
        for key in obj.keys() {
            assert!(
                ALLOWED_KEYS.contains(&key.as_str()),
                "payload leaked a non-whitelisted key: {key}"
            );
        }
        // And the private bits never appear anywhere in the serialized text.
        let text = serde_json::to_string(&p).unwrap();
        assert!(!text.contains("secret"), "file path leaked into payload");
        assert!(!text.contains("source_files"));
        assert!(!text.contains("estimated_fields"));
        assert!(extra_key(&p).is_none());
    }

    #[test]
    fn absent_facts_are_omitted_not_null() {
        // Bare extracted filament → only the always-present keys appear; the
        // optional numeric facts are omitted entirely (not serialized as null).
        let ef = ExtractedFilament {
            manufacturer: Some("eSUN".into()),
            product_name: Some("eSUN PETG".into()),
            polymer: Some(Polymer::Petg),
            ..Default::default()
        };
        let p = build_payload(&ef);
        let value = serde_json::to_value(&p).unwrap();
        let obj = value.as_object().unwrap();
        assert!(!obj.contains_key("density"));
        assert!(!obj.contains_key("nozzle_min"));
        assert!(!obj.contains_key("colors")); // empty vec is skipped
        // Required keys are always present.
        assert!(obj.contains_key("brand"));
        assert!(obj.contains_key("label"));
        assert!(obj.contains_key("base_type"));
        assert!(obj.contains_key("app_version"));
        assert!(obj.contains_key("submission_id"));
    }

    #[test]
    fn submission_ids_are_unique_and_hex() {
        let a = random_submission_id();
        let b = random_submission_id();
        assert_ne!(a, b, "consecutive submission ids must differ");
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(a.len(), 32);
    }

    #[test]
    fn endpoint_is_on_the_trusted_server() {
        // The gate in `submit` relies on this; assert the constant qualifies so
        // a future edit that breaks the prefix is caught by the test suite.
        assert!(update::is_trusted(ENDPOINT_URL));
        // A non-trusted URL would be refused.
        assert!(!update::is_trusted("https://evil.example/contribute"));
    }
}
