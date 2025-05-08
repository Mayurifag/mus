from dataclasses import dataclass


@dataclass
class PlayerState:
    current_track_id: int | None
    progress_seconds: float
    volume_level: float
    is_muted: bool

    @classmethod
    def create_default(cls) -> "PlayerState":
        return cls(
            current_track_id=None,
            progress_seconds=0.0,
            volume_level=1.0,
            is_muted=False,
        )
