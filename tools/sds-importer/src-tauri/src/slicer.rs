//! v0.3.0 — slicer selection + adaptive output path.
//!
//! Generalises the old `profile::snapmaker_orca_user_dir()` (which hard-coded
//! "Snapmaker_Orca") to ANY OrcaSlicer-family slicer. Given a slicer key
//! (`orca` | `bambu` | `creality` | `snapmaker` | `custom`) the resolver returns
//! the directory that should receive the generated `filament/` and `process/`
//! subfolders — applying the same "pick the most-recently-modified profile id
//! under `user/`" logic the slicer itself uses to decide which login the presets
//! belong to.
//!
//! Per-OS base directory (matches the previous code):
//!   - Windows + macOS: `dirs::data_dir()`
//!     (Windows `%AppData%\Roaming`, macOS `~/Library/Application Support`)
//!   - Linux: `dirs::config_dir()` (`~/.config`)
//!
//! AppName segment under that base, per slicer key (verified May 2026):
//!   - "orca"      → "OrcaSlicer"
//!   - "bambu"     → "BambuStudio"
//!   - "snapmaker" → "Snapmaker_Orca"
//!   - "creality"  → "Creality/Creality Print"   (nested; see below)
//!
//! Creality Print is an OrcaSlicer fork but does NOT keep its user presets
//! directly under `<base>/<AppName>/user/<id>`. The current versions (6.x / 7.x)
//! nest them as `<base>/Creality/Creality Print/<version>/user/<USERID>/…`
//! (sources cited in the v0.3.0 report: Creality community forum + wiki). To
//! stay robust against the version segment, the resolver searches for a `user/`
//! directory at the app base OR up to two levels below it (covering the version
//! folder, and Creality's deeper `Creative3D/<v>/server_1/orca/user` layout on
//! older installs), then applies the most-recent-id logic inside it.

use std::fs;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// A selected slicer. Parsed from the frontend's `slicer` string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Slicer {
    Orca,
    Bambu,
    Creality,
    Snapmaker,
    /// User-supplied absolute folder path.
    Custom(String),
}

impl Slicer {
    /// Parse the frontend key. `custom` requires the `custom_dir` argument.
    pub fn parse(key: &str, custom_dir: Option<&str>) -> Result<Slicer> {
        match key {
            "orca" => Ok(Slicer::Orca),
            "bambu" => Ok(Slicer::Bambu),
            "creality" => Ok(Slicer::Creality),
            "snapmaker" => Ok(Slicer::Snapmaker),
            "custom" => {
                let dir = custom_dir
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| {
                        Error::Profile(
                            "Choose a destination folder for the custom slicer option.".into(),
                        )
                    })?;
                Ok(Slicer::Custom(dir.to_string()))
            }
            other => Err(Error::Profile(format!(
                "Unknown slicer '{other}' (expected orca | bambu | creality | snapmaker | custom)."
            ))),
        }
    }

    /// Human-readable name used in error messages.
    pub fn display_name(&self) -> &str {
        match self {
            Slicer::Orca => "OrcaSlicer",
            Slicer::Bambu => "BambuStudio",
            Slicer::Creality => "Creality Print",
            Slicer::Snapmaker => "Snapmaker_Orca",
            Slicer::Custom(_) => "the chosen folder",
        }
    }

    /// The AppName path segment under the per-OS base directory. `None` for the
    /// custom slicer (which carries its own absolute base).
    fn app_segment(&self) -> Option<&str> {
        match self {
            Slicer::Orca => Some("OrcaSlicer"),
            Slicer::Bambu => Some("BambuStudio"),
            Slicer::Snapmaker => Some("Snapmaker_Orca"),
            // Creality nests the app under a vendor folder.
            Slicer::Creality => Some("Creality/Creality Print"),
            Slicer::Custom(_) => None,
        }
    }
}

/// Per-OS base directory the slicers store their data under.
fn os_base_dir() -> Option<PathBuf> {
    if cfg!(target_os = "linux") {
        dirs::config_dir()
    } else {
        // Windows + macOS.
        dirs::data_dir()
    }
}

/// The application base directory for a slicer (before the `user/` logic), e.g.
/// `<data_dir>/OrcaSlicer` or the custom absolute path. Exposed for testing the
/// AppName-segment mapping per key without touching the filesystem.
pub fn app_base_dir(slicer: &Slicer) -> Option<PathBuf> {
    match slicer {
        Slicer::Custom(path) => Some(PathBuf::from(path)),
        _ => {
            let seg = slicer.app_segment()?;
            // `seg` may contain a '/' (Creality) — join each component so the
            // platform separator is used.
            let mut base = os_base_dir()?;
            for part in seg.split('/') {
                base = base.join(part);
            }
            Some(base)
        }
    }
}

/// Given a directory that contains a `user/` subdir, return the most-recently
/// modified profile-id directory under it (the slicer picks presets from the
/// most recent login). Returns `None` if `user/` is absent or empty.
fn most_recent_user_id(dir: &Path) -> Option<PathBuf> {
    let user_dir = dir.join("user");
    if !user_dir.is_dir() {
        return None;
    }
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for entry in fs::read_dir(&user_dir).ok()?.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if let Ok(modified) = meta.modified() {
                match best {
                    Some((t, _)) if t >= modified => {}
                    _ => best = Some((modified, p)),
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Search `base` and up to `max_depth` nested levels for a directory that holds
/// a `user/` subdir, then return its most-recent profile-id directory. This
/// covers slicers that keep `user/` directly under the app folder (OrcaSlicer /
/// BambuStudio / Snapmaker_Orca) AND Creality Print, which nests it under a
/// version folder (`Creality Print/7.0/user/…`) or deeper. The shallowest match
/// wins; among equally-shallow candidates the one whose `user/` was modified
/// most recently wins (the active install).
fn find_user_id_dir(base: &Path, max_depth: usize) -> Option<PathBuf> {
    // BFS by depth so a direct `<base>/user/<id>` beats a nested one.
    let mut frontier = vec![base.to_path_buf()];
    for _ in 0..=max_depth {
        // Among all dirs at this depth, prefer the one with the most-recently
        // modified `user/` folder.
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        for dir in &frontier {
            let user_dir = dir.join("user");
            if user_dir.is_dir() {
                let modified = fs::metadata(&user_dir)
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                match &best {
                    Some((t, _)) if *t >= modified => {}
                    _ => best = Some((modified, dir.clone())),
                }
            }
        }
        if let Some((_, dir)) = best {
            if let Some(id) = most_recent_user_id(&dir) {
                return Some(id);
            }
        }
        // Descend one level.
        let mut next = Vec::new();
        for dir in &frontier {
            if let Ok(rd) = fs::read_dir(dir) {
                for entry in rd.flatten() {
                    let p = entry.path();
                    if p.is_dir() && p.file_name().map_or(true, |n| n != "user") {
                        next.push(p);
                    }
                }
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    None
}

/// Resolve the directory that will receive the generated `filament/` and
/// `process/` subfolders, for the chosen slicer.
///
/// For the named slicers: `<base>/<AppName>` then the most-recent `user/<id>`
/// logic. For `custom`: if the supplied path contains a `user/` subdir, apply
/// the most-recent-id logic; otherwise the supplied path itself is the target
/// directory. Returns `None` only when a named slicer's profile folder does not
/// exist yet (the caller turns that into a "open <slicer> once" message).
pub fn resolve_output_dir(slicer: &Slicer) -> Option<PathBuf> {
    match slicer {
        Slicer::Custom(path) => {
            let base = PathBuf::from(path);
            // Honour a `user/` layout if the user pointed us at a slicer base;
            // otherwise treat the path itself as the destination directory.
            if base.join("user").is_dir() {
                most_recent_user_id(&base).or(Some(base))
            } else {
                Some(base)
            }
        }
        _ => {
            let base = app_base_dir(slicer)?;
            // Depth 2 covers Creality's version segment (and a touch deeper for
            // its older Creative3D layout) while keeping OrcaSlicer / Bambu /
            // Snapmaker_Orca (depth 0) fast and exact.
            find_user_id_dir(&base, 2)
        }
    }
}

/// Resolve the output dir or produce a friendly, slicer-specific error telling
/// the user to open the slicer once so it creates its profile folder.
pub fn resolve_output_dir_or_err(slicer: &Slicer) -> Result<PathBuf> {
    resolve_output_dir(slicer).ok_or_else(|| {
        Error::Profile(format!(
            "{} profile folder not found — open {} once so it creates its profile folder, then retry.",
            slicer.display_name(),
            slicer.display_name(),
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// The AppName segment per slicer key is the verified set. The custom path
    /// is honoured verbatim as the base.
    #[test]
    fn app_segment_mapping_per_key() {
        // Named slicers carry a fixed AppName segment under the per-OS base.
        for (key, seg) in [
            ("orca", "OrcaSlicer"),
            ("bambu", "BambuStudio"),
            ("snapmaker", "Snapmaker_Orca"),
            ("creality", "Creality/Creality Print"),
        ] {
            let s = Slicer::parse(key, None).unwrap();
            let base = app_base_dir(&s).expect("a base on every supported OS");
            // The resolved base must end with the expected AppName component(s),
            // joined with the platform separator.
            let expected_tail: PathBuf = seg.split('/').collect();
            assert!(
                base.ends_with(&expected_tail),
                "key {key}: base {base:?} should end with {expected_tail:?}"
            );
        }
        // Custom path is the base, verbatim.
        let custom = Slicer::parse("custom", Some("/tmp/my slicer")).unwrap();
        assert_eq!(app_base_dir(&custom), Some(PathBuf::from("/tmp/my slicer")));
        // Custom without a path is an error.
        assert!(Slicer::parse("custom", None).is_err());
        assert!(Slicer::parse("custom", Some("   ")).is_err());
        // Unknown key is an error.
        assert!(Slicer::parse("prusa", None).is_err());
    }

    /// Custom path WITHOUT a `user/` subdir → the path itself is the output dir.
    #[test]
    fn custom_path_without_user_is_used_directly() {
        let tmp = std::env::temp_dir().join(format!("sds_custom_{}", std::process::id()));
        let _ = fs::create_dir_all(&tmp);
        let s = Slicer::parse("custom", Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(resolve_output_dir(&s), Some(tmp.clone()));
        let _ = fs::remove_dir_all(&tmp);
    }

    /// Custom path WITH a `user/<id>` layout → the most-recent id dir is used.
    #[test]
    fn custom_path_with_user_layout_picks_recent_id() {
        let tmp = std::env::temp_dir().join(format!("sds_custom_user_{}", std::process::id()));
        let id_dir = tmp.join("user").join("12345");
        let _ = fs::create_dir_all(&id_dir);
        let s = Slicer::parse("custom", Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(resolve_output_dir(&s), Some(id_dir));
        let _ = fs::remove_dir_all(&tmp);
    }

    /// A named slicer's base with a nested version folder (Creality-style) is
    /// resolved by the depth-limited `user/` search.
    #[test]
    fn nested_version_folder_is_found() {
        let tmp = std::env::temp_dir().join(format!("sds_nested_{}", std::process::id()));
        let id_dir = tmp.join("7.0").join("user").join("USER42");
        let _ = fs::create_dir_all(&id_dir);
        assert_eq!(find_user_id_dir(&tmp, 2), Some(id_dir));
        let _ = fs::remove_dir_all(&tmp);
    }
}
