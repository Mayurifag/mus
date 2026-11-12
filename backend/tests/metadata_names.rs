//! Filename normalization, artist credits, and category marker contracts.

use mus_backend::{
    audio_names::{generate_filename, normalize_artists, normalize_text, parse_artists},
    categories::{
        has_ai_cover_marker, has_cover_marker, has_mashup_marker, has_ost_marker,
        has_right_version_marker, inferred_tag_names,
    },
};

#[test]
fn parses_semicolon_separated_artists_and_preserves_commas() {
    assert_eq!(
        parse_artists("aikko,katanacss; INSPACE, playingtheangel"),
        ["aikko,katanacss", "INSPACE, playingtheangel"]
    );
    assert_eq!(
        parse_artists("Tyler, The Creator; Guest"),
        ["Tyler, The Creator", "Guest"]
    );
}

#[test]
fn normalizes_artists_for_storage() {
    assert_eq!(
        normalize_artists("aikko; katanacss;INSPACE"),
        "aikko; katanacss; INSPACE"
    );
    assert_eq!(
        normalize_artists("Tyler, The Creator"),
        "Tyler, The Creator"
    );
}

#[test]
fn generates_filename_with_readable_artist_spacing() {
    assert_eq!(
        generate_filename("aikko; katanacss; INSPACE", "song", "mp3").unwrap(),
        "aikko, katanacss, INSPACE - song.mp3"
    );
}

#[test]
fn normalizes_unicode_to_nfc() {
    assert_eq!(normalize_text("Бедныи\u{0306}"), "Бедный");
    assert_eq!(
        generate_filename("Слава КПСС", "Бедныи\u{0306} Русскии\u{0306}", "mp3").unwrap(),
        "Слава КПСС - Бедный Русский.mp3"
    );
}

#[test]
fn preserves_requested_artist_casing() {
    assert_eq!(normalize_artists("LIL KRYSTALLL"), "LIL KRYSTALLL");
    assert_eq!(
        generate_filename("LIL KRYSTALLL", "Air Force 1", "mp3").unwrap(),
        "LIL KRYSTALLL - Air Force 1.mp3"
    );
}

#[test]
fn detects_right_version_markers_without_rewriting_text() {
    for input in [
        "Song (right version)",
        "Song [right version]",
        "Song right ver",
        "Song right-version",
        "Song RIGHT VERSION",
    ] {
        assert_eq!(normalize_text(input), input);
        assert!(has_right_version_marker(input));
    }
}

#[test]
fn detects_ai_cover_markers_without_rewriting_text() {
    for input in [
        "Song (ai cover)",
        "Song [ai cover]",
        "Song ai-cover",
        "Song AI COVER",
        "Song a.i. cover",
    ] {
        assert_eq!(normalize_text(input), input);
        assert!(has_ai_cover_marker(input));
    }
}

#[test]
fn detects_cover_markers_without_matching_ai_cover() {
    assert!(has_cover_marker("Song (cover)"));
    assert!(has_cover_marker("Song (cover!)"));
    assert!(!has_cover_marker("Discover"));
    assert!(!has_cover_marker("Recover"));
    assert_eq!(inferred_tag_names("Song (cover)"), ["cover"]);
    assert_eq!(inferred_tag_names("Song (AI cover)"), ["ai-cover"]);
}

#[test]
fn detects_ost_and_mashup_markers() {
    assert!(has_ost_marker("Song [OST]"));
    assert!(has_mashup_marker("Song mash-up"));
    assert_eq!(inferred_tag_names("Song [OST mash-up]"), ["ost", "mashup"]);
}
