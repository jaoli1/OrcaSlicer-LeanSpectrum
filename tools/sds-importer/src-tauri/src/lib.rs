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
mod project_process;
mod sds;
mod tds;
mod text_utils;

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
    /// Authoritative single bed temperature when the vendor states one
    /// (e.g. the "test specimen printed under … base plate 60 °C" note),
    /// preferred over the range midpoint.
    pub bed_temp_recommended_c: Option<f64>,
    pub print_speed_min_mm_s: Option<f64>,
    pub print_speed_max_mm_s: Option<f64>,
    /// Authoritative single print speed when the vendor states one (e.g. the
    /// validated "printing speed=80 mm/s" from the test-specimen note).
    pub print_speed_recommended_mm_s: Option<f64>,
    pub max_flow_mm3_s:      Option<f64>,
    pub fan_enabled:         Option<bool>,

    /// Field names whose value was estimated rather than extracted directly.
    pub estimated_fields:    Vec<String>,

    pub needs_review:        bool,
    pub source_files:        Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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

/// Run a sync command body inside `catch_unwind` so a Rust panic inside any
/// helper (regex, byte-slice, PDF parser) becomes a structured `Error` returned
/// to the JS frontend instead of crashing the Tauri worker thread and tearing
/// the WebView2 host down. This is the safety net that prevents the
/// "window closes silently when I click Import" failure mode.
fn run_command<R, F>(op: F) -> std::result::Result<R, Error>
where
    F: FnOnce() -> std::result::Result<R, Error>,
{
    use std::panic::{catch_unwind, AssertUnwindSafe};
    match catch_unwind(AssertUnwindSafe(op)) {
        Ok(r) => r,
        Err(payload) => {
            let msg = panic_message(&payload);
            log::error!("Tauri command panicked: {msg}");
            eprintln!("Tauri command panicked: {msg}");
            Err(Error::Other(format!("internal panic: {msg}")))
        }
    }
}

fn panic_message(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        return (*s).to_string();
    }
    if let Some(s) = payload.downcast_ref::<String>() {
        return s.clone();
    }
    "panic with unknown payload".to_string()
}

#[tauri::command]
fn import_pdf(req: ImportRequest) -> std::result::Result<ImportResult, Error> {
    run_command(|| import_pdf_impl(req))
}

fn import_pdf_impl(req: ImportRequest) -> std::result::Result<ImportResult, Error> {
    let path = PathBuf::from(&req.pdf_path);
    let mut log = Vec::new();

    log.push(format!("Reading {}", path.display()));
    let raw_text = pdf::extract_text(&path).unwrap_or_default();

    let raw_text = if raw_text.trim().len() < 200 {
        log.push("Direct text extraction yielded little; falling back to OCR.".to_string());
        match ocr::run(&path) {
            Ok(t) => t,
            Err(e) => {
                log.push(format!("OCR failed: {e}"));
                raw_text
            }
        }
    } else {
        log.push(format!("Extracted {} chars of native PDF text.", raw_text.len()));
        raw_text
    };

    // Normalize the unicode punctuation that vendor PDFs use interchangeably
    // with ASCII before the regex parsers run. This folds ℃→°C, ）→), en-dash→-,
    // fullwidth comma→, etc. Reduces the surface area for byte-slice surprises
    // and lets ASCII-friendly regexes match more matches without per-pattern
    // unicode classes.
    let text = text_utils::normalize_unicode(&raw_text);

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
                    let tds_text = text_utils::normalize_unicode(&tds_text);
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
    run_command(|| crawler::crawl_vendor_page(&url))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusPdf {
    pub filename:      String,
    pub absolute_path: String,
    pub size_bytes:    u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusBrand {
    pub brand: String,
    pub pdfs:  Vec<CorpusPdf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorpusIndex {
    pub root:     String,
    pub brands:   Vec<CorpusBrand>,
    pub pdf_count: usize,
}

/// Default corpus root: <user_downloads>/filament-corpus/. The frontend can
/// override.
fn default_corpus_root() -> Option<PathBuf> {
    let dl = dirs::download_dir().or_else(|| dirs::home_dir().map(|h| h.join("Downloads")))?;
    Some(dl.join("filament-corpus"))
}

#[tauri::command]
fn corpus_default_path() -> std::result::Result<String, Error> {
    default_corpus_root()
        .map(|p| p.display().to_string())
        .ok_or_else(|| Error::Other("Could not determine the user's Downloads directory.".into()))
}

#[tauri::command]
fn scan_corpus(path: String) -> std::result::Result<CorpusIndex, Error> {
    run_command(|| scan_corpus_impl(path))
}

fn scan_corpus_impl(path: String) -> std::result::Result<CorpusIndex, Error> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err(Error::Other(format!("Not a directory: {}", root.display())));
    }
    let mut brands: Vec<CorpusBrand> = Vec::new();
    let mut total = 0usize;
    for entry in std::fs::read_dir(&root)? {
        let entry = match entry { Ok(e) => e, Err(_) => continue };
        let p = entry.path();
        if !p.is_dir() { continue; }
        let brand_name = p.file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if brand_name.is_empty() || brand_name.starts_with('_') || brand_name.starts_with('.') {
            // Skip hidden / underscore-prefixed admin folders (_collection.log etc.)
            continue;
        }
        let mut pdfs: Vec<CorpusPdf> = Vec::new();
        if let Ok(dir) = std::fs::read_dir(&p) {
            collect_pdfs_into(dir, &mut pdfs);
        }
        if !pdfs.is_empty() {
            pdfs.sort_by(|a, b| a.filename.cmp(&b.filename));
            total += pdfs.len();
            brands.push(CorpusBrand { brand: brand_name, pdfs });
        }
    }
    brands.sort_by(|a, b| a.brand.cmp(&b.brand));
    Ok(CorpusIndex { root: root.display().to_string(), brands, pdf_count: total })
}

fn collect_pdfs_into(dir: std::fs::ReadDir, out: &mut Vec<CorpusPdf>) {
    for entry in dir.flatten() {
        let p = entry.path();
        if p.is_dir() {
            // One level of nesting (SUNLU-style: brand/product/*.pdf).
            if let Ok(sub) = std::fs::read_dir(&p) {
                for e2 in sub.flatten() {
                    let p2 = e2.path();
                    if p2.is_file() && p2.extension().map_or(false, |e| e.eq_ignore_ascii_case("pdf")) {
                        push_pdf(&p2, out);
                    }
                }
            }
            continue;
        }
        if p.extension().map_or(false, |e| e.eq_ignore_ascii_case("pdf")) {
            push_pdf(&p, out);
        }
    }
}

fn push_pdf(path: &std::path::Path, out: &mut Vec<CorpusPdf>) {
    let filename = path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
    if filename.is_empty() { return; }
    let size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    out.push(CorpusPdf {
        filename,
        absolute_path: path.display().to_string(),
        size_bytes,
    });
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
    run_command(|| import_from_urls_impl(req))
}

fn import_from_urls_impl(req: BatchImportRequest) -> std::result::Result<BatchImportResult, Error> {
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
                // Call the panic-safe wrapper so one bad PDF in the batch
                // doesn't take down the whole batch.
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

/// Build the path to the persistent log file:
///   Windows: %LOCALAPPDATA%\Custom Filament Profile Creator\app.log
///   macOS:   ~/Library/Application Support/Custom Filament Profile Creator/app.log
///   Linux:   ~/.local/share/Custom Filament Profile Creator/app.log
/// Falls back to `None` if no data directory can be located — logger then
/// stays on stderr.
fn log_file_path() -> Option<PathBuf> {
    let dir = dirs::data_local_dir()?.join("Custom Filament Profile Creator");
    if std::fs::create_dir_all(&dir).is_err() {
        return None;
    }
    Some(dir.join("app.log"))
}

/// Install a panic hook that appends the panic info to the log file (so a
/// user can send it back after a crash). Also keeps the original hook so
/// stderr / RUST_BACKTRACE behaviour is preserved.
fn install_panic_hook(log_path: Option<PathBuf>) {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let msg = format!(
            "[{}] PANIC: {} at {:?}",
            chrono::Local::now().to_rfc3339(),
            info.payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| info.payload().downcast_ref::<String>().map(|s| s.as_str()))
                .unwrap_or("?"),
            info.location(),
        );
        eprintln!("{msg}");
        if let Some(path) = &log_path {
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                use std::io::Write;
                let _ = writeln!(f, "{msg}");
            }
        }
        prev(info);
    }));
}

/// Initialise env_logger. Writes to the log file when one is available
/// (production), otherwise stays on stderr (dev with `cargo tauri dev`).
fn init_logger(log_path: Option<&std::path::Path>) {
    let mut builder = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    );
    if let Some(path) = log_path {
        if let Ok(file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            builder.target(env_logger::Target::Pipe(Box::new(file)));
        }
    }
    let _ = builder.try_init();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let log_path = log_file_path();
    init_logger(log_path.as_deref());
    install_panic_hook(log_path.clone());

    log::info!(
        "Custom Filament Profile Creator v{} starting (log file: {:?})",
        env!("CARGO_PKG_VERSION"),
        log_path,
    );

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            import_pdf, pick_pdf, crawl_catalog, import_from_urls,
            corpus_default_path, scan_corpus
        ])
        .setup(|app| {
            log::info!(
                "main window ready: {}",
                app.get_webview_window("main").map(|_| "main").unwrap_or("?"),
            );
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri runtime failure");
}
