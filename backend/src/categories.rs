//! Marker-based category inference for existing library titles.
//!
//! These broad title checks differ from the stricter download-credit parser.
//!
//! ```
//! use mus_backend::categories::*;
//! assert!(has_right_version_marker("Song [right-version]"));
//! assert!(has_ai_cover_marker("Song (AI cover)"));
//! assert!(!has_cover_marker("Song (AI cover)"));
//! assert!(has_ost_marker("Song [OST]"));
//! assert!(has_mashup_marker("Song mash-up"));
//! assert_eq!(inferred_tag_names("Song [OST mash-up]"), ["ost", "mashup"]);
//! ```

pub(crate) const CATEGORIES: &[(&str, &str)] = &[
    ("right-version", "Right version"),
    ("ai-cover", "AI cover"),
    ("cover", "Cover"),
    ("ost", "OST"),
    ("mashup", "Mashup"),
];

pub(crate) fn category_display_name(name: &str) -> Option<&'static str> {
    CATEGORIES
        .iter()
        .find_map(|(category, label)| (*category == name).then_some(*label))
}

/// Detects a normalized `right version` or `right ver` substring.
pub fn has_right_version_marker(value: &str) -> bool {
    let normalized = marker_match_text(value);
    normalized.contains("right version") || normalized.contains("right ver")
}

/// Detects normalized `ai cover` or `a i cover` text.
pub fn has_ai_cover_marker(value: &str) -> bool {
    let normalized = marker_match_text(value);
    normalized.contains("ai cover") || normalized.contains("a i cover")
}

/// Detects a `cover` word, excluding titles with an AI-cover marker.
pub fn has_cover_marker(value: &str) -> bool {
    marker_match_text(value)
        .split(|character: char| !character.is_alphanumeric())
        .any(|word| word == "cover")
        && !has_ai_cover_marker(value)
}

/// Detects an `ost` word after marker normalization.
pub fn has_ost_marker(value: &str) -> bool {
    marker_match_text(value)
        .split_whitespace()
        .any(|word| word == "ost")
}

/// Detects normalized `mashup` or `mash up` text.
pub fn has_mashup_marker(value: &str) -> bool {
    let normalized = marker_match_text(value);
    normalized.contains("mashup") || normalized.contains("mash up")
}

/// Infers tags in right-version, AI-cover/cover, OST, then mashup order.
pub fn inferred_tag_names(title: &str) -> Vec<String> {
    let mut tags = Vec::new();
    if has_right_version_marker(title) {
        tags.push("right-version".into());
    }
    if has_ai_cover_marker(title) {
        tags.push("ai-cover".into());
    } else if has_cover_marker(title) {
        tags.push("cover".into());
    }
    if has_ost_marker(title) {
        tags.push("ost".into());
    }
    if has_mashup_marker(title) {
        tags.push("mashup".into());
    }
    tags
}

fn marker_match_text(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .replace(['(', ')', '[', ']', '-', '_', '.'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
