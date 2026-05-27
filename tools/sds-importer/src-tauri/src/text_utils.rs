//! UTF-8-safe text utilities shared by the SDS / TDS parsers.
//!
//! The whole point of this module is `safe_slice`: real-world vendor PDFs
//! contain `°C` (U+00B0, 2-byte UTF-8), `℃` (U+2103, 3-byte UTF-8),
//! `–` (U+2013), fullwidth `）`(U+FF09), accented letters, etc.
//!
//! Byte-indexed slices like `&text[idx..idx+200]` panic with
//! "byte index N is not a char boundary" whenever the bound lands inside
//! a multi-byte sequence — which on a TDS with degree signs in every
//! temperature range happens immediately. The panic propagates out of
//! the Tauri command worker thread and tears the WebView2 host down,
//! so the user sees the window close silently.
//!
//! `safe_slice` snaps each bound to the nearest valid char boundary
//! before borrowing — bounds DOWN for `start`, bounds UP for `end`,
//! ensuring the returned `&str` is always valid.

/// Slice `s[start..end]` clamped to byte-length and snapped to char
/// boundaries. Never panics.
///
/// - If `start > s.len()` it is clamped to `s.len()`.
/// - If `end > s.len()` it is clamped to `s.len()`.
/// - If `start > end` after clamping, an empty slice is returned.
/// - `start` is rounded DOWN to the next valid char boundary.
/// - `end`   is rounded UP   to the next valid char boundary.
pub fn safe_slice(s: &str, start: usize, end: usize) -> &str {
    if s.is_empty() {
        return s;
    }
    let len = s.len();
    let raw_start = start.min(len);
    let raw_end = end.min(len);
    if raw_start >= raw_end {
        return "";
    }
    // Snap start DOWN to a char boundary (floor).
    let mut start = raw_start;
    while start > 0 && !s.is_char_boundary(start) {
        start -= 1;
    }
    // Snap end UP to a char boundary (ceil).
    let mut end = raw_end;
    while end < len && !s.is_char_boundary(end) {
        end += 1;
    }
    // After snapping, start may have crept under end's floor — fine — but
    // we have to recheck the ordering one more time.
    if start >= end {
        return "";
    }
    &s[start..end]
}

/// Normalize a handful of Unicode characters that vendor PDFs use
/// interchangeably with their ASCII equivalents. Run this once on
/// extracted text BEFORE handing it to the parsers; the regexes are
/// already ASCII-friendly, so post-normalization most matches just
/// work without per-regex Unicode classes.
///
/// Currently normalized:
/// - `℃` (U+2103) → `°C` (the parsers can match both, but folding
///   reduces the surface for byte-slice bugs)
/// - `℉` (U+2109) → `°F`
/// - `–` (U+2013 en dash) and `—` (U+2014 em dash) → `-` ASCII hyphen
/// - `）` (U+FF09 fullwidth right paren) → `)`
/// - `（` (U+FF08 fullwidth left paren) → `(`
/// - `，` (U+FF0C fullwidth comma) → `,`
/// - `：` (U+FF1A fullwidth colon) → `:`
/// - `；` (U+FF1B fullwidth semicolon) → `;`
/// - non-breaking space (U+00A0) → regular space
///
/// This is a fast path: short replacements over already-decoded text.
pub fn normalize_unicode(text: &str) -> String {
    // Fast bail-out if the text is pure ASCII.
    if text.is_ascii() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '\u{2103}' => out.push_str("°C"),
            '\u{2109}' => out.push_str("°F"),
            '\u{2013}' | '\u{2014}' => out.push('-'),
            '\u{FF09}' => out.push(')'),
            '\u{FF08}' => out.push('('),
            '\u{FF0C}' => out.push(','),
            '\u{FF1A}' => out.push(':'),
            '\u{FF1B}' => out.push(';'),
            '\u{00A0}' => out.push(' '),
            other      => out.push(other),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_slice_empty() {
        assert_eq!(safe_slice("", 0, 0), "");
        assert_eq!(safe_slice("", 0, 10), "");
        assert_eq!(safe_slice("", 5, 10), "");
    }

    #[test]
    fn safe_slice_pure_ascii() {
        assert_eq!(safe_slice("hello world", 0, 5), "hello");
        assert_eq!(safe_slice("hello world", 6, 11), "world");
        assert_eq!(safe_slice("hello world", 0, 999), "hello world");
        assert_eq!(safe_slice("hello world", 999, 1000), "");
        assert_eq!(safe_slice("hello world", 5, 3), "");
    }

    #[test]
    fn safe_slice_degree_celsius_u2103() {
        // U+2103 ℃ is 3 bytes UTF-8 (e2 84 83).
        let s = "190℃-220℃";
        // Don't panic on any bound inside the multi-byte sequence.
        for start in 0..=s.len() {
            for end in start..=s.len() {
                let _ = safe_slice(s, start, end); // must not panic
            }
        }
        // A slice straddling the first ℃ should snap correctly and
        // contain a valid prefix.
        let slice = safe_slice(s, 0, 4); // raw byte 4 lands inside ℃
        assert!(slice.is_char_boundary(0));
        assert!(slice == "190" || slice == "190℃");
    }

    #[test]
    fn safe_slice_degree_sign_u00b0() {
        // U+00B0 ° is 2 bytes UTF-8 (c2 b0).
        let s = "190°C-220°C";
        for start in 0..=s.len() {
            for end in start..=s.len() {
                let _ = safe_slice(s, start, end);
            }
        }
        let slice = safe_slice(s, 0, 4); // straddle the first °
        assert!(slice == "190" || slice == "190°");
    }

    #[test]
    fn safe_slice_window_after_label() {
        // Mimics the tds.rs scan_range_with_hint path that crashed
        // on the ERYONE PLA+ TDS.
        let text = "Nozzle temperature 190℃-220℃ then more stuff with ° here";
        let label = "Nozzle temperature";
        let idx   = text.find(label).unwrap();
        let forward = safe_slice(text, idx + label.len(),
                                 (text.len()).min(idx + label.len() + 200));
        assert!(forward.contains("190"));
        assert!(forward.contains("220"));
    }

    #[test]
    fn normalize_celsius_glyph() {
        assert_eq!(normalize_unicode("190℃-220℃"), "190°C-220°C");
        assert_eq!(normalize_unicode("65℉"),       "65°F");
    }

    #[test]
    fn normalize_en_dash() {
        assert_eq!(normalize_unicode("190 – 220"), "190 - 220");
        assert_eq!(normalize_unicode("190—220"),   "190-220");
    }

    #[test]
    fn normalize_fullwidth_punct() {
        assert_eq!(normalize_unicode("Density(g/cm³ at 21.5°C）"),
                   "Density(g/cm³ at 21.5°C)");
        assert_eq!(normalize_unicode("65℃-75℃，12H"),
                   "65°C-75°C,12H");
    }

    #[test]
    fn normalize_ascii_is_zero_copy_equivalent() {
        let s = "plain ascii text";
        assert_eq!(normalize_unicode(s), s);
    }
}
