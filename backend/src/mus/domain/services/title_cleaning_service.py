"""Title cleaning domain service.

Provides pure functions to strip YouTube junk from video titles and to split
a raw title into (artist, title) pairs.  No I/O, no side-effects.
"""

import re
import unicodedata

# ---------------------------------------------------------------------------
# Private constants
# ---------------------------------------------------------------------------

# Junk keyword phrases that appear verbatim (case-insensitive) in YouTube
# titles and carry no musical meaning.
_JUNK_KEYWORDS: tuple[str, ...] = (
    "official music video",
    "official video",
    "official lyric video",
    "official audio",
    "official visualizer",
    "official hd",
    "music video",
    "lyric video",
    "lyrics video",
    "audio only",
    "full album",
    "hd version",
    "hd remaster",
    "remastered version",
    "extended version",
    "extended mix",
    "radio edit",
    "album version",
    "single version",
    "explicit version",
    "clean version",
    "video clip",
    "visualizer",
    "fan made",
    "fan video",
    "with lyrics",
    "with subtitles",
    "subtitles",
    "closed captions",
    "auto-generated",
    "auto generated",
    "provided to youtube",
    "released on",
    "available on",
    "subscribe",
)

# Matches bracketed / parenthesised junk: e.g. "[Official Video]", "(HQ)"
# built from _JUNK_KEYWORDS plus common short tokens.
_junk_kw_pattern = "|".join(re.escape(kw) for kw in _JUNK_KEYWORDS)
_JUNK_PATTERN: re.Pattern[str] = re.compile(
    r"[\(\[\{]"
    r"\s*(?:"
    + _junk_kw_pattern
    + r"|hq|4k|1080p|720p|hd|remaster(?:ed)?|live(?:\sperformance)?)"
    r"[^\)\]\}]*[\)\]\}]",
    re.IGNORECASE,
)

# Matches YouTube video-ID suffixes that sometimes leak into titles,
# e.g. "Song Title AbCdEfGhIjK" where the last token is an 11-char ID.
_VIDEO_ID_PATTERN: re.Pattern[str] = re.compile(r"\s+[A-Za-z0-9_-]{11}$")

# Matches "feat." / "ft." / "featuring" artist credits embedded in a title.
# Group 1 captures the featured artist(s).
_FEAT_PATTERN: re.Pattern[str] = re.compile(
    r"\s*[\(\[]?\s*(?:feat(?:uring)?\.?|ft\.)\s+([^\)\]\n]+?)[\)\]]?\s*$",
    re.IGNORECASE,
)

# Delimiters used to split "Artist — Title" or "Artist: Title" etc.
# Ordered by specificity: em-dash first, then double-hyphen, then hyphen.
_ARTIST_TITLE_SPLIT: re.Pattern[str] = re.compile(
    r"\s*(?:\u2014|\u2013|--|–)\s*|\s+-\s+"
)

# YouTube auto-generated "Topic" channel suffix.
_TOPIC_SUFFIX: re.Pattern[str] = re.compile(r"\s*-\s*Topic$", re.IGNORECASE)


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def _normalize_unicode(text: str) -> str:
    """Return *text* after NFC normalisation and full-width → ASCII fold.

    Full-width Latin characters (U+FF01–U+FF5E) are common in Japanese and
    Korean music metadata and map to their ASCII equivalents by subtracting
    0xFEE0.  NFC normalization ensures composed forms are used consistently.
    """
    text = unicodedata.normalize("NFC", text)
    # Fold full-width ASCII variants (！ … ～) to their regular counterparts.
    folded = []
    for ch in text:
        cp = ord(ch)
        if 0xFF01 <= cp <= 0xFF5E:
            folded.append(chr(cp - 0xFEE0))
        else:
            folded.append(ch)
    return "".join(folded)


def clean_video_title(raw: str) -> str:
    """Strip YouTube noise from *raw* and return a clean title string.

    Steps applied in order:
    1. Unicode NFC + full-width fold.
    2. Remove bracketed junk tokens (e.g. "[Official Video]", "(HQ)").
    3. Remove bare junk keywords not inside brackets.
    4. Remove leaked video-ID suffix.
    5. Collapse repeated whitespace / trim.
    """
    text = _normalize_unicode(raw)

    # Step 2 — bracketed junk
    text = _JUNK_PATTERN.sub("", text)

    # Step 3 — bare (unbracketed) junk keywords
    for kw in _JUNK_KEYWORDS:
        text = re.sub(re.escape(kw), "", text, flags=re.IGNORECASE)

    # Step 4 — YouTube video-ID leaked suffix
    text = _VIDEO_ID_PATTERN.sub("", text)

    # Step 5 — normalise whitespace
    text = re.sub(r"\s{2,}", " ", text).strip()
    # Remove leading/trailing punctuation noise (pipe, dash) left over after removal
    text = re.sub(r"^[\s\-|–—]+|[\s\-|–—]+$", "", text).strip()

    return text


def extract_artist_title(raw: str, channel_name: str = "") -> tuple[str, str]:
    """Split *raw* into (artist, title).

    Resolution order:
    1. If *channel_name* ends with " - Topic" it is a YouTube auto-generated
       channel: the channel name (sans " - Topic") is the artist and the
       cleaned title is the track title.
    2. Try splitting on em-dash / en-dash / " - " delimiter.
    3. Fall back to (channel_name or "", cleaned title).

    The returned strings are individually cleaned with :func:`clean_video_title`.
    """
    cleaned_title = clean_video_title(raw)

    # --- Topic channel resolution ---
    if channel_name:
        topic_match = _TOPIC_SUFFIX.search(channel_name)
        if topic_match:
            artist = channel_name[: topic_match.start()].strip()
            return artist, cleaned_title

    # --- Delimiter split ---
    parts = _ARTIST_TITLE_SPLIT.split(cleaned_title, maxsplit=1)
    if len(parts) == 2:
        artist_part, title_part = parts[0].strip(), parts[1].strip()
        if artist_part and title_part:
            return artist_part, title_part

    # --- Fallback ---
    fallback_artist = channel_name.strip() if channel_name else ""
    return fallback_artist, cleaned_title
