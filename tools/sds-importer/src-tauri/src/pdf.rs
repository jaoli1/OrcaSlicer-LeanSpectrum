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
}
