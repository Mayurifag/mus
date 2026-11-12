//! Removes known upload packaging while preserving recording/version markers.

pub(super) fn strip_ascii_suffix<'a>(value: &'a str, suffix: &str) -> Option<&'a str> {
    let index = value.len().checked_sub(suffix.len())?;
    value
        .get(index..)
        .filter(|end| end.eq_ignore_ascii_case(suffix))
        .and_then(|_| value.get(..index))
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

pub(super) fn clean_packaging(raw: &str) -> String {
    let mut title = raw.trim();
    loop {
        let mut changed = false;
        for (open, close) in [('(', ')'), ('[', ']')] {
            if title.ends_with(close)
                && let Some(index) = title.rfind(open)
            {
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
        if changed {
            continue;
        }
        for suffix in ["official music video", "official video", "official audio"] {
            if let Some(prefix) = strip_ascii_suffix(title, suffix)
                && prefix.ends_with(char::is_whitespace)
            {
                let prefix = prefix.trim_end().trim_end_matches(['-', '|']).trim_end();
                if !prefix.is_empty() {
                    title = prefix;
                    changed = true;
                    break;
                }
            }
        }
        if !changed {
            break;
        }
    }
    title.to_string()
}
