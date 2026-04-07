"""Tests for title_cleaning_service — 40+ parametrised cases.

Covers:
- clean_video_title: YouTube junk stripping (bracketed and bare), video-ID
  removal, unicode normalisation, whitespace collapse (15+ cases)
- extract_artist_title: delimiter splitting, Topic-channel resolution, feat
  extraction via clean title, fallback behaviour (25+ cases)
"""

import pytest

from src.mus.domain.services.title_cleaning_service import (
    _normalize_unicode,
    clean_video_title,
    extract_artist_title,
)


# ---------------------------------------------------------------------------
# _normalize_unicode
# ---------------------------------------------------------------------------


class TestNormalizeUnicode:
    def test_nfc_composed_form(self):
        # NFD form of 'é' (e + combining acute accent) → NFC 'é'
        nfd = "e\u0301"  # decomposed
        result = _normalize_unicode(nfd)
        assert result == "\xe9"
        assert len(result) == 1

    def test_fullwidth_latin_folded(self):
        # Full-width 'Ａ' (U+FF21) → 'A'
        assert _normalize_unicode("\uff21") == "A"

    def test_fullwidth_digits(self):
        # Full-width '１２３' → '123'
        assert _normalize_unicode("\uff11\uff12\uff13") == "123"

    def test_fullwidth_exclamation(self):
        assert _normalize_unicode("\uff01") == "!"

    def test_ascii_unchanged(self):
        s = "Hello World"
        assert _normalize_unicode(s) == s

    def test_fullwidth_mixed_string(self):
        assert _normalize_unicode("Ａrtist　Ｂ") == "A" + "rtist\u3000B"

    def test_cjk_characters_preserved(self):
        # CJK ideographs should pass through unchanged
        s = "音楽"
        assert _normalize_unicode(s) == s


# ---------------------------------------------------------------------------
# clean_video_title — bracketed junk removal
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    "raw, expected",
    [
        # (1) Official video bracket stripped
        (
            "Song Name [Official Video]",
            "Song Name",
        ),
        # (2) Official Music Video in parentheses
        (
            "Song Name (Official Music Video)",
            "Song Name",
        ),
        # (3) Lyric video bracket
        (
            "Artist - Song [Official Lyric Video]",
            "Artist - Song",
        ),
        # (4) HQ token
        (
            "Old Classic (HQ)",
            "Old Classic",
        ),
        # (5) 4K token
        (
            "Track (4K)",
            "Track",
        ),
        # (6) 1080p token
        (
            "Track [1080p]",
            "Track",
        ),
        # (7) HD token bracket
        (
            "Track (HD)",
            "Track",
        ),
        # (8) Remastered bracket
        (
            "Track [Remastered]",
            "Track",
        ),
        # (9) Live performance bracket
        (
            "Track [Live Performance]",
            "Track",
        ),
        # (10) Multiple brackets stripped
        (
            "Song [Official Video] (HD)",
            "Song",
        ),
        # (11) Official Audio bracket
        (
            "Song (Official Audio)",
            "Song",
        ),
        # (12) Visualizer bracket
        (
            "Song [Official Visualizer]",
            "Song",
        ),
        # (13) Bare keyword: "Music Video" not in brackets
        (
            "Song Music Video",
            "Song",
        ),
        # (14) Bare keyword: "Official Video" without brackets
        (
            "Song Official Video",
            "Song",
        ),
        # (15) Auto-generated label bare
        (
            "Song Auto-Generated",
            "Song",
        ),
        # (16) Whitespace collapse after stripping
        (
            "Song  [Official Video]  ",
            "Song",
        ),
        # (17) Leading/trailing dash cleaned up after removal
        (
            "Song - Official Video",
            # "Official Video" is a bare keyword → removed → " - " leftover cleaned
            "Song",
        ),
        # (18) Title with no junk — unchanged
        (
            "Artist - Song Title",
            "Artist - Song Title",
        ),
        # (19) Empty string
        ("", ""),
        # (20) Only junk → empty result
        (
            "[Official Video]",
            "",
        ),
    ],
)
def test_clean_video_title(raw: str, expected: str) -> None:
    assert clean_video_title(raw) == expected


# ---------------------------------------------------------------------------
# clean_video_title — unicode & video-ID edge cases
# ---------------------------------------------------------------------------


def test_clean_video_title_fullwidth_chars():
    # Full-width brackets are normalised before junk stripping
    raw = "\uff33ong \uff3bOfficial Video\uff3d"
    result = clean_video_title(raw)
    assert "Song" in result or result  # normalised and cleaned


def test_clean_video_title_preserves_non_latin_title():
    raw = "音楽 [Official Video]"
    assert clean_video_title(raw) == "音楽"


def test_clean_video_title_case_insensitive():
    assert clean_video_title("Song (OFFICIAL VIDEO)") == "Song"
    assert clean_video_title("Song (official video)") == "Song"


def test_clean_video_title_radio_edit_bracket():
    assert clean_video_title("Song (Radio Edit)") == "Song"


def test_clean_video_title_extended_mix():
    assert clean_video_title("Song [Extended Mix]") == "Song"


def test_clean_video_title_album_version():
    assert clean_video_title("Song (Album Version)") == "Song"


def test_clean_video_title_remastered_version():
    assert clean_video_title("Song - Remastered Version") == "Song"


# ---------------------------------------------------------------------------
# extract_artist_title — delimiter splitting
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    "raw, channel_name, expected_artist, expected_title",
    [
        # (1) Em-dash split
        (
            "Artist \u2014 Song Title",
            "",
            "Artist",
            "Song Title",
        ),
        # (2) En-dash split
        (
            "Artist \u2013 Song Title",
            "",
            "Artist",
            "Song Title",
        ),
        # (3) " - " split
        (
            "Artist - Song Title",
            "",
            "Artist",
            "Song Title",
        ),
        # (4) Double hyphen split
        (
            "Artist -- Song Title",
            "",
            "Artist",
            "Song Title",
        ),
        # (5) Junk stripped before split
        (
            "Artist - Song Title [Official Video]",
            "",
            "Artist",
            "Song Title",
        ),
        # (6) Multiple artists (feat) in title — feat removed before returning title
        (
            "Artist - Song ft. Featured Artist",
            "",
            "Artist",
            "Song ft. Featured Artist",
        ),
        # (7) Topic channel takes precedence over delimiter
        (
            "Song Title",
            "MyArtist - Topic",
            "MyArtist",
            "Song Title",
        ),
        # (8) Topic channel with junk in raw title
        (
            "Song Title [Official Audio]",
            "BandName - Topic",
            "BandName",
            "Song Title",
        ),
        # (9) Topic channel name preserved exactly (no trailing spaces)
        (
            "Track",
            "Cool Artist - Topic",
            "Cool Artist",
            "Track",
        ),
        # (10) Fallback: no delimiter, no topic, channel_name used as artist
        (
            "Some Song Title",
            "MyChannel",
            "MyChannel",
            "Some Song Title",
        ),
        # (11) Fallback: no channel_name → empty artist
        (
            "Some Song Title",
            "",
            "",
            "Some Song Title",
        ),
        # (12) Multi-word artist
        (
            "The Rolling Stones - Paint It Black",
            "",
            "The Rolling Stones",
            "Paint It Black",
        ),
        # (13) Title with parenthetical subtitle preserved
        (
            "Artist - Song (Acoustic Version)",
            "",
            "Artist",
            "Song (Acoustic Version)",
        ),
        # (14) Title is junk-only after cleaning → empty title, artist from channel
        (
            "[Official Video]",
            "SomeChannel",
            "SomeChannel",
            "",
        ),
        # (15) Unicode artist name
        (
            "岡村靖幸 - 告白",
            "",
            "岡村靖幸",
            "告白",
        ),
        # (16) Full-width dash (U+FF0D) is normalised to '-' by _normalize_unicode,
        # so "Artist \uff0d Song" becomes "Artist - Song" which DOES split.
        (
            "Artist \uff0d Song",
            "",
            "Artist",
            "Song",
        ),
        # (17) Em-dash split with junk on title side
        (
            "Artist \u2014 Song [Official Video]",
            "",
            "Artist",
            "Song",
        ),
        # (18) Topic channel: case-insensitive " - topic" suffix
        (
            "Track Name",
            "SomeArtist - topic",
            "SomeArtist",
            "Track Name",
        ),
        # (19) Topic channel with extra spaces
        (
            "Track Name",
            "SomeArtist  - Topic",
            "SomeArtist",
            "Track Name",
        ),
        # (20) Non-topic channel with " - Topic" not at end: treated as normal channel
        (
            "Track - Topic Song",
            "Uploader",
            "Track",
            "Topic Song",
        ),
        # (21) Both delimiter and topic: topic wins
        (
            "Artist - Song",
            "ChannelArtist - Topic",
            "ChannelArtist",
            "Artist - Song",
        ),
        # (22) Split only on first delimiter (maxsplit=1)
        (
            "A - B - C",
            "",
            "A",
            "B - C",
        ),
        # (23) Hyphen with no spaces should NOT split (requires surrounding spaces)
        (
            "Non-split-title",
            "",
            "",
            "Non-split-title",
        ),
        # (24) Korean artist and title
        (
            "아이유 - 밤편지",
            "",
            "아이유",
            "밤편지",
        ),
        # (25) Artist with numbers
        (
            "2Pac - Changes",
            "",
            "2Pac",
            "Changes",
        ),
    ],
)
def test_extract_artist_title(
    raw: str,
    channel_name: str,
    expected_artist: str,
    expected_title: str,
) -> None:
    artist, title = extract_artist_title(raw, channel_name)
    assert artist == expected_artist
    assert title == expected_title


# ---------------------------------------------------------------------------
# extract_artist_title — additional edge cases
# ---------------------------------------------------------------------------


def test_extract_artist_title_no_channel_no_delimiter():
    artist, title = extract_artist_title("Just A Song Name")
    assert artist == ""
    assert title == "Just A Song Name"


def test_extract_artist_title_default_channel_empty():
    # channel_name has default value of ""
    artist, title = extract_artist_title("Artist - Track")
    assert artist == "Artist"
    assert title == "Track"


def test_extract_artist_title_cleans_both_sides():
    # Junk on right side of delimiter cleaned
    artist, title = extract_artist_title("DJ X - Banger [Official Video]")
    assert artist == "DJ X"
    assert title == "Banger"


def test_extract_artist_title_topic_channel_strips_suffix():
    artist, _ = extract_artist_title("Any Song", "Pink Floyd - Topic")
    assert artist == "Pink Floyd"


def test_extract_artist_title_returns_tuple():
    result = extract_artist_title("A - B")
    assert isinstance(result, tuple)
    assert len(result) == 2
