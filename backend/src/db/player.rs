use anyhow::Result;
use rusqlite::{params, OptionalExtension};

use super::run;
use crate::{models::PlayerState, state::AppState};

pub async fn load_player_state(state: &AppState) -> Result<PlayerState> {
    run(state, |conn| {
        let player_state = conn
            .query_row(
                "SELECT current_track_id, progress_seconds, volume_level, is_muted,
                        is_playing, is_shuffle, is_repeat FROM playerstate WHERE id = 1",
                [],
                |row| {
                    Ok(PlayerState {
                        current_track_id: row.get(0)?,
                        progress_seconds: row.get(1)?,
                        volume_level: row.get(2)?,
                        is_muted: row.get(3)?,
                        is_playing: row.get(4)?,
                        is_shuffle: row.get(5)?,
                        is_repeat: row.get(6)?,
                    })
                },
            )
            .optional()?;
        Ok(player_state.unwrap_or_default())
    })
    .await
}

pub async fn save_player_state(state: &AppState, player_state: &PlayerState) -> Result<()> {
    let player_state = player_state.clone();
    run(state, move |conn| {
        conn.execute(
            "INSERT INTO playerstate (id, current_track_id, progress_seconds, volume_level, is_muted, is_playing, is_shuffle, is_repeat)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET current_track_id=excluded.current_track_id,
             progress_seconds=excluded.progress_seconds, volume_level=excluded.volume_level,
             is_muted=excluded.is_muted, is_playing=excluded.is_playing,
             is_shuffle=excluded.is_shuffle, is_repeat=excluded.is_repeat",
            params![
                player_state.current_track_id,
                player_state.progress_seconds,
                player_state.volume_level,
                player_state.is_muted,
                player_state.is_playing,
                player_state.is_shuffle,
                player_state.is_repeat,
            ],
        )?;
        Ok(())
    })
    .await
}
