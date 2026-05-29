//! v0.1.17 — update checker for the Optimisateur.
//!
//! Hits a small JSON manifest on Maison Drabiec's server and compares it to the
//! running app version + the locally-cached filament database version:
//!   - if a newer DATABASE is published → download it to the client app-data dir
//!     automatically (it is just data), and remember its version,
//!   - if a newer APP is published → report it + the download URL (the frontend
//!     proposes opening the download page; distribution is a ZIP, not a signed
//!     auto-updater, so we never silently replace the binary),
//!   - otherwise report "already up to date".
//!
//! Manifest shape (served at MANIFEST_URL):
//! {
//!   "app_version": "0.8.0",
//!   "db_version":  "2026-05-29",        // ISO date — lexical compare is correct
//!   "db_url":      "https://.../filaments.sqlite",
//!   "download_url":"https://.../optimisateur-md-latest.zip",
//!   "db_sha256":   "hex sha-256 of the DB file (informational in v0.8.0)",
//!   "notes":       "optional changelog blurb",
//!   "signature":   "base64(ed25519 sig of SIGNED_PAYLOAD_v1)"
//! }
//!
//! Signed-payload bytes (v0.8.0) — must match `sign_manifest.py` exactly:
//!
//!   SIGNED_PAYLOAD_v1 =
//!       b"v1"
//!       \x00 utf8(app_version)
//!       \x00 utf8(db_version)
//!       \x00 utf8(db_url)
//!       \x00 utf8(download_url)
//!       \x00 utf8(db_sha256)
//!       \x00 utf8(notes)
//!
//! Strict policy: from v0.8.0, an absent or invalid signature is treated as a
//! check failure (the update channel surfaces the error and does NOT fall back
//! to the unsigned data). Older clients (0.7.x) ignore the signature field
//! entirely — the schema is backward-compatible.

use std::path::PathBuf;
use std::time::Duration;

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine as _;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const MANIFEST_URL: &str =
    "https://slicer.maisondrabiec.fr/o-8e1ff3fc4498/manifest.json";

/// Update artefacts must live on the Maison Drabiec HTTPS host — guards
/// against a tampered manifest redirecting the DB download / app link elsewhere.
/// The host MUST be parsed and compared by equality (not `starts_with`), so
/// `https://slicer.maisondrabiec.fr.evil.com/` and `https://slicer.maisondrabiec.fr@evil.com/`
/// are both rejected. The scheme must be HTTPS (the manifest itself is signed,
/// but the trust check protects the URLs it carries).
const TRUSTED_HOST: &str = "slicer.maisondrabiec.fr";
pub fn is_trusted(url: &str) -> bool {
    let Ok(u) = reqwest::Url::parse(url) else { return false; };
    u.scheme() == "https"
        && u.host_str() == Some(TRUSTED_HOST)
        && u.port().is_none()
}

/// Maison Drabiec's manifest-signing ed25519 public key (32 bytes, raw).
///
/// Generated locally by `scripts/generate_signing_keypair.py` — the matching
/// private key lives ONLY on the release machine at
/// `%USERPROFILE%\.maison_drabiec\manifest_signing.ed25519` and is NEVER
/// committed, NEVER copied to the IONOS server, NEVER printed in logs.
///
/// Rotating this key requires shipping a new app version with the new bytes
/// embedded — older clients keep verifying against the previous key. Don't
/// rotate casually.
const SIGNING_PUBLIC_KEY: [u8; 32] = [
    0x54, 0x7b, 0xb8, 0xc4, 0x6c, 0x33, 0x7f, 0xa7,
    0x5c, 0x87, 0xf0, 0xfc, 0x66, 0x8c, 0x31, 0x84,
    0xea, 0x67, 0x6b, 0x58, 0x91, 0x38, 0xc3, 0xaa,
    0x4b, 0xeb, 0xf0, 0xf3, 0x9d, 0x85, 0x7b, 0x8a,
];

/// Format tag for the signed-payload bytes. Bump if the field set / order ever
/// changes, and ship a client that accepts both old and new during the cut-over.
const SIGNED_PAYLOAD_VERSION: &[u8] = b"v1";

/// True only for a strict "YYYY-MM-DD" string (the db_version format). A
/// malformed local marker must NOT win a lexical compare and silently block
/// database updates — callers treat "not a date" as "needs (re)download".
fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && s[0..4].bytes().all(|c| c.is_ascii_digit())
        && s[5..7].bytes().all(|c| c.is_ascii_digit())
        && s[8..10].bytes().all(|c| c.is_ascii_digit())
}

#[derive(Debug, Deserialize)]
struct Manifest {
    app_version: String,
    db_version: String,
    db_url: String,
    download_url: String,
    #[serde(default)]
    notes: String,
    /// SHA-256 of the served filaments.sqlite (hex). Signed but not yet verified
    /// against the downloaded bytes in v0.8.0 — a future client can add that
    /// check without changing the manifest format.
    #[serde(default)]
    db_sha256: String,
    /// base64(ed25519 signature over SIGNED_PAYLOAD_v1). Absent on legacy
    /// manifests; required from v0.8.0 onward.
    #[serde(default)]
    signature: String,
}

/// Build the deterministic byte string the signer signed. Must match
/// `build_signed_payload` in `scripts/sign_manifest.py` exactly — concat with
/// NUL separators avoids every canonical-JSON edge case (whitespace, key order,
/// unicode escapes).
fn build_signed_payload(m: &Manifest) -> Vec<u8> {
    let fields: [&str; 6] = [
        &m.app_version,
        &m.db_version,
        &m.db_url,
        &m.download_url,
        &m.db_sha256,
        &m.notes,
    ];
    // Pre-size: tag + (1 NUL + field bytes) × N
    let cap = SIGNED_PAYLOAD_VERSION.len()
        + fields.iter().map(|f| 1 + f.len()).sum::<usize>();
    let mut out = Vec::with_capacity(cap);
    out.extend_from_slice(SIGNED_PAYLOAD_VERSION);
    for f in fields {
        out.push(0);
        out.extend_from_slice(f.as_bytes());
    }
    out
}

/// Strict verification: refuse the manifest unless its `signature` field is a
/// valid ed25519 signature over the canonical SIGNED_PAYLOAD_v1 bytes, made by
/// the holder of the matching private key (embedded `SIGNING_PUBLIC_KEY`).
fn verify_signature(m: &Manifest) -> Result<()> {
    if m.signature.is_empty() {
        return Err(Error::Fetch(
            "manifest is not signed (signature field missing)".into(),
        ));
    }
    let sig_bytes = BASE64_STANDARD
        .decode(m.signature.as_bytes())
        .map_err(|e| Error::Fetch(format!("manifest signature is not valid base64: {e}")))?;
    let sig_arr: [u8; 64] = sig_bytes
        .as_slice()
        .try_into()
        .map_err(|_| Error::Fetch(format!(
            "manifest signature must be 64 bytes (got {})", sig_bytes.len()
        )))?;
    let sig = Signature::from_bytes(&sig_arr);
    // SIGNING_PUBLIC_KEY is hard-coded — only fails if the bytes are not a
    // valid ed25519 point, which is a compile-time bug, not a runtime one.
    let pk = VerifyingKey::from_bytes(&SIGNING_PUBLIC_KEY).map_err(|e| {
        Error::Fetch(format!("embedded public key is not a valid ed25519 point: {e}"))
    })?;
    let payload = build_signed_payload(m);
    // `verify_strict` (rather than `verify`) rejects signatures over low-order /
    // non-canonical R values — defense-in-depth against ed25519 malleability.
    // Cite: https://docs.rs/ed25519-dalek/2/ed25519_dalek/struct.VerifyingKey.html#method.verify_strict
    pk.verify_strict(&payload, &sig)
        .map_err(|_| Error::Fetch("manifest signature does NOT verify against the embedded public key".into()))
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStatus {
    pub current_app_version: String,
    pub latest_app_version: String,
    pub app_update_available: bool,
    pub download_url: String,
    pub current_db_version: Option<String>,
    pub latest_db_version: String,
    pub db_update_available: bool,
    pub db_downloaded: bool,
    pub up_to_date: bool,
    pub notes: String,
    /// Set when the check itself failed (offline, server down). The frontend
    /// shows it without treating it as "an update is available".
    pub error: Option<String>,
}

pub(crate) fn data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("Optimisateur MD"))
}
/// Canonical on-disk location of the filament database. `update::download_db`
/// writes here; the first-run seeder copies the bundled snapshot here; and
/// `library` reads from here. One location, three writers/readers.
pub(crate) fn db_path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("filaments.sqlite"))
}
fn db_version_path() -> Option<PathBuf> {
    data_dir().map(|d| d.join("db_version.txt"))
}

/// Version of the locally-cached database, or None if it was never downloaded.
pub fn local_db_version() -> Option<String> {
    std::fs::read_to_string(db_version_path()?)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn client() -> Result<reqwest::blocking::Client> {
    reqwest::blocking::Client::builder()
        .user_agent("OptimisateurMD-Updater")
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| Error::Fetch(e.to_string()))
}

fn fetch_manifest() -> Result<Manifest> {
    let resp = client()?
        .get(MANIFEST_URL)
        .send()
        .map_err(|e| Error::Fetch(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(Error::Fetch(format!("manifest HTTP {}", resp.status())));
    }
    let body = resp.text().map_err(|e| Error::Fetch(e.to_string()))?;
    let m: Manifest = serde_json::from_str(&body)
        .map_err(|e| Error::Fetch(format!("manifest parse: {e}")))?;
    // Strict: an absent or invalid signature is a hard failure. The check
    // surfaces it to the caller (which puts it on UpdateStatus.error) — we
    // never silently fall through to the unsigned data.
    verify_signature(&m)?;
    Ok(m)
}

/// True if dotted-numeric `candidate` is strictly newer than `current`
/// (e.g. "0.1.17" > "0.1.16"). Missing components count as 0.
fn semver_newer(current: &str, candidate: &str) -> bool {
    let parse = |s: &str| {
        s.split(|c| c == '.' || c == '-')
            .map(|x| x.trim().parse::<u64>().unwrap_or(0))
            .collect::<Vec<_>>()
    };
    let (cur, cand) = (parse(current), parse(candidate));
    for i in 0..cur.len().max(cand.len()) {
        let x = cur.get(i).copied().unwrap_or(0);
        let y = cand.get(i).copied().unwrap_or(0);
        if y != x {
            return y > x;
        }
    }
    false
}

fn download_db(url: &str, version: &str) -> Result<()> {
    if !is_trusted(url) {
        return Err(Error::Fetch(
            "refused: the database URL is not on the Maison Drabiec server".into(),
        ));
    }
    let dir = data_dir().ok_or_else(|| Error::Other("no app-data directory".into()))?;
    std::fs::create_dir_all(&dir)?;
    let bytes = client()?
        .get(url)
        .send()
        .map_err(|e| Error::Fetch(e.to_string()))?
        .error_for_status()
        .map_err(|e| Error::Fetch(e.to_string()))?
        .bytes()
        .map_err(|e| Error::Fetch(e.to_string()))?;
    // Sanity: a real SQLite file starts with this 16-byte magic header. Guards
    // against saving an HTML error page or a truncated download as the DB.
    if bytes.len() < 16 || &bytes[..16] != b"SQLite format 3\0" {
        return Err(Error::Fetch(
            "downloaded database is not a valid SQLite file (server returned something else)".into(),
        ));
    }
    // Write to a temp sibling then rename, so a crash mid-write never leaves a
    // half-written DB behind. PID + nanos in the temp name so two concurrent
    // update-checks (Tauri commands run on a thread-pool) don't clobber each
    // other's `.part` and end up renaming a partial file on Windows
    // (ERROR_SHARING_VIOLATION) or, worse, silently writing junk.
    let final_db = db_path().ok_or_else(|| Error::Other("no app-data directory".into()))?;
    let vpath = db_version_path().ok_or_else(|| Error::Other("no app-data directory".into()))?;
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp = final_db.with_extension(format!("sqlite.{}.{nanos}.part", std::process::id()));
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, &final_db)?;
    std::fs::write(vpath, version)?;
    Ok(())
}

/// Check for updates. When `download_db_if_newer` is set and the server DB is
/// newer (or no local DB exists), the database is fetched automatically.
pub fn check(download_db_if_newer: bool) -> Result<UpdateStatus> {
    let mut st = UpdateStatus {
        current_app_version: env!("CARGO_PKG_VERSION").to_string(),
        current_db_version: local_db_version(),
        ..Default::default()
    };
    let m = match fetch_manifest() {
        Ok(m) => m,
        Err(e) => {
            st.error = Some(e.to_string());
            return Ok(st);
        }
    };
    st.latest_app_version = m.app_version.clone();
    st.download_url = m.download_url.clone();
    st.latest_db_version = m.db_version.clone();
    st.notes = m.notes.clone();

    st.app_update_available = semver_newer(&st.current_app_version, &m.app_version);
    // db_version is an ISO date string → lexical compare orders it correctly.
    st.db_update_available = match &st.current_db_version {
        Some(v) if is_iso_date(v) => v.as_str() < m.db_version.as_str(),
        _ => true, // no local DB, or a malformed version marker → (re)download
    };

    if st.db_update_available && download_db_if_newer {
        match download_db(&m.db_url, &m.db_version) {
            Ok(()) => {
                st.db_downloaded = true;
                st.current_db_version = Some(m.db_version.clone());
            }
            Err(e) => st.error = Some(format!("Database download failed: {e}")),
        }
    }
    st.up_to_date = !st.app_update_available && !st.db_update_available && st.error.is_none();
    Ok(st)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_ordering() {
        assert!(semver_newer("0.1.16", "0.1.17"));
        assert!(semver_newer("0.1.16", "0.2.0"));
        assert!(semver_newer("0.9.9", "1.0.0"));
        assert!(!semver_newer("0.1.17", "0.1.17"));
        assert!(!semver_newer("0.1.17", "0.1.16"));
        assert!(!semver_newer("1.0.0", "0.9.9"));
    }

    /// Build the exact fixture that scripts/sign_manifest.py signed (the test
    /// fixture lives in this test module — there is no on-disk artefact).
    fn fixture() -> Manifest {
        Manifest {
            app_version: "0.8.0-test".into(),
            db_version: "2026-05-29".into(),
            db_url: "https://slicer.maisondrabiec.fr/o-8e1ff3fc4498/filaments.sqlite".into(),
            download_url: "https://slicer.maisondrabiec.fr/o-8e1ff3fc4498/optimisateur-md-latest.zip".into(),
            notes: "test fixture — vérifie que la chaîne signature ↔ vérification est end-to-end (é, €, em-dash)".into(),
            db_sha256: String::new(),
            signature: String::new(),
        }
    }

    #[test]
    fn signed_payload_is_deterministic_and_byte_for_byte() {
        // The byte-string format MUST match scripts/sign_manifest.py exactly,
        // or signatures produced by the deploy script won't verify here. Lock
        // it down with a literal byte expectation (NULs included).
        let m = fixture();
        let got = build_signed_payload(&m);
        let mut want = Vec::new();
        want.extend_from_slice(b"v1");
        want.push(0); want.extend_from_slice(b"0.8.0-test");
        want.push(0); want.extend_from_slice(b"2026-05-29");
        want.push(0); want.extend_from_slice(b"https://slicer.maisondrabiec.fr/o-8e1ff3fc4498/filaments.sqlite");
        want.push(0); want.extend_from_slice(b"https://slicer.maisondrabiec.fr/o-8e1ff3fc4498/optimisateur-md-latest.zip");
        want.push(0); // db_sha256 empty
        want.push(0); want.extend_from_slice(
            "test fixture — vérifie que la chaîne signature ↔ vérification est end-to-end (é, €, em-dash)"
                .as_bytes(),
        );
        assert_eq!(got, want);
    }

    /// Signature produced offline by `python scripts/sign_manifest.py` against
    /// the fixture above, using the production private key whose matching
    /// public key is hard-coded as `SIGNING_PUBLIC_KEY`. The test proves:
    ///   1. The embedded public key is the right pair for the deploy private key.
    ///   2. The Python signer and Rust verifier agree on every byte of the
    ///      signed payload (UTF-8 of unicode chars, NUL separators, "v1" tag).
    /// If you ever rotate the signing keypair, regenerate this string.
    const FIXTURE_SIGNATURE: &str =
        "GL5LgUHinNhBfjMlfTVJfZCy9+6xFJVeWQlOwkqQD2wmVDUb/R9vyTGTnJaOISYJr+BnpbImsJ6b0kLOStT8Aw==";

    #[test]
    fn verify_accepts_a_real_signature_from_the_deploy_script() {
        let mut m = fixture();
        m.signature = FIXTURE_SIGNATURE.into();
        verify_signature(&m).expect("signature must verify against the embedded public key");
    }

    #[test]
    fn verify_refuses_an_unsigned_manifest() {
        // Strict policy: an absent signature is a hard failure — the update
        // channel must not silently fall back to the unsigned data.
        let m = fixture();
        let err = verify_signature(&m).unwrap_err().to_string();
        assert!(err.contains("not signed"), "unexpected error: {err}");
    }

    #[test]
    fn verify_refuses_a_tampered_manifest() {
        // Flip a single byte in any signed field → ed25519 verification fails.
        let mut m = fixture();
        m.signature = FIXTURE_SIGNATURE.into();
        m.notes.push_str(" TAMPERED");
        let err = verify_signature(&m).unwrap_err().to_string();
        assert!(err.contains("signature") && err.contains("does NOT verify"),
            "unexpected error: {err}");
    }

    #[test]
    fn verify_refuses_garbage_base64() {
        let mut m = fixture();
        m.signature = "not-base-64-!@#$".into();
        assert!(verify_signature(&m).is_err());
    }
}
