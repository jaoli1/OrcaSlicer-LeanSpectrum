//! Born-digital PDF text extraction.

use std::path::Path;

use crate::{Error, Result};

/// Extract text from a PDF file. Returns an empty string when the file has no
/// extractable text layer (typical for scanned documents). Callers should
/// fall back to [`crate::ocr::run`] in that case.
pub fn extract_text(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path)?;
    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| Error::Pdf(format!("{e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_returns_io_error() {
        let r = extract_text(Path::new("/this/does/not/exist.pdf"));
        assert!(matches!(r, Err(Error::Io(_))));
    }

    /// Temporary debug helper: dump the exact pdf-extract output of a PDF
    /// given via the DUMP_PDF env var. Run with:
    ///   DUMP_PDF=/path/to.pdf cargo test --release dump_pdf_text -- --ignored --nocapture
    #[test]
    #[ignore]
    fn dump_pdf_text() {
        let path = std::env::var("DUMP_PDF").expect("set DUMP_PDF=/path/to.pdf");
        let text = extract_text(Path::new(&path)).expect("extract failed");
        eprintln!("===== {} chars =====", text.len());
        eprintln!("{text}");
        eprintln!("===== end =====");
    }
}
