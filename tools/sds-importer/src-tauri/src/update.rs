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
//!   "app_version": "0.1.17",
//!   "db_version":  "2026-05-28",        // ISO date — lexical compare is correct
//!   "db_url":      "https://.../filaments.sqlite",
//!   "download_url":"https://.../optimisateur-md-latest.zip",
//!   "notes":       "optional changelog blurb"
//! }

use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{Error, Result};

const MANIFEST_URL: &str =
    "https://slicer.maisondrabiec.fr/o-8e1ff3fc4498/manifest.json";

#[derive(Debug, Deserialize)]
struct Manifest {
    app_version: String,
    db_version: String,
    db_url: String,
    download_url: String,
    #[serde(default)]
    notes: String,
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

fn data_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("Optimisateur MD"))
}
fn db_path() -> Option<PathBuf> {
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
    serde_json::from_str::<Manifest>(&body)
        .map_err(|e| Error::Fetch(format!("manifest parse: {e}")))
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
    // half-written DB behind.
    let final_db = db_path().unwrap();
    let tmp = final_db.with_extension("sqlite.part");
    std::fs::write(&tmp, &bytes)?;
    std::fs::rename(&tmp, &final_db)?;
    std::fs::write(db_version_path().unwrap(), version)?;
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
        Some(v) => v.as_str() < m.db_version.as_str(),
        None => true,
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
    use super::semver_newer;
    #[test]
    fn semver_ordering() {
        assert!(semver_newer("0.1.16", "0.1.17"));
        assert!(semver_newer("0.1.16", "0.2.0"));
        assert!(semver_newer("0.9.9", "1.0.0"));
        assert!(!semver_newer("0.1.17", "0.1.17"));
        assert!(!semver_newer("0.1.17", "0.1.16"));
        assert!(!semver_newer("1.0.0", "0.9.9"));
    }
}
