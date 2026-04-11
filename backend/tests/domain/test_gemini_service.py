from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from src.mus.domain.services.gemini_service import parse_track_metadata, _TrackMetadata


def _make_response(artist: str, title: str) -> MagicMock:
    mock_resp = MagicMock()
    mock_resp.parsed = _TrackMetadata(artist=artist, title=title)
    return mock_resp


def _mock_aio_client(return_value=None, side_effect=None) -> MagicMock:
    mock_generate = AsyncMock()
    if side_effect is not None:
        mock_generate.side_effect = side_effect
    else:
        mock_generate.return_value = return_value
    client = MagicMock()
    client.aio.models.generate_content = mock_generate
    return client


@pytest.mark.asyncio
async def test_parse_track_metadata_no_api_key_returns_none():
    with patch("src.mus.domain.services.gemini_service.settings") as mock_settings:
        mock_settings.GEMINI_API_KEY = None
        result = await parse_track_metadata("Radiohead - Creep (Official Music Video)")
    assert result is None


@pytest.mark.asyncio
async def test_parse_track_metadata_success():
    with (
        patch("src.mus.domain.services.gemini_service.settings") as mock_settings,
        patch(
            "src.mus.domain.services.gemini_service._get_client",
            return_value=_mock_aio_client(_make_response("Radiohead", "Creep")),
        ),
    ):
        mock_settings.GEMINI_API_KEY = "test-key"
        mock_settings.GEMINI_MODEL = "gemini-3.1-flash-lite-preview"
        result = await parse_track_metadata(
            "Radiohead - Creep (Official Music Video)", "Radiohead"
        )
    assert result == ("Radiohead", "Creep")


@pytest.mark.asyncio
async def test_parse_track_metadata_no_channel():
    with (
        patch("src.mus.domain.services.gemini_service.settings") as mock_settings,
        patch(
            "src.mus.domain.services.gemini_service._get_client",
            return_value=_mock_aio_client(_make_response("Billie Eilish", "bad guy")),
        ),
    ):
        mock_settings.GEMINI_API_KEY = "test-key"
        mock_settings.GEMINI_MODEL = "gemini-3.1-flash-lite-preview"
        result = await parse_track_metadata("Billie Eilish - bad guy")
    assert result == ("Billie Eilish", "bad guy")


@pytest.mark.asyncio
async def test_parse_track_metadata_api_error_returns_none():
    with (
        patch("src.mus.domain.services.gemini_service.settings") as mock_settings,
        patch(
            "src.mus.domain.services.gemini_service._get_client",
            return_value=_mock_aio_client(side_effect=Exception("API error")),
        ),
    ):
        mock_settings.GEMINI_API_KEY = "test-key"
        mock_settings.GEMINI_MODEL = "gemini-3.1-flash-lite-preview"
        result = await parse_track_metadata("Artist - Track (Official Video)")
    assert result is None


@pytest.mark.asyncio
async def test_parse_track_metadata_empty_title_returns_none():
    with (
        patch("src.mus.domain.services.gemini_service.settings") as mock_settings,
        patch(
            "src.mus.domain.services.gemini_service._get_client",
            return_value=_mock_aio_client(_make_response("Some Artist", "")),
        ),
    ):
        mock_settings.GEMINI_API_KEY = "test-key"
        mock_settings.GEMINI_MODEL = "gemini-3.1-flash-lite-preview"
        result = await parse_track_metadata("garbage title")
    assert result is None


@pytest.mark.asyncio
async def test_parse_track_metadata_featuring_artist_preserved():
    """Featured artist name must be kept even though 'Ft.' marker is dropped."""
    with (
        patch("src.mus.domain.services.gemini_service.settings") as mock_settings,
        patch(
            "src.mus.domain.services.gemini_service._get_client",
            return_value=_mock_aio_client(_make_response("Anima, Sheera", "Moon")),
        ),
    ):
        mock_settings.GEMINI_API_KEY = "test-key"
        mock_settings.GEMINI_MODEL = "gemini-3.1-flash-lite-preview"
        result = await parse_track_metadata(
            "Anima Ft. Sheera - Moon (Original Mix)", "AnimaChannel"
        )
    assert result == ("Anima, Sheera", "Moon")
