from abc import ABC, abstractmethod

from mus.domain.player_state import PlayerState


class IPlayerStateRepository(ABC):
    @abstractmethod
    async def save_state(self, state: PlayerState) -> None:
        """Save the current player state."""
        pass

    @abstractmethod
    async def load_state(self) -> PlayerState | None:
        """Load the current player state. Returns None if no state exists."""
        pass
