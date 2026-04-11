"""Google Gemini service for track metadata parsing.

Uses the google-genai SDK to extract artist/title from raw YouTube video titles.
Falls back gracefully if API key is missing or request fails.
"""

import logging

from google import genai
from google.genai import types
from pydantic import BaseModel

from src.mus.config import settings

logger = logging.getLogger(__name__)

_SYSTEM_PROMPT = (
    "Extract artist(s) and title from a YouTube video title. "
    "Strip junk words: Official Video/Audio/Music Video, Lyric Video, Lyrics, HD, 4K, HQ, "
    "Remastered, Live, Audio Only, Music Video, Fan Made, Visualizer, Topic, Extended Mix, Original Mix, "
    "Radio Edit, Album/Single Version, Provided to YouTube, Auto-generated. "
    "Multiple artists (feat./ft./featuring/x/&/and) → comma-separated in 'artist', "
    "do NOT include feat./ft./featuring keyword itself but DO keep the featured artist name. "
    "Artist comes from the video title first; use channel name only if the title gives no artist."
)

_client: genai.Client | None = None


def _get_client() -> genai.Client:
    global _client
    if _client is None:
        _client = genai.Client(api_key=settings.GEMINI_API_KEY)
    return _client


class _TrackMetadata(BaseModel):
    artist: str
    title: str


async def parse_track_metadata(
    raw_title: str,
    channel_name: str = "",
) -> tuple[str, str] | None:
    """Use Gemini LLM to extract (artist, title) from a raw YouTube title.

    Returns a (artist, title) tuple on success, or None if the API key is
    missing / request fails (caller should fall back to regex).
    """
    if not settings.GEMINI_API_KEY:
        return None

    user_message = f'Video title: "{raw_title}"'
    if channel_name:
        user_message += f'\nChannel name: "{channel_name}"'

    try:
        response = await _get_client().aio.models.generate_content(
            model=settings.GEMINI_MODEL,
            contents=user_message,
            config=types.GenerateContentConfig(
                system_instruction=_SYSTEM_PROMPT,
                response_mime_type="application/json",
                response_schema=_TrackMetadata,
                max_output_tokens=100,
            ),
        )
        result = response.parsed
    except Exception as exc:
        logger.warning("Gemini request failed, skipping LLM parsing: %s", exc)
        return None

    if not isinstance(result, _TrackMetadata) or not result.title:
        return None

    return result.artist, result.title
