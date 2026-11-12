//! Strict category markers in download titles, distinct from library inference.

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

pub(super) fn category_tags(title: &str, parsed_title: &str) -> Vec<&'static str> {
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
    if let Some((before, marker)) = trailing
        && !before.trim().is_empty()
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
