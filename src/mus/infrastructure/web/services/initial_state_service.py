from mus.application.ports.track_repository import ITrackRepository
from mus.application.use_cases.load_player_state import LoadPlayerStateUseCase


class InitialStateService:
    def __init__(
        self,
        load_player_state_use_case: LoadPlayerStateUseCase,
        track_repository: ITrackRepository,
    ):
        self._load_player_state = load_player_state_use_case
        self._track_repository = track_repository

    async def get_initial_state(self) -> dict:
        state = await self._load_player_state.execute()
        tracks = await self._track_repository.get_all()

        player_state_dict = None
        if state is not None and state.current_track_id is not None:
            if any(track.id == state.current_track_id for track in tracks):
                # State is valid and refers to a currently available track
                player_state_dict = {
                    "current_track_id": state.current_track_id,
                    "progress_seconds": state.progress_seconds,
                    "volume_level": state.volume_level,
                    "is_muted": state.is_muted,
                }
            # else: The saved track ID is no longer in the main list, ignore saved state

        if player_state_dict is None:
            # Fallback logic: Use defaults or first track if available
            if tracks:
                # Default to the first track in the list
                player_state_dict = {
                    "current_track_id": tracks[0].id,
                    "progress_seconds": 0.0,
                    "volume_level": 1.0,
                    "is_muted": False,
                }
            else:
                # No tracks available, use default empty state
                player_state_dict = {
                    "current_track_id": None,
                    "progress_seconds": 0.0,
                    "volume_level": 1.0,
                    "is_muted": False,
                }

        return {"tracks": tracks, "player_state": player_state_dict}
