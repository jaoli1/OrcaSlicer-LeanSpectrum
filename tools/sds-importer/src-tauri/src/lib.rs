//! LeanSpectrum SDS / TDS importer.
//!
//! Library crate that exposes the Tauri command surface. Tests live next to
//! each module.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::Manager;

mod crawler;
mod fetcher;
mod ocr;
mod pdf;
mod polymer;
mod profile;
mod sds;
mod tds;

pub use crawler::{CatalogEntry, CrawlResult, DocType};
pub use polymer::Polymer;
pub use profile::{FilamentProfile, RecommendedProcess};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("PDF parse error: {0}")]
    Pdf(String),
    #[error("OCR error: {0}")]
    Ocr(String),
    #[error("Web fetch error: {0}")]
    Fetch(String),
    #[error("Profile write error: {0}")]
    Profile(String),
    #[error("Unsupported file: {0}")]
    Unsupported(String),
    #[error("Tauri error: {0}")]
    Tauri(#[from] tauri::Error),
    #[error("Other: {0}")]
    Other(String),
}

impl serde::Serialize for Error {
    // Use the std path explicitly: the crate-local `Result<T>` alias below
    // is one-arg and would shadow the trait's two-arg `Result<Ok, Err>`.
    fn serialize<S: serde::Serializer>(&self, ser: S) -> std::result::Result<S::Ok, S::Error> {
        ser.serialize_str(self.to_string().as_str())
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Aggregated data extracted from one or two PDFs (SDS + optional TDS).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractedFilament {
    pub product_name:        Option<String>,
    pub manufacturer:        Option<String>,
    pub manufacturer_url:    Option<String>,
    pub revision_date:       Option<String>,
    pub language:            Option<String>,

    pub polymer:             Option<Polymer>,
    pub density_g_cm3:       Option<f64>,
    pub glass_transition_c:  Option<f64>,
    pub melt_temp_min_c:     Option<f64>,
    pub melt_temp_max_c:     Option<f64>,
    pub decomposition_c:     Option<f64>,

    // From TDS only (or estimated from SDS).
    pub nozzle_temp_min_c:   Option<f64>,
    pub nozzle_temp_max_c:   Option<f64>,
    pub nozzle_temp_recommended_c: Option<f64>,
    pub bed_temp_min_c:      Option<f64>,
    pub bed_temp_max_c:      Option<f64>,
    pub print_speed_min_mm_s: Option<f64>,
    pub print_speed_max_mm_s: Option<f64>,
    pub max_flow_mm3_s:      Option<f64>,
    pub fan_enabled:         Option<bool>,

    /// Field names whose value was estimated rather than extracted directly.
    pub estimated_fields:    Vec<String>,

    pub needs_review:        bool,
    pub source_files:        Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    pub pdf_path: String,
    pub fetch_online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportResult {
    pub extracted: ExtractedFilament,
    pub profile_path: Option<String>,
    pub recommended_process: Option<RecommendedProcess>,
    pub log: Vec<String>,
}

#[tauri::command]
fn import_pdf(req: ImportRequest) -> std::result::Result<ImportResult, Error> {
    let path = PathBuf::from(&req.pdf_path);
    let mut log = Vec::new();

    log.push(format!("Reading {}", path.display()));
    let text = pdf::extract_text(&path).unwrap_or_default();

    let text = if text.trim().len() < 200 {
        log.push("Direct text extraction yielded little; falling back to OCR.".to_string());
        match ocr::run(&path) {
            Ok(t) => t,
            Err(e) => {
                log.push(format!("OCR failed: {e}"));
                text
            }
        }
    } else {
        log.push(format!("Extracted {} chars of native PDF text.", text.len()));
        text
    };

    // Try SDS parsing first; if a TDS-like header dominates, switch.
    let (mut extracted, kind) = if tds::looks_like_tds(&text) {
        log.push("Document classified as TDS.".to_string());
        (tds::parse(&text), "tds")
    } else {
        log.push("Document classified as SDS.".to_string());
        (sds::parse(&text), "sds")
    };
    extracted.source_files.push(path.display().to_string());

    if req.fetch_online {
        if let Some(url) = extracted.manufacturer_url.clone() {
            log.push(format!("Looking for an additional datasheet at {url}"));
            match fetcher::try_fetch_tds(&url) {
                Ok(Some(tds_text)) => {
                    log.push(format!("Found additional datasheet ({} chars).", tds_text.len()));
                    let extra = tds::parse(&tds_text);
                    profile::merge(&mut extracted, extra);
                }
                Ok(None) => log.push("No additional TDS link found on the manufacturer page.".to_string()),
                Err(e)   => log.push(format!("Online fetch failed: {e}")),
            }
        }
    }

    // Estimate missing temperatures from polymer family + melting range
    // when the file gave us a SDS only.
    if kind == "sds" {
        profile::estimate_missing_temperatures(&mut extracted, &mut log);
    }

    let (profile_path, recommended) = profile::build_and_save(&extracted, &mut log)?;
    Ok(ImportResult {
        extracted,
        profile_path: profile_path.map(|p| p.display().to_string()),
        recommended_process: recommended,
        log,
    })
}

#[tauri::command]
fn crawl_catalog(url: String) -> std::result::Result<CrawlResult, Error> {
    crawler::crawl_vendor_page(&url)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchImportRequest {
    pub urls:         Vec<String>,
    pub fetch_online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchImportResult {
    pub succeeded: Vec<ImportResult>,
    pub failed:    Vec<(String, String)>,    // (url, error)
}

#[tauri::command]
fn import_from_urls(req: BatchImportRequest) -> std::result::Result<BatchImportResult, Error> {
    let mut succeeded = Vec::with_capacity(req.urls.len());
    let mut failed    = Vec::new();
    for url in &req.urls {
        match crawler::download_to_temp(url) {
            Err(e) => failed.push((url.clone(), format!("download: {e}"))),
            Ok(path) => {
                let sub = ImportRequest {
                    pdf_path: path.display().to_string(),
                    fetch_online: req.fetch_online,
                };
                match import_pdf(sub) {
                    Ok(r)  => succeeded.push(r),
                    Err(e) => failed.push((url.clone(), e.to_string())),
                }
                let _ = std::fs::remove_file(&path);
            }
        }
    }
    Ok(BatchImportResult { succeeded, failed })
}

#[tauri::command]
fn pick_pdf(app: tauri::AppHandle) -> std::result::Result<Option<String>, Error> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("PDF", &["pdf"])
        .pick_file(move |f| {
            let _ = tx.send(f.and_then(|p| p.into_path().ok()));
        });
    let path = rx.recv().map_err(|e| Error::Other(e.to_string()))?;
    Ok(path.map(|p| p.display().to_string()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![import_pdf, pick_pdf, crawl_catalog, import_from_urls])
        .setup(|app| {
            log::info!("LeanSpectrum SDS Importer starting; main window id = {}", app.get_webview_window("main").map(|_| "main").unwrap_or("?"));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri runtime failure");
}
