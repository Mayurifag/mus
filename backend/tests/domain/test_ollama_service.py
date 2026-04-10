import json
from unittest.mock import AsyncMock, MagicMock, patch

import httpx
import pytest

from src.mus.domain.services.ollama_service import parse_track_metadata


def _make_response(artist: str, title: str) -> MagicMock:
    """Build a fake httpx.Response containing a valid Ollama chat response."""
    content_json = json.dumps({"artist": artist, "title": title})
    body = {"message": {"content": content_json}}
    mock_resp = MagicMock()
    mock_resp.json.return_value = body
    mock_resp.raise_for_status = MagicMock()
    return mock_resp


def _mock_client(post_return=None, post_side_effect=None) -> MagicMock:
    client = AsyncMock()
    if post_side_effect is not None:
        client.post = AsyncMock(side_effect=post_side_effect)
    else:
        client.post = AsyncMock(return_value=post_return)
    return client


@pytest.mark.asyncio
async def test_parse_track_metadata_success():
    mock_resp = _make_response("Radiohead", "Creep")
    with patch(
        "src.mus.domain.services.ollama_service._get_client",
        return_value=_mock_client(mock_resp),
    ):
        result = await parse_track_metadata(
            "Radiohead - Creep (Official Music Video)", "Radiohead"
        )
    assert result == ("Radiohead", "Creep")


@pytest.mark.asyncio
async def test_parse_track_metadata_no_channel():
    mock_resp = _make_response("Billie Eilish", "bad guy")
    with patch(
        "src.mus.domain.services.ollama_service._get_client",
        return_value=_mock_client(mock_resp),
    ):
        result = await parse_track_metadata("Billie Eilish - bad guy")
    assert result == ("Billie Eilish", "bad guy")


@pytest.mark.asyncio
async def test_parse_track_metadata_ollama_unavailable():
    with patch(
        "src.mus.domain.services.ollama_service._get_client",
        return_value=_mock_client(post_side_effect=httpx.ConnectError("refused")),
    ):
        result = await parse_track_metadata("Artist - Track (Official Video)")
    assert result is None


@pytest.mark.asyncio
async def test_parse_track_metadata_timeout():
    with patch(
        "src.mus.domain.services.ollama_service._get_client",
        return_value=_mock_client(post_side_effect=httpx.TimeoutException("timeout")),
    ):
        result = await parse_track_metadata("Artist - Track")
    assert result is None


@pytest.mark.asyncio
async def test_parse_track_metadata_http_error():
    mock_resp = MagicMock()
    mock_resp.raise_for_status.side_effect = httpx.HTTPStatusError(
        "500", request=MagicMock(), response=MagicMock(status_code=500)
    )
    with patch(
        "src.mus.domain.services.ollama_service._get_client",
        return_value=_mock_client(mock_resp),
    ):
        result = await parse_track_metadata("Artist - Track")
    assert result is None


@pytest.mark.asyncio
async def test_parse_track_metadata_invalid_json():
    mock_resp = MagicMock()
    mock_resp.raise_for_status = MagicMock()
    mock_resp.json.return_value = {"message": {"content": "not json at all"}}
    with patch(
        "src.mus.domain.services.ollama_service._get_client",
        return_value=_mock_client(mock_resp),
    ):
        result = await parse_track_metadata("Artist - Track")
    assert result is None


@pytest.mark.asyncio
async def test_parse_track_metadata_empty_title_returns_none():
    mock_resp = _make_response("Some Artist", "")
    with patch(
        "src.mus.domain.services.ollama_service._get_client",
        return_value=_mock_client(mock_resp),
    ):
        result = await parse_track_metadata("garbage title")
    assert result is None


@pytest.mark.asyncio
async def test_parse_track_metadata_featuring_artist_preserved():
    """Featured artist name must be kept even though 'Ft.' marker is dropped."""
    mock_resp = _make_response("Anima, Sheera", "Moon")
    with patch(
        "src.mus.domain.services.ollama_service._get_client",
        return_value=_mock_client(mock_resp),
    ):
        result = await parse_track_metadata(
            "Anima Ft. Sheera - Moon (Original Mix)", "AnimaChannel"
        )
    assert result == ("Anima, Sheera", "Moon")
