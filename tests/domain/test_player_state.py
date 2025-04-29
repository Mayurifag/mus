from mus.domain.player_state import PlayerState


def test_create_default_player_state():
    state = PlayerState.create_default()
    assert state.current_track_id is None
    assert state.progress_seconds == 0.0
    assert state.volume_level == 1.0
    assert state.is_muted is False


def test_create_player_state_with_values():
    state = PlayerState(
        current_track_id=1, progress_seconds=30.5, volume_level=0.75, is_muted=True
    )
    assert state.current_track_id == 1
    assert state.progress_seconds == 30.5
    assert state.volume_level == 0.75
    assert state.is_muted is True
