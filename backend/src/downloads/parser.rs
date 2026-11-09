use std::collections::HashMap;

use crate::util::{normalize_artists, normalize_text, parse_artists};

pub(super) struct Parsed {
    pub title: String,
    pub artist: String,
    pub tags: Vec<&'static str>,
}

pub(super) fn needs_structured_artist(title: &str, channel: &str) -> bool {
    let (channel, topic) = channel_name(channel);
    let title = clean_packaging(title);
    match split_title(&title) {
        None => true,
        Some((left, right)) => {
            topic && !same_text(right, channel) && !artist_mentions_channel(left, channel)
        }
    }
}

pub(super) fn topic_artist(channel: &str) -> Option<&str> {
    let (name, topic) = channel_name(channel);
    topic.then_some(name)
}

pub(super) fn with_library_artists(mut parsed: Parsed, library: &[String]) -> Parsed {
    let mut names: HashMap<String, Option<String>> = HashMap::new();
    for name in library.iter().flat_map(|entry| parse_artists(entry)) {
        let key = normalize_text(&name).to_lowercase();
        names
            .entry(key)
            .and_modify(|current| {
                if current.as_deref() != Some(name.as_str()) {
                    *current = None;
                }
            })
            .or_insert(Some(name));
    }
    parsed.artist = parse_artists(&parsed.artist)
        .into_iter()
        .map(|name| {
            names
                .get(&normalize_text(&name).to_lowercase())
                .and_then(|value| value.as_ref())
                .cloned()
                .unwrap_or(name)
        })
        .collect::<Vec<_>>()
        .join("; ");
    parsed
}

pub(super) fn parse(raw_title: &str, raw_channel: &str, source_artist: Option<&str>) -> Parsed {
    let (channel, topic) = channel_name(raw_channel);
    let cleaned = clean_packaging(raw_title);
    let structured = source_artist
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mut prefix_feature = None;
    let (artist, title) = match split_title(&cleaned) {
        Some((left, right))
            if structured.is_some_and(|artist| matches_structured(left, artist)) =>
        {
            prefix_feature = split_feature(left).1.map(str::to_string);
            (
                structured.unwrap_or_default().to_string(),
                right.to_string(),
            )
        }
        Some((left, right))
            if structured.is_some_and(|artist| matches_structured(right, artist)) =>
        {
            prefix_feature = split_feature(right).1.map(str::to_string);
            (structured.unwrap_or_default().to_string(), left.to_string())
        }
        Some(_) if structured.is_some() => (structured.unwrap_or_default().to_string(), cleaned),
        Some((left, right)) if !channel.is_empty() && same_text(right, channel) => {
            (channel.to_string(), left.to_string())
        }
        Some((left, _)) if topic && !artist_mentions_channel(left, channel) => {
            (channel.to_string(), cleaned)
        }
        Some((left, right)) => (left.to_string(), right.to_string()),
        None => (
            structured
                .unwrap_or(if channel.is_empty() {
                    "Unknown Artist"
                } else {
                    channel
                })
                .to_string(),
            cleaned,
        ),
    };

    let (base_artist, artist_feature) = split_feature(&artist);
    let mut artists = Vec::new();
    add_feature(&mut artists, base_artist, channel);
    if let Some(feature) = artist_feature {
        add_feature(&mut artists, feature, channel);
    }
    if let Some(feature) = prefix_feature {
        add_feature(&mut artists, &feature, channel);
    }
    let (base_title, title_feature) = title_feature(&title);
    if let Some(feature) = title_feature {
        add_feature(&mut artists, &feature, channel);
    }
    for value in &mut artists {
        if structured.is_none()
            && !channel.is_empty()
            && same_text(value, channel)
            && has_mixed_case(channel)
            && !has_mixed_case(value)
        {
            *value = channel.to_string();
        }
    }
    Parsed {
        artist: normalize_artists(&artists.join("; ")),
        title: normalize_text(base_title.trim()),
        tags: category_tags(raw_title, &base_title),
    }
}

fn has_mixed_case(value: &str) -> bool {
    value.chars().any(char::is_uppercase) && value.chars().any(char::is_lowercase)
}

fn add_feature(artists: &mut Vec<String>, feature: &str, channel: &str) {
    for value in split_collaboration(feature, channel) {
        for value in parse_artists(&value) {
            if !artists.iter().any(|artist| same_text(artist, &value)) {
                artists.push(value);
            }
        }
    }
}

fn same_text(left: &str, right: &str) -> bool {
    normalize_text(left.trim()).to_lowercase() == normalize_text(right.trim()).to_lowercase()
}

fn matches_structured(value: &str, structured: &str) -> bool {
    let value = split_feature(value).0;
    same_text(&normalize_artists(value), &normalize_artists(structured))
        || parse_artists(structured)
            .first()
            .is_some_and(|first| same_text(value, first))
}

fn channel_name(channel: &str) -> (&str, bool) {
    let channel = channel.trim();
    if let Some(name) = strip_ascii_suffix(channel, " - topic") {
        if !name.trim().is_empty() {
            return (name.trim(), true);
        }
    }
    (channel, false)
}

fn strip_ascii_suffix<'a>(value: &'a str, suffix: &str) -> Option<&'a str> {
    let index = value.len().checked_sub(suffix.len())?;
    value
        .get(index..)
        .filter(|end| end.eq_ignore_ascii_case(suffix))
        .and_then(|_| value.get(..index))
}

fn split_title(title: &str) -> Option<(&str, &str)> {
    [" - ", " – ", " — "]
        .iter()
        .filter_map(|separator| title.find(separator).map(|index| (index, *separator)))
        .min_by_key(|(index, _)| *index)
        .and_then(|(index, separator)| {
            let left = title[..index].trim();
            let right = title[index + separator.len()..].trim();
            (!left.is_empty() && !right.is_empty()).then_some((left, right))
        })
}

pub(super) fn channel_at_start_of_comma_credit(credit: &str, channel: &str) -> bool {
    if channel.is_empty() {
        return false;
    }
    let credit = normalize_text(credit).to_lowercase();
    let channel = normalize_text(channel).to_lowercase();
    credit
        .strip_prefix(&channel)
        .is_some_and(|remainder| remainder.starts_with(','))
}

fn artist_mentions_channel(artist: &str, channel: &str) -> bool {
    if channel.is_empty() {
        return false;
    }
    split_collaboration(artist, channel)
        .iter()
        .flat_map(|part| parse_artists(part))
        .any(|part| same_text(&part, channel) || channel_at_start_of_comma_credit(&part, channel))
}

fn split_collaboration(value: &str, channel: &str) -> Vec<String> {
    let value = value.trim();
    for separator in [" & ", " x ", " X "] {
        let parts = value.split(separator).map(str::trim).collect::<Vec<_>>();
        if parts.len() > 1
            && parts.iter().all(|part| !part.is_empty())
            && parts.iter().any(|part| same_text(part, channel))
        {
            return parts.into_iter().map(str::to_string).collect();
        }
    }
    vec![value.to_string()]
}

fn split_feature(value: &str) -> (&str, Option<&str>) {
    let lower = value.to_ascii_lowercase();
    for marker in [
        " (feat. ",
        " (feat ",
        " (ft. ",
        " (ft ",
        " (featuring ",
        " [feat. ",
        " [feat ",
        " [ft. ",
        " [ft ",
        " [featuring ",
        " feat. ",
        " feat ",
        " ft. ",
        " ft ",
        " featuring ",
    ] {
        let Some(index) = lower.rfind(marker) else {
            continue;
        };
        let base = value[..index].trim();
        let mut feature = value[index + marker.len()..].trim();
        if marker.contains('(') {
            let Some(inner) = feature.strip_suffix(')') else {
                continue;
            };
            feature = inner.trim();
        } else if marker.contains('[') {
            let Some(inner) = feature.strip_suffix(']') else {
                continue;
            };
            feature = inner.trim();
        }
        if !base.is_empty() && !feature.is_empty() && !feature.contains(['(', ')', '[', ']', '|']) {
            return (base, Some(feature));
        }
    }
    (value, None)
}

fn title_feature(value: &str) -> (String, Option<String>) {
    let lower = value.to_ascii_lowercase();
    for (marker, close) in [
        (" (feat. ", ')'),
        (" (feat ", ')'),
        (" (ft. ", ')'),
        (" (ft ", ')'),
        (" (featuring ", ')'),
        (" [feat. ", ']'),
        (" [feat ", ']'),
        (" [ft. ", ']'),
        (" [ft ", ']'),
        (" [featuring ", ']'),
    ] {
        let Some(index) = lower.find(marker) else {
            continue;
        };
        let start = index + marker.len();
        let Some(end) = value[start..].find(close).map(|end| start + end) else {
            continue;
        };
        let feature = value[start..end].trim();
        let base = value[..index].trim();
        if base.is_empty()
            || feature.is_empty()
            || feature.contains(['(', ')', '[', ']', '|'])
            || [" - ", " – ", " — "]
                .iter()
                .any(|dash| feature.contains(dash))
        {
            continue;
        }
        let suffix = value[end + close.len_utf8()..].trim();
        let title = if suffix.is_empty() {
            base.to_string()
        } else {
            format!("{base} {suffix}")
        };
        return (title, Some(feature.to_string()));
    }
    (value.to_string(), None)
}

fn is_packaging(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "official video"
            | "official music video"
            | "official audio"
            | "official lyric video"
            | "lyric video"
            | "lyrics video"
            | "music video"
            | "official visualizer"
            | "visualizer"
            | "lyrics"
            | "audio"
    )
}

fn clean_packaging(raw: &str) -> String {
    let mut title = raw.trim();
    loop {
        let mut changed = false;
        for (open, close) in [('(', ')'), ('[', ']')] {
            if title.ends_with(close) {
                if let Some(index) = title.rfind(open) {
                    let prefix = title[..index].trim();
                    if !prefix.is_empty()
                        && !prefix.ends_with(['-', '–', '—', '|'])
                        && is_packaging(&title[index + 1..title.len() - 1])
                    {
                        title = prefix;
                        changed = true;
                        break;
                    }
                }
            }
        }
        if changed {
            continue;
        }
        for suffix in ["official music video", "official video", "official audio"] {
            if let Some(prefix) = strip_ascii_suffix(title, suffix) {
                if prefix.ends_with(char::is_whitespace) {
                    let prefix = prefix.trim_end().trim_end_matches(['-', '|']).trim_end();
                    if !prefix.is_empty() {
                        title = prefix;
                        changed = true;
                        break;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    title.to_string()
}

fn category_marker_text(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn category_tags(title: &str, parsed_title: &str) -> Vec<&'static str> {
    let mut markers = Vec::new();
    for (open, close) in [('(', ')'), ('[', ']')] {
        let mut remaining = title;
        while let Some(start) = remaining.find(open) {
            let after = &remaining[start + open.len_utf8()..];
            let Some(end) = after.find(close) else { break };
            markers.push(after[..end].to_string());
            remaining = &after[end + close.len_utf8()..];
        }
    }
    markers.extend(title.split('|').skip(1).map(str::to_string));
    let trailing = [" - ", " – ", " — "]
        .iter()
        .filter_map(|separator| parsed_title.rsplit_once(separator))
        .min_by_key(|(_, marker)| marker.len());
    if let Some((before, marker)) = trailing {
        if !before.trim().is_empty()
            && matches!(
                category_marker_text(marker).as_str(),
                "right version"
                    | "right ver"
                    | "ai cover"
                    | "a i cover"
                    | "cover"
                    | "ost"
                    | "mashup"
                    | "mash up"
            )
        {
            markers.push(marker.to_string());
        }
    }
    if title.to_ascii_lowercase().contains("♂ right version ♂") {
        markers.push("right version".into());
    }
    let markers = markers
        .iter()
        .filter(|marker| !marker.contains('?'))
        .map(|marker| category_marker_text(marker))
        .filter(|marker| {
            !marker
                .split_whitespace()
                .any(|word| matches!(word, "no" | "not" | "without" | "не"))
        })
        .collect::<Vec<_>>();
    let contains = |needle: &str| markers.iter().any(|marker| marker == needle);
    let starts = |needle: &str| {
        markers
            .iter()
            .any(|marker| marker.starts_with(&format!("{needle} ")))
    };
    let ends = |needle: &str| {
        markers
            .iter()
            .any(|marker| marker.ends_with(&format!(" {needle}")))
    };
    let mut tags = Vec::new();
    if contains("right version") || contains("right ver") {
        tags.push("right-version");
    }
    if contains("ai cover") || contains("a i cover") || starts("ai cover") || ends("ai cover") {
        tags.push("ai-cover");
    } else if contains("cover") || starts("cover by") {
        tags.push("cover");
    }
    if contains("ost") || starts("ost") || ends("ost") {
        tags.push("ost");
    }
    if contains("mashup")
        || contains("mash up")
        || starts("mashup")
        || starts("mash up")
        || ends("mashup")
        || ends("mash up")
    {
        tags.push("mashup");
    }
    tags
}

#[cfg(test)]
mod tests {
    use super::{parse, with_library_artists};

    fn check(
        title: &str,
        channel: &str,
        source_artist: Option<&str>,
        expected: (&str, &str, &[&str]),
    ) {
        let parsed = parse(title, channel, source_artist);
        assert_eq!(
            (
                parsed.artist.as_str(),
                parsed.title.as_str(),
                parsed.tags.as_slice()
            ),
            expected
        );
    }

    #[test]
    fn library_casing_requires_one_existing_spelling_per_artist() {
        let parsed = parse("ARTIST feat. Guest - Song", "Publisher", None);
        let result = with_library_artists(parsed, &["Artist; Guest".into(), "Artist".into()]);
        assert_eq!(result.artist, "Artist; Guest");
        let parsed = parse("Artist - Song", "Publisher", None);
        let result = with_library_artists(parsed, &["ARTIST".into(), "artist".into()]);
        assert_eq!(result.artist, "Artist");
        let parsed = parse("Other - Song", "Publisher", None);
        let result = with_library_artists(parsed, &["Otter".into()]);
        assert_eq!(result.artist, "Other");
    }

    #[test]
    fn parses_youtube_and_structured_metadata() {
        check(
            "Artist - Song (Official Video)",
            "Publisher",
            None,
            ("Artist", "Song", &[]),
        );
        check(
            "Artist — Song [Lyric Video]",
            "Artist",
            None,
            ("Artist", "Song", &[]),
        );
        check("Song", "Artist - Topic", None, ("Artist", "Song", &[]));
        check("Song — Artist", "Artist", None, ("Artist", "Song", &[]));
        check(
            "Artist - Song (Official Audio)",
            "Publisher",
            Some("Artist"),
            ("Artist", "Song", &[]),
        );
        check(
            "Song Official Audio",
            "Publisher",
            Some("Artist, Guest"),
            ("Artist, Guest", "Song", &[]),
        );
        check("Song", "Artist - Topic", Some(" "), ("Artist", "Song", &[]));
        check(
            "Song",
            "Publisher",
            Some("Artist, Guest, Guest"),
            ("Artist, Guest, Guest", "Song", &[]),
        );
        check(
            "Artist, Guest - Song",
            "Artist - Topic",
            None,
            ("Artist, Guest", "Song", &[]),
        );
    }

    #[test]
    fn handles_explicit_features_and_supported_collaborations() {
        check(
            "Artist feat. Guest - Song",
            "Artist",
            None,
            ("Artist; Guest", "Song", &[]),
        );
        check(
            "Artist - Song (ft. Guest) [Official Video]",
            "Artist",
            None,
            ("Artist; Guest", "Song", &[]),
        );
        check(
            "Artist & Guest - Song",
            "Guest",
            None,
            ("Artist; Guest", "Song", &[]),
        );
        check(
            "Artist & Guest - Song",
            "Publisher",
            None,
            ("Artist & Guest", "Song", &[]),
        );
        check(
            "Artist - Song (feat. Guest) (Cover)",
            "Artist",
            None,
            ("Artist; Guest", "Song (Cover)", &["cover"]),
        );
        check(
            "Artist - Song (feat. Guest)",
            "Publisher",
            Some("Artist; Guest"),
            ("Artist; Guest", "Song", &[]),
        );
        check(
            "Artist - Song (feat. Guest)",
            "Publisher",
            Some("Artist"),
            ("Artist; Guest", "Song", &[]),
        );
        check(
            "Artist feat. Guest - Song (feat. Guest)",
            "Artist",
            None,
            ("Artist; Guest", "Song", &[]),
        );
        check(
            "Artist feat. Guest - Song",
            "Publisher",
            Some("Artist"),
            ("Artist; Guest", "Song", &[]),
        );
    }

    #[test]
    fn preserves_acoustic_recording_markers() {
        check(
            "Artist - Song (Acoustic) (Official Video)",
            "Artist",
            None,
            ("Artist", "Song (Acoustic)", &[]),
        );
        check(
            "Song [Acoustic Version] (Official Audio)",
            "Artist - Topic",
            Some("Artist"),
            ("Artist", "Song [Acoustic Version]", &[]),
        );
        check(
            "Artist - Song (feat. Guest) (Acoustic)",
            "Artist",
            None,
            ("Artist; Guest", "Song (Acoustic)", &[]),
        );
    }

    #[test]
    fn preserves_ambiguous_feature_credits_and_version_markers() {
        check(
            "Artist - Song feat. Guest - Live",
            "Artist",
            None,
            ("Artist", "Song feat. Guest - Live", &[]),
        );
        check(
            "Artist - Song feat. Guest Acoustic",
            "Artist",
            None,
            ("Artist", "Song feat. Guest Acoustic", &[]),
        );
        check(
            "Artist - Song (feat. Guest - Live)",
            "Artist",
            None,
            ("Artist", "Song (feat. Guest - Live)", &[]),
        );
        check(
            "Artist - Song (feat. Guest) - Live",
            "Artist",
            None,
            ("Artist; Guest", "Song - Live", &[]),
        );
    }

    #[test]
    fn preserves_commas_in_artist_names() {
        check(
            "Song",
            "Tyler, The Creator - Topic",
            Some("Tyler, The Creator"),
            ("Tyler, The Creator", "Song", &[]),
        );
        check(
            "Song",
            "Publisher",
            Some("Tyler, The Creator; Guest"),
            ("Tyler, The Creator; Guest", "Song", &[]),
        );
    }

    #[test]
    fn preserves_ambiguous_titles_and_category_markers() {
        check(
            "Artist - Song - AI Cover",
            "Artist",
            None,
            ("Artist", "Song - AI Cover", &["ai-cover"]),
        );
        check(
            "Artist - Song - Cover",
            "Artist",
            None,
            ("Artist", "Song - Cover", &["cover"]),
        );
        for (title, expected) in [
            ("Artist - Song – OST", "ost"),
            ("Artist - Song — Mash-Up", "mashup"),
            ("Artist - Song - Right Version", "right-version"),
        ] {
            let parsed = parse(title, "Artist", None);
            assert_eq!(parsed.tags, [expected]);
        }
        check(
            "Song - AI Cover",
            "Artist - Topic",
            None,
            ("Artist", "Song - AI Cover", &["ai-cover"]),
        );
        check(
            "Artist - Song - Not AI Cover",
            "Artist",
            None,
            ("Artist", "Song - Not AI Cover", &[]),
        );
        check(
            "Artist - Song - AI Cover?",
            "Artist",
            None,
            ("Artist", "Song - AI Cover?", &[]),
        );
        check(
            "Artist - Song - Cover Me",
            "Artist",
            None,
            ("Artist", "Song - Cover Me", &[]),
        );
        check(
            "Artist - Song - AI Cover (Official Video)",
            "Artist",
            None,
            ("Artist", "Song - AI Cover", &["ai-cover"]),
        );
        check(
            "Song - Subtitle",
            "Artist - Topic",
            None,
            ("Artist", "Song - Subtitle", &[]),
        );
        check(
            "Artist - Song (Live at Home)",
            "Artist",
            None,
            ("Artist", "Song (Live at Home)", &[]),
        );
        check(
            "Artist - Song (AI cover)",
            "Artist",
            None,
            ("Artist", "Song (AI cover)", &["ai-cover"]),
        );
        check(
            "Artist - Song [OST mash-up]",
            "Artist",
            None,
            ("Artist", "Song [OST mash-up]", &["ost", "mashup"]),
        );
        check(
            "Artist - Song (Game OST)",
            "Artist",
            None,
            ("Artist", "Song (Game OST)", &["ost"]),
        );
        check(
            "Artist - Song (Mashup by Guest)",
            "Artist",
            None,
            ("Artist", "Song (Mashup by Guest)", &["mashup"]),
        );
        check(
            "Artist - Song (Guest AI Cover)",
            "Artist",
            None,
            ("Artist", "Song (Guest AI Cover)", &["ai-cover"]),
        );
        check(
            "Artist - Song (♂ Right Version ♂)",
            "Artist",
            None,
            ("Artist", "Song (♂ Right Version ♂)", &["right-version"]),
        );
        check(
            "Artist - Song ♂ Right Version ♂ Remix",
            "Artist",
            None,
            ("Artist", "Song ♂ Right Version ♂ Remix", &["right-version"]),
        );
        check(
            "Artist - Song (Not AI Cover)",
            "Artist",
            None,
            ("Artist", "Song (Not AI Cover)", &[]),
        );
        check(
            "Artist - Song [Right Version] (Cover)",
            "Artist",
            None,
            (
                "Artist",
                "Song [Right Version] (Cover)",
                &["right-version", "cover"],
            ),
        );
        check(
            "Artist - Discover",
            "Artist",
            None,
            ("Artist", "Discover", &[]),
        );
        check(
            "Artist - Cover Me",
            "Artist",
            None,
            ("Artist", "Cover Me", &[]),
        );
        check("Artist - Cover", "Artist", None, ("Artist", "Cover", &[]));
        check(
            "Artist - (Official Audio)",
            "Artist",
            None,
            ("Artist", "(Official Audio)", &[]),
        );
        check("Artist - Song", "ARTIST", None, ("Artist", "Song", &[]));
        check("ARTIST - Song", "Artist", None, ("Artist", "Song", &[]));
        check(
            "Artist - Song (Album Cover)",
            "Artist",
            None,
            ("Artist", "Song (Album Cover)", &[]),
        );
    }
}
