"""Ollama LLM service for track metadata parsing.

Calls a local Ollama instance to extract artist/title from raw YouTube video
titles.  Falls back gracefully if Ollama is unavailable or returns garbage.
"""

import json
import logging

import httpx

from src.mus.config import settings

logger = logging.getLogger(__name__)

_MODEL = "qwen3.5:2b"
_TIMEOUT = 20.0

_SYSTEM_PROMPT = (
    "Extract artist(s) and title from a YouTube video title. "
    "Strip junk words: Official Video/Audio/Music Video, Lyric Video, Lyrics, HD, 4K, HQ, "
    "Remastered, Live, Audio Only, Music Video, Fan Made, Visualizer, Topic, Extended Mix, Original Mix, "
    "Radio Edit, Album/Single Version, Provided to YouTube, Auto-generated. "
    "Multiple artists (feat./ft./featuring/x/&/and) → comma-separated in 'artist', "
    "do NOT include feat./ft./featuring keyword itself but DO keep the featured artist name. "
    "Artist comes from the video title first; use channel name only if the title gives no artist. "
    'Example 1: "Anima Ft. Sheera - Moon (Original Mix)" → {"artist":"Anima, Sheera","title":"Moon"}. '
    'Example 2: "Eminem - Love The Way You Lie featuring Rihanna" → {"artist":"Eminem, Rihanna","title":"Love The Way You Lie"}. '
    'Respond ONLY with JSON: {"artist":"...","title":"..."}.'
)

_http_client: httpx.AsyncClient | None = None


def _get_client() -> httpx.AsyncClient:
    global _http_client
    if _http_client is None or _http_client.is_closed:
        _http_client = httpx.AsyncClient(timeout=_TIMEOUT)
    return _http_client


async def parse_track_metadata(
    raw_title: str,
    channel_name: str = "",
) -> tuple[str, str] | None:
    """Use Ollama LLM to extract (artist, title) from a raw YouTube title.

    Returns a (artist, title) tuple on success, or None if Ollama is
    unavailable / returns unparseable output (caller should fall back to regex).
    """
    user_message = f'Video title: "{raw_title}"'
    if channel_name:
        user_message += f'\nChannel name: "{channel_name}"'

    payload = {
        "model": _MODEL,
        "think": False,
        "messages": [
            {"role": "system", "content": _SYSTEM_PROMPT},
            {"role": "user", "content": user_message},
        ],
        "stream": False,
        "format": "json",
        "options": {"num_ctx": 512, "num_predict": 100},
    }

    try:
        response = await _get_client().post(
            f"{settings.OLLAMA_URL}/api/chat",
            json=payload,
        )
        response.raise_for_status()
    except (httpx.ConnectError, httpx.TimeoutException) as exc:
        logger.debug("Ollama unavailable, skipping LLM parsing: %s", exc)
        return None
    except httpx.HTTPStatusError as exc:
        logger.warning(
            "Ollama returned HTTP %s, skipping LLM parsing", exc.response.status_code
        )
        return None

    try:
        data = response.json()
        content = data["message"]["content"]
        parsed = json.loads(content)
        artist = str(parsed.get("artist", "")).strip()
        title = str(parsed.get("title", "")).strip()
    except (KeyError, ValueError, json.JSONDecodeError) as exc:
        logger.warning("Could not parse Ollama response: %s", exc)
        return None

    if not title:
        return None

    return artist, title
