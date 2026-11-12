use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use anyhow::Result;
use rusqlite::{params, Connection};

use super::run;
use crate::{
    categories::{category_display_name, inferred_tag_names, CATEGORIES},
    models::{Tag, Track},
    state::AppState,
};

pub(super) fn ensure_seed_tags(conn: &Connection) -> Result<()> {
    for (name, display_name) in CATEGORIES {
        conn.execute(
            "INSERT INTO tag (name, display_name) VALUES (?1, ?2) ON CONFLICT(name) DO UPDATE SET display_name=excluded.display_name",
            params![name, display_name],
        )?;
    }
    Ok(())
}

pub(super) fn remove_deprecated_tags(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM tag WHERE name = 'gachi'", [])?;
    Ok(())
}

pub(super) fn ensure_inferred_category_tags(conn: &Connection) -> Result<()> {
    let tracks = {
        let mut stmt = conn.prepare("SELECT id, title FROM track WHERE tags_finalized = 0")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (track_id, title) in tracks {
        for name in inferred_tag_names(&title) {
            conn.execute(
                "INSERT OR IGNORE INTO track_tag (track_id, tag_id) SELECT ?1, id FROM tag WHERE name = ?2",
                params![track_id, name],
            )?;
        }
    }
    Ok(())
}

pub async fn set_track_tags(
    state: &AppState,
    track_id: &str,
    tag_names: &[String],
    finalize: bool,
) -> Result<()> {
    let track_id = track_id.to_owned();
    let tag_names = tag_names.to_vec();
    run(state, move |conn| {
        let tx = conn.transaction()?;
        if finalize {
            tx.execute(
                "UPDATE track SET tags_finalized = 1 WHERE id = ?1",
                params![track_id],
            )?;
        }
        set_track_tags_conn(&tx, &track_id, &tag_names)?;
        tx.commit()?;
        Ok(())
    })
    .await
}

pub async fn add_inferred_track_tags(
    state: &AppState,
    track_id: &str,
    tag_names: &[String],
) -> Result<()> {
    let track_id = track_id.to_owned();
    let tag_names = tag_names.to_vec();
    run(state, move |conn| {
        for name in valid_tag_names(&tag_names) {
            conn.execute(
                "INSERT OR IGNORE INTO track_tag (track_id, tag_id)
                 SELECT track.id, tag.id FROM track JOIN tag ON tag.name = ?2
                 WHERE track.id = ?1 AND track.tags_finalized = 0",
                params![track_id, name],
            )?;
        }
        Ok(())
    })
    .await
}

/// Atomically finalizes categories if the track still has the stored publication path.
///
/// Returns `false` without changes if the ID/path pair no longer matches. Unknown
/// tag names and duplicates are ignored. This checks the database, not the filesystem.
///
/// # Errors
/// Returns worker, connection-lock, or SQLite errors.
///
/// # Panics
/// Panics if polled outside a Tokio runtime.
///
/// ```no_run
/// # async fn example(state: &mus_backend::state::AppState, id: &str, published: &std::path::Path) -> anyhow::Result<()> {
/// let applied = mus_backend::db::set_download_tags(state, id, published, &["ost".into()]).await?;
/// if !applied {
///     // The registered track moved or disappeared; do not change another track's tags.
/// }
/// # Ok(())
/// # }
/// ```
pub async fn set_download_tags(
    state: &AppState,
    track_id: &str,
    published_path: &Path,
    tag_names: &[String],
) -> Result<bool> {
    let track_id = track_id.to_owned();
    let published_path = published_path.to_path_buf();
    let tag_names = tag_names.to_vec();
    run(state, move |conn| {
        let tx = conn.transaction()?;
        let changed = tx.execute(
            "UPDATE track SET tags_finalized = 1 WHERE id = ?1 AND file_path = ?2",
            params![track_id, published_path.to_string_lossy().as_ref()],
        )?;
        if changed == 0 {
            return Ok(false);
        }
        set_track_tags_conn(&tx, &track_id, &tag_names)?;
        tx.commit()?;
        Ok(true)
    })
    .await
}

/// Reports whether an explicit category choice has disabled title-based inference.
///
/// # Errors
/// Returns an error for a missing track, worker failure, poisoned connection lock,
/// or SQLite failure. See the [module example](crate::db).
///
/// # Panics
/// Panics if polled outside a Tokio runtime.
pub async fn track_tags_finalized(state: &AppState, track_id: &str) -> Result<bool> {
    let track_id = track_id.to_owned();
    run(state, move |conn| {
        Ok(conn.query_row(
            "SELECT tags_finalized FROM track WHERE id = ?1",
            params![track_id],
            |row| row.get(0),
        )?)
    })
    .await
}

fn set_track_tags_conn(conn: &Connection, track_id: &str, tag_names: &[String]) -> Result<()> {
    conn.execute(
        "DELETE FROM track_tag WHERE track_id = ?1",
        params![track_id],
    )?;
    for name in valid_tag_names(tag_names) {
        conn.execute(
            "INSERT INTO track_tag (track_id, tag_id) SELECT ?1, id FROM tag WHERE name = ?2",
            params![track_id, name],
        )?;
    }
    Ok(())
}

fn valid_tag_names(tag_names: &[String]) -> impl Iterator<Item = &str> {
    let mut seen = HashSet::new();
    tag_names
        .iter()
        .map(String::as_str)
        .filter(move |name| category_display_name(name).is_some() && seen.insert(*name))
}

pub(super) fn load_track_tags(conn: &Connection, tracks: &mut [Track]) -> Result<()> {
    if tracks.is_empty() {
        return Ok(());
    }
    // One JSON parameter avoids SQLite's variable limit for large libraries.
    let track_ids =
        serde_json::to_string(&tracks.iter().map(|track| &track.id).collect::<Vec<_>>())?;
    let mut stmt = conn.prepare(
        "SELECT track_tag.track_id, tag.name, tag.display_name FROM tag JOIN track_tag ON track_tag.tag_id = tag.id WHERE track_tag.track_id IN (SELECT value FROM json_each(?1)) ORDER BY track_tag.track_id, tag.id"
    )?;
    let rows = stmt.query_map([track_ids], |row| {
        Ok((
            row.get::<_, String>(0)?,
            Tag {
                name: row.get(1)?,
                display_name: row.get(2)?,
            },
        ))
    })?;
    let mut tags_by_track: HashMap<String, Vec<Tag>> = HashMap::new();
    for row in rows {
        let (track_id, tag) = row?;
        tags_by_track.entry(track_id).or_default().push(tag);
    }
    for track in tracks {
        track.tags = tags_by_track.remove(&track.id).unwrap_or_default();
    }
    Ok(())
}

pub(super) fn track_tags(conn: &Connection, track_id: &str) -> Result<Vec<Tag>> {
    let mut stmt = conn.prepare(
        "SELECT tag.name, tag.display_name FROM tag JOIN track_tag ON track_tag.tag_id = tag.id WHERE track_tag.track_id = ?1 ORDER BY tag.id",
    )?;
    let tags = stmt
        .query_map(params![track_id], |row| {
            Ok(Tag {
                name: row.get(0)?,
                display_name: row.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tags)
}
