//! Unicode normalization and the filename convention used for music publication.
//!
//! ```
//! use mus_backend::audio_names::{generate_filename, normalize_artists, normalize_text, parse_artists};
//! assert_eq!(parse_artists("A; B"), ["A", "B"]);
//! assert_eq!(normalize_artists("A;B"), "A; B");
//! assert_eq!(normalize_text("e\u{301}"), "é");
//! assert_eq!(generate_filename("A; B", "Song?", "mp3")?, "A, B - Song.mp3");
//! # Ok::<(), mus_backend::error::AppError>(())
//! ```

use unicode_normalization::UnicodeNormalization;

use crate::error::AppError;

/// Splits semicolon-separated credits, preserving commas within artist names.
///
/// Trims empty credits and normalizes each retained name to Unicode NFC.
pub fn parse_artists(artist: &str) -> Vec<String> {
    artist
        .split(';')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(normalize_text)
        .collect()
}

/// Joins normalized artist credits with `; ` for storage.
pub fn normalize_artists(artist: &str) -> String {
    parse_artists(artist).join("; ")
}

/// Converts text to Unicode NFC without changing casing or punctuation.
pub fn normalize_text(value: &str) -> String {
    value.nfc().collect()
}

/// Builds `Artist, Guest - Title.ext`, removing forbidden filename characters.
///
/// The extension is supplied by trusted format selection, not arbitrary user text.
///
/// # Errors
/// Returns a bad-request error if the resulting filename exceeds 255 UTF-8 bytes.
pub fn generate_filename(artist: &str, title: &str, ext: &str) -> Result<String, AppError> {
    let artists = parse_artists(artist).join(", ");
    let filename = format!(
        "{} - {}.{}",
        sanitize(&artists),
        sanitize(title),
        ext.trim_start_matches('.')
    );
    if filename.len() > 255 {
        return Err(AppError::bad_request("Filename too long"));
    }
    Ok(filename)
}

fn sanitize(value: &str) -> String {
    normalize_text(value)
        .chars()
        .filter(|character| {
            !matches!(
                character,
                '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*'
            )
        })
        .collect()
}
