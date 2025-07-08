from src.mus.application.dtos.track import TrackListDTO
from src.mus.domain.entities.track import Track


def create_track_dto_with_covers(track: Track) -> TrackListDTO:
    """
    Create TrackListDTO with proper cover URLs and cache-busting parameters.

    This utility ensures consistent cover URL generation across all SSE events,
    preventing cache issues when track metadata is updated.

    Args:
        track: The Track entity to convert

    Returns:
        TrackListDTO with proper cover URLs if the track has covers
    """
    track_dto = TrackListDTO.model_validate(track)
    if track.has_cover:
        cover_base = f"/api/v1/tracks/{track.id}/covers"
        cache_param = f"?v={track.updated_at}"
        track_dto.cover_small_url = f"{cover_base}/small.webp{cache_param}"
        track_dto.cover_original_url = f"{cover_base}/original.webp{cache_param}"
    return track_dto
