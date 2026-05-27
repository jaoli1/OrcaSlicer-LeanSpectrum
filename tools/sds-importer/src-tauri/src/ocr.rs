//! Tesseract OCR fallback for scanned PDFs.
//!
//! Compiled in only when the `ocr` feature is enabled. Without the feature,
//! `run()` returns a clear error pointing users to the rebuild instructions.
//! We do not bundle Tesseract: when the feature IS enabled, the user
//! installs the system Tesseract binary plus its trained-data files via
//! their package manager (brew / winget / apt).

use std::path::{Path, PathBuf};

use crate::{Error, Result};

#[cfg(feature = "ocr")]
pub fn run(pdf_path: &Path) -> Result<String> {
    let pages = rasterise_pages(pdf_path)?;
    if pages.is_empty() {
        return Err(Error::Ocr("PDF produced no rasterised pages".into()));
    }

    let langs = std::env::var("LEANSPECTRUM_OCR_LANGS").unwrap_or_else(|_| "fra+eng".into());

    let mut joined = String::new();
    for page in pages {
        match ocr_one(&page, &langs) {
            Ok(t) => {
                joined.push_str(&t);
                joined.push('\n');
            }
            Err(e) => {
                log::warn!("OCR on {} failed: {e}", page.display());
            }
        }
        let _ = std::fs::remove_file(&page);
    }

    if joined.trim().is_empty() {
        return Err(Error::Ocr("Tesseract returned no text".into()));
    }
    Ok(joined)
}

#[cfg(not(feature = "ocr"))]
pub fn run(_pdf_path: &Path) -> Result<String> {
    Err(Error::Ocr(
        "OCR fallback not compiled in. Rebuild with `cargo tauri build --features ocr` \
         after installing the system Tesseract and Leptonica libraries (brew/apt/vcpkg)."
            .into(),
    ))
}

#[cfg(feature = "ocr")]
fn ocr_one(image_path: &Path, langs: &str) -> Result<String> {
    let mut t = tesseract::Tesseract::new(None, Some(langs))
        .map_err(|e| Error::Ocr(format!(
            "Tesseract init failed ({e}). Is tesseract installed and are the language files present?"
        )))?;
    t = t.set_image(image_path.to_str().ok_or_else(|| Error::Ocr("non-utf8 path".into()))?)
        .map_err(|e| Error::Ocr(format!("set_image: {e}")))?;
    t.get_text().map_err(|e| Error::Ocr(format!("get_text: {e}")))
}

#[cfg(all(feature = "ocr", feature = "pdfium"))]
fn rasterise_pages(pdf_path: &Path) -> Result<Vec<PathBuf>> {
    use pdfium_render::prelude::*;
    let pdfium = Pdfium::default();
    let doc = pdfium
        .load_pdf_from_file(pdf_path, None)
        .map_err(|e| Error::Ocr(format!("pdfium load: {e}")))?;

    let mut out = Vec::new();
    let tmp_dir = std::env::temp_dir().join(format!("leanspectrum-ocr-{}", std::process::id()));
    std::fs::create_dir_all(&tmp_dir)?;

    for (i, page) in doc.pages().iter().enumerate() {
        let cfg = PdfRenderConfig::new().set_target_width(2000);
        let bitmap = page
            .render_with_config(&cfg)
            .map_err(|e| Error::Ocr(format!("pdfium render page {i}: {e}")))?;
        let out_path = tmp_dir.join(format!("page-{i:04}.png"));
        bitmap
            .as_image()
            .save(&out_path)
            .map_err(|e| Error::Ocr(format!("save page {i}: {e}")))?;
        out.push(out_path);
    }
    Ok(out)
}

#[cfg(all(feature = "ocr", not(feature = "pdfium")))]
fn rasterise_pages(_pdf_path: &Path) -> Result<Vec<PathBuf>> {
    Err(Error::Ocr(
        "pdfium feature disabled; rebuild with --features pdfium,ocr to enable OCR on scanned PDFs".into(),
    ))
}
