import structlog

from mus.application.ports.track_repository import ITrackRepository
from mus.domain.track import Track

logger = structlog.get_logger()


class SearchTracksUseCase:
    def __init__(self, track_repository: ITrackRepository):
        self._track_repository = track_repository

    async def search_by_title(self, query: str) -> list[Track]:
        logger.info("searching_tracks_by_title", query=query)
        tracks = await self._track_repository.search_by_title(query)
        logger.info("search_completed", result_count=len(tracks))
        return tracks

    async def get_all(self) -> list[Track]:
        logger.info("getting_all_tracks")
        tracks = await self._track_repository.get_all()
        logger.info("get_all_completed", result_count=len(tracks))
        return tracks
