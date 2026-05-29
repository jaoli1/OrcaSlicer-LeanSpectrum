//! LeanSpectrum SDS / TDS importer.
//!
//! Library crate that exposes the Tauri command surface. Tests live next to
//! each module.
#![recursion_limit = "256"] // the project_process `json!` process objects are large

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::Manager;

mod catalog;
mod contribute;
mod crawler;
mod fetcher;
mod library;
mod ocr;
mod pdf;
mod polymer;
mod profile;
mod project_process;
mod sds;
mod slicer;
mod tds;
mod text_utils;
mod update;

pub use crawler::{CatalogEntry, CrawlResult, DocType};
pub use polymer::Polymer;
pub use profile::{FilamentProfile, RecommendedProcess};
pub use slicer::Slicer;

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
    /// v0.3.0 — destination slicer (orca|bambu|creality|snapmaker|custom) and,
    /// for `custom`, the absolute output folder. The selected printer (vendor /
    /// model / nozzle) drives both the filament parent and the 7 project-type
    /// process profiles, so the single-PDF flow is consistent with the one-click
    /// flow. All optional so older callers / the batch path still deserialize.
    #[serde(default)]
    pub slicer: Option<String>,
    #[serde(default)]
    pub custom_dir: Option<String>,
    #[serde(default)]
    pub vendor: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub nozzle: Option<f64>,
    #[serde(default)]
    pub all_nozzles: bool,
    /// v0.4.0 — opt-in, anonymous community contribution. When true, the
    /// manufacturer FACTS of a freshly-imported filament are sent to the shared
    /// database queue *after* the profiles are written. v0.6.0: defaults to
    /// FALSE (true opt-in / GDPR) — the UI checkbox is unticked by default and
    /// callers must set `share: true` on purpose. Never blocks or fails import.
    #[serde(default = "default_share")]
    pub share: bool,
}

/// `share` defaults to FALSE — true opt-in: a contribution only happens when
/// the user explicitly ticks the box (or a caller sets `share: true`).
fn default_share() -> bool {
    false
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub extracted: ExtractedFilament,
    pub profile_path: Option<String>,
    pub recommended_process: Option<RecommendedProcess>,
    /// v0.3.0 — the single-PDF flow now also writes the shared 7 project-type
    /// process profiles (like the one-click flow); these surface the count + dir.
    #[serde(default)]
    pub process_count: usize,
    #[serde(default)]
    pub process_dir: Option<String>,
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

    let polymer = extracted.polymer.unwrap_or(Polymer::Other);

    // Resolve the destination slicer. Default to Snapmaker_Orca when the caller
    // sent nothing (older frontends / the batch path), preserving the previous
    // behaviour of targeting the U1.
    let slicer = Slicer::parse(
        req.slicer.as_deref().unwrap_or("snapmaker"),
        req.custom_dir.as_deref(),
    )?;

    // Resolve the chosen printer target(s). When the caller named a printer we
    // generate the filament parent + the 7 project-type process for it; without
    // one we fall back to the Snapmaker U1 (the historical single-PDF target).
    let specs: Vec<project_process::PrinterSpec> = match (&req.vendor, &req.model) {
        (Some(v), Some(m)) if !v.is_empty() && !m.is_empty() => {
            resolve_specs(v, m, req.nozzle, req.all_nozzles)?
        }
        _ => vec![catalog::resolve("Snapmaker", "Snapmaker U1", Some(0.4)).unwrap_or(
            project_process::PrinterSpec {
                printer_name: "Snapmaker U1 (0.4 nozzle)".to_string(),
                base_process: "0.20 Standard @Snapmaker U1 (0.4 nozzle)".to_string(),
                nozzle: 0.4,
                max_layer_height: 0.30,
            },
        )],
    };
    let mut printers: Vec<String> = Vec::new();
    for s in &specs {
        if !printers.contains(&s.printer_name) {
            printers.push(s.printer_name.clone());
        }
    }
    let is_u1 = printers.iter().any(|p| p.contains("Snapmaker U1"));

    // Try to resolve the slicer output dir; on failure (slicer never opened) we
    // fall back to writing next to the source PDF so the user still gets files.
    let (filament_dir, process_dir, dir_note): (PathBuf, PathBuf, Option<String>) =
        match slicer::resolve_output_dir(&slicer) {
            Some(base) => (base.join("filament"), base.join("process"), None),
            None => {
                let fallback = path
                    .parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
                (
                    fallback.clone(),
                    fallback,
                    Some(format!(
                        "{} profile folder not found; saving the profiles next to the source PDF instead.",
                        slicer.display_name()
                    )),
                )
            }
        };
    if let Some(note) = dir_note {
        log.push(note);
    }

    // 1) Filament profile (same builder as the one-click flow → consistent name
    //    and parent selection).
    let (fname, fval) =
        profile::build_filament_json_for(&extracted, polymer, &printers, is_u1, &mut log);
    std::fs::create_dir_all(&filament_dir)?;
    let fpath = profile::write_unique_json(&filament_dir, &fname, &fval)?;
    log.push(format!("Saved filament profile to {}", fpath.display()));

    // 2) The shared 7 project-type process profiles for the chosen printer — the
    //    old per-filament "Scarf" companion is gone (v0.3.0).
    std::fs::create_dir_all(&process_dir)?;
    let mut proc_count = 0usize;
    for (name, value) in project_process::build_library_for(&specs) {
        profile::write_unique_json(&process_dir, &name, &value)?;
        proc_count += 1;
    }
    log.push(format!(
        "Saved {proc_count} project-type process profiles across {} printer variant(s).",
        specs.len()
    ));

    // v0.4.0 — opt-in, anonymous community contribution. Runs only after the
    // profiles are safely written, and is fully non-propagating: any failure
    // (offline, server 4xx, untrusted URL, panic) is caught and logged, never
    // turned into an import error. The import has already succeeded.
    if req.share {
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contribute::submit(&extracted)
        }));
        match outcome {
            Ok(Ok(())) => log.push("Shared with the community database.".to_string()),
            Ok(Err(e)) => log.push(format!("Community share skipped: {e}")),
            Err(_) => log.push("Community share skipped: internal error.".to_string()),
        }
    }

    Ok(ImportResult {
        extracted,
        profile_path: Some(fpath.display().to_string()),
        recommended_process: None,
        process_count: proc_count,
        process_dir: Some(process_dir.display().to_string()),
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
                    // Batch path is no longer wired to the UI (the Catalogue tab
                    // is removed). Default to the Snapmaker_Orca / U1 target so it
                    // still compiles and produces a filament + the 7 process.
                    slicer: None,
                    custom_dir: None,
                    vendor: None,
                    model: None,
                    nozzle: None,
                    all_nozzles: false,
                    // Batch path is headless / not wired to the opt-in checkbox;
                    // don't auto-contribute from it.
                    share: false,
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

/// v0.3.0 — pick a destination folder for the "custom" slicer option.
#[tauri::command]
fn pick_folder(app: tauri::AppHandle) -> std::result::Result<Option<String>, Error> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |f| {
        let _ = tx.send(f.and_then(|p| p.into_path().ok()));
    });
    let path = rx.recv().map_err(|e| Error::Other(e.to_string()))?;
    Ok(path.map(|p| p.display().to_string()))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProcessLibraryResult {
    count: usize,
    dir: String,
    names: Vec<String>,
}

/// v0.1.16 — write the shared project-type process library (7 project types ×
/// 4 nozzles = 28 process profiles) into the Snapmaker_Orca user `process/`
/// folder so they appear in the slicer's Process dropdown. One shared set; the
/// per-filament tuning stays on the filament profile.
#[tauri::command]
fn generate_process_library(
    slicer: Option<String>,
    custom_dir: Option<String>,
) -> std::result::Result<ProcessLibraryResult, Error> {
    run_command(move || {
        let sl = slicer::Slicer::parse(slicer.as_deref().unwrap_or("snapmaker"), custom_dir.as_deref())?;
        let user = slicer::resolve_output_dir_or_err(&sl)?;
        let dir = user.join("process");
        std::fs::create_dir_all(&dir)?;
        let mut names = Vec::new();
        for (name, value) in project_process::build_library() {
            profile::write_unique_json(&dir, &name, &value)?;
            names.push(name);
        }
        Ok(ProcessLibraryResult {
            count: names.len(),
            dir: dir.display().to_string(),
            names,
        })
    })
}

// --- v0.1.18 universal printer support: catalogue selectors + on-demand process
//     library for ANY OrcaSlicer-family printer (vendor -> model -> nozzle). ---

#[tauri::command]
fn list_printer_vendors() -> std::result::Result<Vec<String>, Error> {
    run_command(|| Ok(catalog::vendors()))
}

#[tauri::command]
fn list_printer_models(vendor: String) -> std::result::Result<Vec<String>, Error> {
    run_command(move || Ok(catalog::models(&vendor)))
}

#[tauri::command]
fn list_printer_nozzles(vendor: String, model: String) -> std::result::Result<Vec<f64>, Error> {
    run_command(move || Ok(catalog::nozzles(&vendor, &model)))
}

/// Generate the 7 project-type process profiles for a chosen catalogue printer
/// (on-demand). Resolves the printer's stock base process + preset name from the
/// machine catalogue, then writes the profiles to the Snapmaker_Orca process/ dir.
/// Resolve the catalogue print target(s): one nozzle, or EVERY nozzle of the
/// model when `all_nozzles` is set. Shared by the process-only and the
/// filament+process commands so "toutes les buses" behaves identically.
fn resolve_specs(
    vendor: &str,
    model: &str,
    nozzle: Option<f64>,
    all_nozzles: bool,
) -> Result<Vec<project_process::PrinterSpec>> {
    let specs: Vec<project_process::PrinterSpec> = if all_nozzles {
        catalog::nozzles(vendor, model)
            .into_iter()
            .filter_map(|n| catalog::resolve(vendor, model, Some(n)))
            .collect()
    } else {
        vec![catalog::resolve(vendor, model, nozzle).ok_or_else(|| {
            Error::Other(format!("Unknown printer in catalogue: {vendor} / {model}"))
        })?]
    };
    if specs.is_empty() {
        return Err(Error::Other(format!(
            "No printer variant resolved for {vendor} / {model}."
        )));
    }
    Ok(specs)
}

#[tauri::command]
fn generate_process_library_for(
    vendor: String,
    model: String,
    nozzle: Option<f64>,
    all_nozzles: bool,
    slicer: Option<String>,
    custom_dir: Option<String>,
) -> std::result::Result<ProcessLibraryResult, Error> {
    run_command(move || {
        let specs = resolve_specs(&vendor, &model, nozzle, all_nozzles)?;
        let sl = slicer::Slicer::parse(slicer.as_deref().unwrap_or("snapmaker"), custom_dir.as_deref())?;
        let user = slicer::resolve_output_dir_or_err(&sl)?;
        let dir = user.join("process");
        std::fs::create_dir_all(&dir)?;
        let mut names = Vec::new();
        for (name, value) in project_process::build_library_for(&specs) {
            profile::write_unique_json(&dir, &name, &value)?;
            names.push(name);
        }
        Ok(ProcessLibraryResult { count: names.len(), dir: dir.display().to_string(), names })
    })
}

// --- v0.2.0 Bibliothèque Filament: read the bundled/updatable filament
//     database, and the one-click flow that turns a chosen material + printer
//     into a filament profile + the 7 project-type process profiles. ---

#[tauri::command]
fn list_filaments(
    query: Option<String>,
    brand: Option<String>,
) -> std::result::Result<Vec<library::FilamentSummary>, Error> {
    run_command(move || library::list(query, brand, 500))
}

/// v0.4.0 — distinct brand names for the Bibliothèque Filament "Marque" filter.
#[tauri::command]
fn list_filament_brands() -> std::result::Result<Vec<String>, Error> {
    run_command(library::list_brands)
}

#[tauri::command]
fn get_filament(id: i64) -> std::result::Result<library::FilamentDetail, Error> {
    run_command(move || library::get(id))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CombinedResult {
    /// v0.3.0 — one filament profile per selected material (multi-select).
    filament_names: Vec<String>,
    filament_count: usize,
    process_count: usize,
    process_dir: String,
    process_names: Vec<String>,
    printers: Vec<String>,
    log: Vec<String>,
}

/// The one-click flow: from one OR MORE chosen DATABASE materials + a chosen
/// printer (vendor/model + one nozzle, or every nozzle of the machine via
/// `all_nozzles`), generate ONE filament profile per material AND the shared 7
/// project-type process profiles ONCE (they depend only on the printer/nozzle,
/// not the filament). Multi-brand selection is allowed. The destination is the
/// chosen slicer's user directory (v0.3.0). The filament-specific tuning stays
/// on the filament profile; the shared process set carries the fork features +
/// cornering/resonance.
#[tauri::command]
fn generate_filament_and_process(
    material_ids: Vec<i64>,
    vendor: String,
    model: String,
    nozzle: Option<f64>,
    all_nozzles: bool,
    slicer: Option<String>,
    custom_dir: Option<String>,
) -> std::result::Result<CombinedResult, Error> {
    run_command(move || {
        if material_ids.is_empty() {
            return Err(Error::Profile("Select at least one filament.".into()));
        }
        // 1. Resolve the print target(s) from the machine catalogue.
        let specs = resolve_specs(&vendor, &model, nozzle, all_nozzles)?;
        // Distinct printer preset names → the filament's compatible_printers.
        let mut printers: Vec<String> = Vec::new();
        for s in &specs {
            if !printers.contains(&s.printer_name) {
                printers.push(s.printer_name.clone());
            }
        }
        let is_u1 = printers.iter().any(|p| p.contains("Snapmaker U1"));

        // 2. Resolve the destination slicer user dir.
        let sl = slicer::Slicer::parse(slicer.as_deref().unwrap_or("snapmaker"), custom_dir.as_deref())?;
        let user = slicer::resolve_output_dir_or_err(&sl)?;
        let fdir = user.join("filament");
        std::fs::create_dir_all(&fdir)?;

        // 3. One filament profile per selected material (multi-brand allowed).
        let mut log = Vec::new();
        let mut filament_names: Vec<String> = Vec::new();
        for id in &material_ids {
            let ef = library::material_to_extracted(*id)?;
            let polymer = ef.polymer.unwrap_or(Polymer::Other);
            let (fname, fval) =
                profile::build_filament_json_for(&ef, polymer, &printers, is_u1, &mut log);
            let fpath = profile::write_unique_json(&fdir, &fname, &fval)?;
            log.push(format!("Saved filament profile to {}", fpath.display()));
            filament_names.push(fname);
        }

        // 4. The shared 7×N project-type process set — built ONCE (printer-only).
        let pdir = user.join("process");
        std::fs::create_dir_all(&pdir)?;
        let mut names = Vec::new();
        for (name, value) in project_process::build_library_for(&specs) {
            profile::write_unique_json(&pdir, &name, &value)?;
            names.push(name);
        }
        log.push(format!(
            "Saved {} filament profile(s) + {} process profiles across {} printer variant(s).",
            filament_names.len(),
            names.len(),
            specs.len()
        ));

        Ok(CombinedResult {
            filament_count: filament_names.len(),
            filament_names,
            process_count: names.len(),
            process_dir: pdir.display().to_string(),
            process_names: names,
            printers,
            log,
        })
    })
}

/// v0.1.17 — check the Maison Drabiec server for a newer app and/or filament
/// database. A newer DATABASE is downloaded automatically; a newer APP is only
/// reported (the frontend offers to open the download page).
#[tauri::command]
fn check_updates() -> std::result::Result<update::UpdateStatus, Error> {
    run_command(|| update::check(true))
}

/// Open a URL in the user's default browser (used by the app-update flow to
/// reach the download page — never auto-replaces the binary).
#[tauri::command]
fn open_external(url: String) -> std::result::Result<(), Error> {
    run_command(move || {
        // Only ever open the Maison Drabiec download page — never an arbitrary
        // URL that a tampered manifest could inject.
        if !update::is_trusted(&url) {
            return Err(Error::Other(
                "refused to open a URL outside the Maison Drabiec server".into(),
            ));
        }
        open::that(&url).map_err(|e| Error::Other(e.to_string()))?;
        Ok(())
    })
}

/// v0.6.0 — open a local folder (the generated-profiles directory we just wrote
/// to) in the OS file manager, so the user can jump straight to the result.
/// Local, existing path only — never a URL.
#[tauri::command]
fn reveal_in_folder(path: String) -> std::result::Result<(), Error> {
    run_command(move || {
        let p = std::path::Path::new(&path);
        if !p.exists() {
            return Err(Error::Other("folder does not exist".into()));
        }
        open::that(&path).map_err(|e| Error::Other(e.to_string()))?;
        Ok(())
    })
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
            import_pdf, pick_pdf, pick_folder, crawl_catalog, import_from_urls,
            corpus_default_path, scan_corpus, generate_process_library,
            check_updates, open_external, reveal_in_folder,
            list_printer_vendors, list_printer_models, list_printer_nozzles,
            generate_process_library_for,
            list_filaments, list_filament_brands, get_filament,
            generate_filament_and_process
        ])
        .setup(|app| {
            // First run: seed the bundled filament database into the app-data
            // dir so the Bibliothèque Filament works offline immediately. The
            // launch `check_updates` then refreshes it from the server whenever
            // a newer version is published.
            if let Some(db) = update::db_path() {
                if !db.exists() {
                    match app
                        .path()
                        .resolve("filaments.sqlite", tauri::path::BaseDirectory::Resource)
                    {
                        Ok(res) if res.exists() => {
                            if let Some(parent) = db.parent() {
                                let _ = std::fs::create_dir_all(parent);
                            }
                            match std::fs::copy(&res, &db) {
                                Ok(n) => log::info!(
                                    "Seeded filament DB ({n} bytes) from bundle to {}",
                                    db.display()
                                ),
                                Err(e) => log::warn!("Could not seed filament DB: {e}"),
                            }
                        }
                        Ok(res) => log::warn!("Bundled filament DB missing at {}", res.display()),
                        Err(e) => log::warn!("Could not resolve bundled filament DB: {e}"),
                    }
                }
            }
            log::info!(
                "main window ready: {}",
                app.get_webview_window("main").map(|_| "main").unwrap_or("?"),
            );
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Tauri runtime failure");
}
