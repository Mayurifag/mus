//! Library discovery, identity, and category persistence integration tests.

use std::{fs, path::Path, process::Command, time::Duration};

use id3::{Tag, TagLike, Version};
use mus_backend::{
    db,
    files::{file_content_hash, file_signature, inode},
    scanner::scan_music_dir,
};
use rusqlite::{params, Connection};
use tempfile::TempDir;
use tokio::time::timeout;

mod common;
use common::test_state;

fn make_state() -> (TempDir, mus_backend::state::AppState) {
    let tmp = TempDir::new().unwrap();
    let data_dir = tmp.path().to_path_buf();
    fs::create_dir_all(data_dir.join("music")).unwrap();
    let state = test_state(data_dir, Connection::open_in_memory().unwrap());
    (tmp, state)
}

fn set_mtime(path: &std::path::Path, seconds: u64) {
    fs::set_times(
        path,
        fs::FileTimes::new().set_modified(std::time::UNIX_EPOCH + Duration::from_secs(seconds)),
    )
    .unwrap();
}

fn create_mp3(path: &Path, title: &str, artist: &str) {
    let output = Command::new("ffmpeg")
        .args([
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:duration=0.1",
            "-c:a",
            "libmp3lame",
            "-q:a",
            "9",
            "-metadata",
            &format!("title={title}"),
            "-metadata",
            &format!("artist={artist}"),
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "ffmpeg failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn write_silent_wav(path: &Path) {
    let sample_rate = 8_000u32;
    let channels = 1u16;
    let bits_per_sample = 16u16;
    let samples = sample_rate / 10;
    let data_len = samples * channels as u32 * bits_per_sample as u32 / 8;
    let mut data = Vec::new();
    data.extend_from_slice(b"RIFF");
    data.extend_from_slice(&(36 + data_len).to_le_bytes());
    data.extend_from_slice(b"WAVEfmt ");
    data.extend_from_slice(&16u32.to_le_bytes());
    data.extend_from_slice(&1u16.to_le_bytes());
    data.extend_from_slice(&channels.to_le_bytes());
    data.extend_from_slice(&sample_rate.to_le_bytes());
    data.extend_from_slice(
        &(sample_rate * channels as u32 * bits_per_sample as u32 / 8).to_le_bytes(),
    );
    data.extend_from_slice(&(channels * bits_per_sample / 8).to_le_bytes());
    data.extend_from_slice(&bits_per_sample.to_le_bytes());
    data.extend_from_slice(b"data");
    data.extend_from_slice(&data_len.to_le_bytes());
    data.resize(data.len() + data_len as usize, 0);
    fs::write(path, data).unwrap();
}

#[test]
fn init_db_migrates_existing_schema() {
    let mut conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        r#"
        CREATE TABLE track (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            artist TEXT NOT NULL,
            duration INTEGER NOT NULL,
            file_path TEXT NOT NULL UNIQUE,
            added_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL DEFAULT 0,
            has_cover BOOLEAN NOT NULL DEFAULT 0,
            inode INTEGER,
            content_hash TEXT,
            processing_status TEXT NOT NULL DEFAULT 'COMPLETE',
            last_error JSON
        );
        CREATE TABLE playerstate (
            id INTEGER PRIMARY KEY DEFAULT 1,
            current_track_id INTEGER,
            progress_seconds REAL NOT NULL DEFAULT 0,
            volume_level REAL NOT NULL DEFAULT 1,
            is_muted BOOLEAN NOT NULL DEFAULT 0,
            is_shuffle BOOLEAN NOT NULL DEFAULT 0,
            is_repeat BOOLEAN NOT NULL DEFAULT 0
        );
        "#,
    )
    .unwrap();

    db::init_db(&mut conn).unwrap();

    let file_signature: String = conn
        .query_row(
            "SELECT name FROM pragma_table_info('track') WHERE name = 'file_signature'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(file_signature, "file_signature");
}

#[test]
fn category_choice_migration_keeps_existing_track_and_tags() {
    let (_tmp, state) = make_state();
    let id = "1111111111111111111111111111111111111111111111111111111111111111";
    let mut conn = state.db.lock().unwrap();
    conn.execute_batch(
        "ALTER TABLE track DROP COLUMN cover_thumbhash;
         ALTER TABLE track DROP COLUMN tags_finalized;
         ALTER TABLE track ADD COLUMN content_hash TEXT NOT NULL DEFAULT '';
         PRAGMA user_version = 5;",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO track (id,title,artist,duration,file_path,added_at,updated_at,content_hash) VALUES (?1,'Song','Artist',1,'/old/song.mp3',1,1,?1)",
        params![id],
    ).unwrap();
    conn.execute(
        "INSERT INTO track_tag (track_id,tag_id) SELECT ?1,id FROM tag WHERE name='ost'",
        params![id],
    )
    .unwrap();
    db::init_db(&mut conn).unwrap();
    let row: (String, bool, String) = conn.query_row(
        "SELECT track.title, track.tags_finalized, tag.name FROM track JOIN track_tag ON track_tag.track_id=track.id JOIN tag ON tag.id=track_tag.tag_id WHERE track.id=?1",
        params![id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
    ).unwrap();
    assert_eq!(row, ("Song".into(), false, "ost".into()));
}

#[tokio::test]
async fn identity_migration_preserves_tracks_categories_order_and_player_state() {
    let (_tmp, state) = make_state();
    let first_id = "b".repeat(64);
    let second_id = "a".repeat(64);
    {
        let mut conn = state.db.lock().unwrap();
        conn.execute_batch(
            "ALTER TABLE track DROP COLUMN cover_thumbhash;
             ALTER TABLE track ADD COLUMN content_hash TEXT NOT NULL DEFAULT '';
             PRAGMA user_version = 6;",
        )
        .unwrap();
        for (id, title, added_at) in [(&first_id, "First", 10), (&second_id, "Second", 20)] {
            conn.execute(
                "INSERT INTO track (id,title,artist,duration,file_path,added_at,updated_at,content_hash,tags_finalized)
                 VALUES (?1,?2,'Artist',3,?2,?3,?3,?1,1)",
                params![id,title,added_at],
            ).unwrap();
        }
        conn.execute(
            "INSERT INTO track_tag (track_id,tag_id) SELECT ?1,id FROM tag WHERE name='ost'",
            [&first_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO playerstate (id,current_track_id,progress_seconds,is_shuffle) VALUES (1,?1,7.5,1)",
            [&second_id],
        ).unwrap();
        for _ in 0..2 {
            db::init_db(&mut conn).unwrap();
        }
        let column_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM pragma_table_info('track') WHERE name='content_hash'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(column_count, 0);
        let rows: Vec<(String, String, i64)> = conn
            .prepare("SELECT id,title,added_at FROM track ORDER BY rowid")
            .unwrap()
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            rows,
            [
                (first_id.clone(), "First".into(), 10),
                (second_id.clone(), "Second".into(), 20)
            ]
        );
        let player: (String, f64, bool) = conn
            .query_row(
                "SELECT current_track_id,progress_seconds,is_shuffle FROM playerstate",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(player, (second_id, 7.5, true));
        assert_eq!(
            conn.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |row| {
                row.get::<_, i64>(0)
            })
            .unwrap(),
            0
        );
    }
    let first = db::get_track(&state, &first_id).await.unwrap().unwrap();
    assert_eq!(first.tags[0].name, "ost");
    assert!(db::track_tags_finalized(&state, &first_id).await.unwrap());
}

#[tokio::test]
async fn confirmed_download_tags_survive_restart_and_changed_file() {
    let (_tmp, state) = make_state();
    let chosen = state.music_dir.join("Artist - Song (cover).mp3");
    let legacy = state.music_dir.join("Artist - Old (cover).mp3");
    create_mp3(&chosen, "Song (cover)", "Artist");
    create_mp3(&legacy, "Old (cover)", "Artist");
    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).await.unwrap();
    let chosen_track = tracks
        .iter()
        .find(|track| track.file_path == chosen.to_string_lossy())
        .unwrap();
    let legacy_track = tracks
        .iter()
        .find(|track| track.file_path == legacy.to_string_lossy())
        .unwrap();
    assert_eq!(chosen_track.tags[0].name, "cover");
    assert!(
        !db::set_download_tags(&state, &legacy_track.id, &chosen, &["ost".into()])
            .await
            .unwrap()
    );
    assert!(!db::track_tags_finalized(&state, &legacy_track.id)
        .await
        .unwrap());
    assert!(
        db::set_download_tags(&state, &chosen_track.id, &chosen, &[])
            .await
            .unwrap()
    );
    {
        let mut conn = state.db.lock().unwrap();
        db::init_db(&mut conn).unwrap();
    }
    assert!(db::get_track(&state, &chosen_track.id)
        .await
        .unwrap()
        .unwrap()
        .tags
        .is_empty());
    assert_eq!(
        db::get_track(&state, &legacy_track.id)
            .await
            .unwrap()
            .unwrap()
            .tags[0]
            .name,
        "cover"
    );

    let mut tag = Tag::read_from_path(&chosen).unwrap();
    tag.set_album("Updated album");
    tag.write_to_path(&chosen, Version::Id3v23).unwrap();
    scan_music_dir(state.clone()).await.unwrap();
    let updated = db::list_tracks(&state)
        .await
        .unwrap()
        .into_iter()
        .find(|track| track.file_path == chosen.to_string_lossy())
        .unwrap();
    assert_ne!(updated.id, chosen_track.id);
    assert!(db::track_tags_finalized(&state, &updated.id).await.unwrap());
    assert!(updated.tags.is_empty());
}

#[tokio::test]
async fn scan_music_dir_finds_nested_audio_files() {
    let (_tmp, state) = make_state();
    let nested = state.music_dir.join("artist").join("album");
    fs::create_dir_all(&nested).unwrap();
    let path = nested.join("song.wav");
    write_silent_wav(&path);

    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title, "song");
    assert_eq!(tracks[0].file_path, path.to_string_lossy());
}

#[tokio::test]
async fn scan_music_dir_skips_unparseable_audio_files() {
    let (_tmp, state) = make_state();
    fs::write(state.music_dir.join("broken.mp3"), b"not real audio").unwrap();

    scan_music_dir(state.clone()).await.unwrap();

    assert!(db::list_tracks(&state).await.unwrap().is_empty());
}

#[tokio::test]
async fn exact_copies_do_not_change_the_registered_path_or_repeat_added_events() {
    let (_tmp, state) = make_state();
    let original = state.music_dir.join("Ace of Base - Happy Nation.mp3");
    create_mp3(&original, "Happy Nation", "Ace of Base");
    scan_music_dir(state.clone()).await.unwrap();
    let registered = db::list_tracks(&state).await.unwrap().remove(0);
    let duplicate = state.music_dir.join("another copy.mp3");
    fs::copy(&original, &duplicate).unwrap();
    let before = fs::read(&duplicate).unwrap();
    let mut events = state.events.subscribe();

    for _ in 0..2 {
        scan_music_dir(state.clone()).await.unwrap();
        let tracks = db::list_tracks(&state).await.unwrap();
        assert_eq!(tracks.len(), 1);
        assert_eq!(tracks[0].id, registered.id);
        assert_eq!(tracks[0].file_path, registered.file_path);
        assert_eq!(tracks[0].added_at, registered.added_at);
        assert_eq!(fs::read(&duplicate).unwrap(), before);
        assert!(original.is_file());
        while let Ok(event) = events.try_recv() {
            assert_ne!(
                event.action_key.as_deref(),
                Some("track_added"),
                "Exact copies are not new tracks"
            );
        }
    }
}

#[tokio::test]
async fn replacing_a_registered_file_with_a_copy_keeps_the_original_entry() {
    let (_tmp, state) = make_state();
    let original = state.music_dir.join("original.wav");
    let replaced = state.music_dir.join("replaced.wav");
    write_silent_wav(&original);
    let mut different_audio = fs::read(&original).unwrap();
    *different_audio.last_mut().unwrap() = 1;
    fs::write(&replaced, different_audio).unwrap();
    scan_music_dir(state.clone()).await.unwrap();

    let original_id = file_content_hash(&original).unwrap();
    let replaced_id = file_content_hash(&replaced).unwrap();
    assert!(
        db::set_download_tags(&state, &original_id, &original, &["ost".into()])
            .await
            .unwrap()
    );
    assert!(
        db::set_download_tags(&state, &replaced_id, &replaced, &["cover".into()])
            .await
            .unwrap()
    );
    let mut events = state.events.subscribe();
    let original_bytes = fs::read(&original).unwrap();
    fs::copy(&original, &replaced).unwrap();

    scan_music_dir(state.clone()).await.unwrap();
    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].id, original_id);
    assert_eq!(tracks[0].file_path, original.to_string_lossy());
    assert_eq!(tracks[0].tags.len(), 1);
    assert_eq!(tracks[0].tags[0].name, "ost");
    assert!(db::track_tags_finalized(&state, &original_id)
        .await
        .unwrap());
    assert_eq!(fs::read(original).unwrap(), original_bytes);
    assert_eq!(fs::read(replaced).unwrap(), original_bytes);
    let event = events.try_recv().unwrap();
    assert_eq!(event.action_key.as_deref(), Some("track_deleted"));
    assert_eq!(event.action_payload.unwrap()["id"], replaced_id);
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn moving_a_registered_file_over_another_keeps_the_moved_track() {
    let (_tmp, state) = make_state();
    let original = state.music_dir.join("original.wav");
    let replaced = state.music_dir.join("replaced.wav");
    write_silent_wav(&original);
    let mut different_audio = fs::read(&original).unwrap();
    *different_audio.last_mut().unwrap() = 1;
    fs::write(&replaced, different_audio).unwrap();
    scan_music_dir(state.clone()).await.unwrap();

    let original_id = file_content_hash(&original).unwrap();
    let replaced_id = file_content_hash(&replaced).unwrap();
    assert!(
        db::set_download_tags(&state, &original_id, &original, &["ost".into()])
            .await
            .unwrap()
    );
    let mut events = state.events.subscribe();
    fs::remove_file(&replaced).unwrap();
    fs::rename(&original, &replaced).unwrap();

    scan_music_dir(state.clone()).await.unwrap();
    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].id, original_id);
    assert_eq!(tracks[0].file_path, replaced.to_string_lossy());
    assert_eq!(tracks[0].tags[0].name, "ost");
    assert!(db::track_tags_finalized(&state, &original_id)
        .await
        .unwrap());
    assert_eq!(file_content_hash(&replaced).unwrap(), original_id);
    let deleted = events.try_recv().unwrap();
    assert_eq!(deleted.action_key.as_deref(), Some("track_deleted"));
    assert_eq!(deleted.action_payload.unwrap()["id"], replaced_id);
    let updated = events.try_recv().unwrap();
    assert_eq!(updated.action_key.as_deref(), Some("track_updated"));
    assert_eq!(updated.action_payload.unwrap()["previous_id"], original_id);
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn a_new_file_at_the_vacated_path_does_not_take_the_moved_tracks_identity() {
    for registered_destination in [true, false] {
        check_move_with_new_source(registered_destination).await;
    }
}

async fn check_move_with_new_source(registered_destination: bool) {
    let (_tmp, state) = make_state();
    let a = state.music_dir.join("a.wav");
    let b = state.music_dir.join("b.wav");
    write_silent_wav(&a);
    let mut audio = fs::read(&a).unwrap();
    *audio.last_mut().unwrap() = 1;
    fs::write(&b, &audio).unwrap();
    if !registered_destination {
        fs::remove_file(&a).unwrap();
    }
    scan_music_dir(state.clone()).await.unwrap();
    let displaced_id = registered_destination.then(|| file_content_hash(&a).unwrap());
    let moved_id = file_content_hash(&b).unwrap();
    assert!(
        db::set_download_tags(&state, &moved_id, &b, &["ost".into()])
            .await
            .unwrap()
    );
    state
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO playerstate (id, current_track_id, progress_seconds) VALUES (1, ?1, 0.05)",
            [&moved_id],
        )
        .unwrap();
    let mut events = state.events.subscribe();

    if registered_destination {
        fs::remove_file(&a).unwrap();
    }
    fs::rename(&b, &a).unwrap();
    *audio.last_mut().unwrap() = 2;
    fs::write(&b, &audio).unwrap();
    scan_music_dir(state.clone()).await.unwrap();
    scan_music_dir(state.clone()).await.unwrap();

    let new_id = file_content_hash(&b).unwrap();
    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks.len(), 2);
    if let Some(id) = displaced_id {
        assert!(db::get_track(&state, &id).await.unwrap().is_none());
        let event = events.try_recv().unwrap();
        assert_eq!(event.action_key.as_deref(), Some("track_deleted"));
        assert_eq!(event.action_payload.unwrap()["id"], id);
    }
    let moved = db::get_track(&state, &moved_id).await.unwrap().unwrap();
    assert_eq!(moved.file_path, a.to_string_lossy());
    assert_eq!(moved.tags[0].name, "ost");
    assert!(db::track_tags_finalized(&state, &moved_id).await.unwrap());
    let new = db::get_track(&state, &new_id).await.unwrap().unwrap();
    assert_eq!(new.file_path, b.to_string_lossy());
    assert!(new.tags.is_empty());
    assert!(!db::track_tags_finalized(&state, &new_id).await.unwrap());
    let playback = state
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT current_track_id, progress_seconds FROM playerstate WHERE id = 1",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)),
        )
        .unwrap();
    assert_eq!(playback, (moved_id.clone(), 0.05));
    for (action, id) in [("track_updated", moved_id), ("track_added", new_id)] {
        let event = events.try_recv().unwrap();
        assert_eq!(event.action_key.as_deref(), Some(action));
        assert_eq!(event.action_payload.unwrap()["id"], id);
    }
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn rotating_registered_paths_preserves_content_identity_and_playback() {
    for count in [2, 3] {
        let (_tmp, state) = make_state();
        let paths: Vec<_> = ["first.wav", "second.wav", "third.wav"][..count]
            .iter()
            .map(|name| state.music_dir.join(name))
            .collect();
        for (sample, path) in paths.iter().enumerate() {
            write_silent_wav(path);
            let mut bytes = fs::read(path).unwrap();
            *bytes.last_mut().unwrap() = u8::try_from(sample).unwrap();
            fs::write(path, bytes).unwrap();
        }
        scan_music_dir(state.clone()).await.unwrap();
        let originals = db::list_tracks(&state).await.unwrap();
        let playback_id = &originals[0].id;
        state.db.lock().unwrap().execute(
            "INSERT INTO playerstate (id, current_track_id, progress_seconds) VALUES (1, ?1, 0.05)",
            [playback_id],
        ).unwrap();
        let categories = ["ost", "cover", "mashup"];
        for (track, category) in originals.iter().zip(categories) {
            assert!(db::set_download_tags(
                &state,
                &track.id,
                Path::new(&track.file_path),
                &[category.into()]
            )
            .await
            .unwrap());
            fs::write(
                state.covers_dir.join(format!("{}_small.webp", track.id)),
                &track.id,
            )
            .unwrap();
        }
        let mut events = state.events.subscribe();

        let parked = state.data_dir.join("parked.wav");
        fs::rename(&paths[0], &parked).unwrap();
        for pair in paths.windows(2) {
            fs::rename(&pair[1], &pair[0]).unwrap();
        }
        fs::rename(&parked, &paths[count - 1]).unwrap();
        let hashes: Vec<_> = paths
            .iter()
            .map(|path| file_content_hash(path).unwrap())
            .collect();

        scan_music_dir(state.clone()).await.unwrap();
        scan_music_dir(state.clone()).await.unwrap();

        let after = db::list_tracks(&state).await.unwrap();
        assert_eq!(after.len(), count);
        for ((before, track), category) in originals.iter().zip(after).zip(categories) {
            assert_eq!(track.id, before.id);
            assert_eq!(track.title, before.title);
            assert_eq!(track.added_at, before.added_at);
            let index = hashes.iter().position(|id| id == &track.id).unwrap();
            assert_eq!(track.file_path, paths[index].to_string_lossy());
            assert_eq!(file_content_hash(&paths[index]).unwrap(), track.id);
            assert_eq!(track.tags[0].name, category);
            assert!(db::track_tags_finalized(&state, &track.id).await.unwrap());
            assert_eq!(
                fs::read_to_string(state.covers_dir.join(format!("{}_small.webp", track.id)))
                    .unwrap(),
                track.id
            );
        }
        let playback = state
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT current_track_id, progress_seconds FROM playerstate WHERE id = 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)),
            )
            .unwrap();
        assert_eq!(playback, (playback_id.clone(), 0.05));
        let mut updated = std::collections::HashSet::new();
        for _ in 0..count {
            let event = events.try_recv().unwrap();
            assert_eq!(event.action_key.as_deref(), Some("track_updated"));
            let payload = event.action_payload.unwrap();
            assert_eq!(payload["id"], payload["previous_id"]);
            let index = hashes.iter().position(|id| payload["id"] == *id).unwrap();
            assert!(updated.insert(index));
            assert_eq!(
                payload["filename"].as_str(),
                paths[index].file_name().and_then(|name| name.to_str())
            );
        }
        assert!(events.try_recv().is_err());
    }
}

#[tokio::test]
async fn initial_scan_announces_identical_copies_only_once() {
    let (_tmp, state) = make_state();
    let first = state.music_dir.join("first copy.mp3");
    let second = state.music_dir.join("second copy.mp3");
    create_mp3(&first, "Happy Nation", "Ace of Base");
    fs::copy(&first, &second).unwrap();
    let mut events = state.events.subscribe();
    scan_music_dir(state.clone()).await.unwrap();
    assert_eq!(db::list_tracks(&state).await.unwrap().len(), 1);
    let mut added = 0;
    while let Ok(event) = events.try_recv() {
        if event.action_key.as_deref() == Some("track_added") {
            added += 1;
        }
    }
    assert_eq!(added, 1);
}

#[tokio::test]
async fn remaining_copy_keeps_the_library_entry_when_registered_copy_disappears() {
    let (_tmp, state) = make_state();
    let original = state.music_dir.join("original.wav");
    write_silent_wav(&original);
    scan_music_dir(state.clone()).await.unwrap();
    let registered = db::list_tracks(&state).await.unwrap().remove(0);
    let duplicate = state.music_dir.join("another copy.wav");
    fs::copy(&original, &duplicate).unwrap();
    fs::remove_file(&original).unwrap();
    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].id, registered.id);
    assert_eq!(tracks[0].added_at, registered.added_at);
    assert_eq!(tracks[0].file_path, duplicate.to_string_lossy());
    assert!(duplicate.is_file());
}

#[tokio::test]
async fn scan_music_dir_removes_existing_unparseable_audio_files() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("broken.mp3");
    fs::write(&path, b"not real audio").unwrap();
    let id = "1111111111111111111111111111111111111111111111111111111111111111";
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "INSERT INTO track (id,title,artist,duration,file_path,added_at,updated_at,has_cover,processing_status) VALUES (?1,?2,?3,?4,?5,?6,?7,0,'COMPLETE')",
            params![id, "broken", "Artist", 0, path.to_string_lossy(), 100, 100],
        )
        .unwrap();
    }

    scan_music_dir(state.clone()).await.unwrap();

    assert!(db::list_tracks(&state).await.unwrap().is_empty());
}

#[tokio::test]
async fn list_tracks_keeps_discovery_order_when_updated_at_changes() {
    let (_tmp, state) = make_state();
    let first = state.music_dir.join("first.flac");
    let second = state.music_dir.join("second.flac");
    fs::write(&first, b"first").unwrap();
    fs::write(&second, b"second").unwrap();
    let first_id = "1111111111111111111111111111111111111111111111111111111111111111";
    let second_id = "2222222222222222222222222222222222222222222222222222222222222222";
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "INSERT INTO track (id,title,artist,duration,file_path,added_at,updated_at,has_cover,processing_status) VALUES (?1,?2,?3,?4,?5,?6,?7,0,'COMPLETE')",
            params![first_id, "First", "Artist", 1, first.to_string_lossy(), 100, 100],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO track (id,title,artist,duration,file_path,added_at,updated_at,has_cover,processing_status) VALUES (?1,?2,?3,?4,?5,?6,?7,0,'COMPLETE')",
            params![second_id, "Second", "Artist", 1, second.to_string_lossy(), 200, 200],
        )
        .unwrap();
    }

    assert_eq!(
        db::list_tracks(&state)
            .await
            .unwrap()
            .into_iter()
            .map(|track| track.id)
            .collect::<Vec<_>>(),
        vec![second_id, first_id]
    );

    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "UPDATE track SET updated_at = 9999 WHERE id = ?1",
            params![first_id],
        )
        .unwrap();
    }

    assert_eq!(
        db::list_tracks(&state)
            .await
            .unwrap()
            .into_iter()
            .map(|track| track.id)
            .collect::<Vec<_>>(),
        vec![second_id, first_id]
    );
}

#[tokio::test]
async fn scan_music_dir_leaves_unchanged_tracks_uncached() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("Artist - song.mp3");
    create_mp3(&path, "song", "Artist");
    let meta = fs::metadata(&path).unwrap();
    let content_hash = file_content_hash(&path).unwrap();
    let signature = file_signature(&meta);
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "INSERT INTO track (id,title,artist,duration,file_path,added_at,updated_at,has_cover,inode,file_signature,processing_status) VALUES (?1,?2,?3,?4,?5,?6,?7,0,?8,?9,'COMPLETE')",
            params![&content_hash, "song", "Artist", 1, path.to_string_lossy(), 100, 100, inode(&meta), signature],
        )
        .unwrap();
    }

    scan_music_dir(state.clone()).await.unwrap();

    let hls_dir = state.data_dir.join(".cache/hls").join(&content_hash);
    assert!(!hls_dir.exists());
    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].id, content_hash);
    assert_eq!(tracks[0].file_path, path.to_string_lossy());
}

#[tokio::test]
async fn scan_music_dir_broadcasts_added_tracks() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.wav");
    write_silent_wav(&path);
    let mut events = state.events.subscribe();

    scan_music_dir(state.clone()).await.unwrap();

    let event = timeout(Duration::from_secs(1), events.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(event.action_key.as_deref(), Some("track_added"));
    assert_eq!(event.message_to_show.as_deref(), Some("song.wav"));
    assert_eq!(event.message_level.as_deref(), Some("info"));
    assert_eq!(event.action_payload.unwrap()["title"], "song");
}

#[tokio::test]
async fn scan_music_dir_preserves_comma_artists_without_structured_boundaries() {
    let (_tmp, state) = make_state();
    let path = state
        .music_dir
        .join("aikko,katanacss,INSPACE,playingtheangel - song.mp3");
    create_mp3(&path, "song", "aikko,katanacss,INSPACE,playingtheangel");

    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].artist, "aikko,katanacss,INSPACE,playingtheangel");
    assert_eq!(tracks[0].file_path, path.to_string_lossy());
    assert!(path.exists());
}

#[tokio::test]
async fn scan_music_dir_renames_unstructured_filename_from_metadata() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("random-name.mp3");
    create_mp3(&path, "song", "Artist");

    scan_music_dir(state.clone()).await.unwrap();

    let renamed_path = state.music_dir.join("Artist - song.mp3");
    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title, "song");
    assert_eq!(tracks[0].artist, "Artist");
    assert_eq!(tracks[0].file_path, renamed_path.to_string_lossy());
    let hls_dir = state.data_dir.join(".cache/hls").join(&tracks[0].id);
    assert!(!hls_dir.exists());
    assert!(!path.exists());
    assert!(renamed_path.exists());
}

#[tokio::test]
async fn scan_music_dir_normalizes_unicode_filenames_to_nfc() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("random-name.mp3");
    create_mp3(&path, "Бедныи\u{0306} Русскии\u{0306}", "Слава КПСС");

    scan_music_dir(state.clone()).await.unwrap();

    let renamed_path = state.music_dir.join("Слава КПСС - Бедный Русский.mp3");
    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].title, "Бедный Русский");
    assert_eq!(tracks[0].file_path, renamed_path.to_string_lossy());
    assert!(!path.exists());
    assert!(renamed_path.exists());
}

#[tokio::test]
async fn scan_music_dir_normalizes_unchanged_existing_tracks() {
    let (_tmp, state) = make_state();
    let path = state
        .music_dir
        .join("aikko, katanacss, INSPACE, playingtheange - song.mp3");
    create_mp3(&path, "song", "aikko; katanacss; INSPACE; playingtheange");
    scan_music_dir(state.clone()).await.unwrap();

    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "UPDATE track SET artist = ?1 WHERE file_path = ?2",
            params![
                "aikko; katanacss;INSPACE;playingtheange",
                path.to_string_lossy()
            ],
        )
        .unwrap();
    }

    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(
        tracks[0].artist,
        "aikko; katanacss; INSPACE; playingtheange"
    );
}

#[tokio::test]
async fn scan_music_dir_handles_external_create_update_and_delete() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.flac");
    fs::write(&path, b"first").unwrap();
    set_mtime(&path, 2_000);

    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].updated_at, 2_000);

    fs::write(&path, b"second").unwrap();
    set_mtime(&path, 4_000);
    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].updated_at, 4_000);

    fs::remove_file(&path).unwrap();
    scan_music_dir(state.clone()).await.unwrap();
    assert!(db::list_tracks(&state).await.unwrap().is_empty());
}

#[tokio::test]
async fn scan_announces_the_previous_id_when_file_contents_change() {
    use std::io::Write;

    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.wav");
    write_silent_wav(&path);
    scan_music_dir(state.clone()).await.unwrap();
    let previous = db::list_tracks(&state).await.unwrap().remove(0);
    let old_covers = ["small", "original"].map(|size| {
        state
            .covers_dir
            .join(format!("{}_{size}.webp", previous.id))
    });
    for cover in &old_covers {
        fs::write(cover, b"old cover").unwrap();
    }
    let old_hls = state.data_dir.join(".cache/hls").join(&previous.id);
    fs::create_dir_all(&old_hls).unwrap();
    fs::write(old_hls.join("index.m3u8"), b"#EXTM3U").unwrap();
    let mut events = state.events.subscribe();

    fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap()
        .write_all(b"changed")
        .unwrap();
    scan_music_dir(state.clone()).await.unwrap();

    let event = events.recv().await.unwrap();
    assert_eq!(event.action_key.as_deref(), Some("track_updated"));
    let payload = event.action_payload.unwrap();
    let updated = db::list_tracks(&state).await.unwrap().remove(0);
    assert_ne!(updated.id, previous.id);
    assert_eq!(payload["previous_id"], previous.id);
    assert_eq!(payload["id"], updated.id);
    assert!(old_covers.iter().all(|cover| !cover.exists()));
    assert!(!old_hls.exists());
}

#[tokio::test]
async fn scan_announces_the_previous_id_when_normalizing_metadata() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.wav");
    write_silent_wav(&path);
    scan_music_dir(state.clone()).await.unwrap();
    let previous = db::list_tracks(&state).await.unwrap().remove(0);
    state
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE track SET title = ?1 WHERE id = ?2",
            params!["Cafe\u{301}", previous.id],
        )
        .unwrap();
    let mut events = state.events.subscribe();

    scan_music_dir(state.clone()).await.unwrap();

    let event = events.recv().await.unwrap();
    assert_eq!(event.action_key.as_deref(), Some("track_updated"));
    let payload = event.action_payload.unwrap();
    let updated = db::list_tracks(&state).await.unwrap().remove(0);
    assert_ne!(updated.id, previous.id);
    assert_eq!(payload["previous_id"], previous.id);
    assert_eq!(payload["id"], updated.id);
    assert_eq!(updated.title, "Café");
}

#[cfg(unix)]
#[tokio::test]
async fn scan_announces_the_previous_id_when_a_moved_file_also_changes() {
    use std::io::Write;

    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.wav");
    write_silent_wav(&path);
    scan_music_dir(state.clone()).await.unwrap();
    let previous = db::list_tracks(&state).await.unwrap().remove(0);
    let mut events = state.events.subscribe();

    let moved = state.music_dir.join("moved.wav");
    fs::rename(path, &moved).unwrap();
    fs::OpenOptions::new()
        .append(true)
        .open(&moved)
        .unwrap()
        .write_all(b"changed")
        .unwrap();
    scan_music_dir(state.clone()).await.unwrap();

    let event = events.recv().await.unwrap();
    assert_eq!(event.action_key.as_deref(), Some("track_updated"));
    let payload = event.action_payload.unwrap();
    let updated = db::list_tracks(&state).await.unwrap().remove(0);
    assert_ne!(updated.id, previous.id);
    assert_eq!(payload["previous_id"], previous.id);
    assert_eq!(payload["id"], updated.id);
}

#[tokio::test]
async fn scan_music_dir_preserves_track_identity_after_move() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.flac");
    fs::write(&path, b"first").unwrap();
    set_mtime(&path, 2_000);

    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).await.unwrap();
    let id = tracks[0].id.clone();
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "UPDATE track SET title = ?1, artist = ?2 WHERE id = ?3",
            params!["Kept Title", "Kept Artist", &id],
        )
        .unwrap();
    }

    let moved_path = state.music_dir.join("moved.flac");
    fs::rename(&path, &moved_path).unwrap();
    scan_music_dir(state.clone()).await.unwrap();

    let renamed_path = state.music_dir.join("Kept Artist - Kept Title.flac");
    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks.len(), 1);
    assert_eq!(tracks[0].id, id);
    assert_eq!(tracks[0].title, "Kept Title");
    assert_eq!(tracks[0].artist, "Kept Artist");
    assert_eq!(tracks[0].file_path, renamed_path.to_string_lossy());
    assert!(!moved_path.exists());
    assert!(renamed_path.exists());
}

#[tokio::test]
async fn scan_music_dir_skips_unchanged_existing_tracks() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.flac");
    fs::write(&path, b"first").unwrap();
    set_mtime(&path, 2_000);

    scan_music_dir(state.clone()).await.unwrap();
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "UPDATE track SET title = ?1 WHERE file_path = ?2",
            params!["Kept Title", path.to_string_lossy()],
        )
        .unwrap();
    }

    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks[0].title, "Kept Title");
    assert!(tracks[0].file_signature.is_some());

    set_mtime(&path, 3_000);
    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks[0].title, "Kept Title");

    fs::write(&path, b"second").unwrap();
    set_mtime(&path, 4_000);
    scan_music_dir(state.clone()).await.unwrap();
    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks[0].title, "song");
    assert_eq!(tracks[0].updated_at, 4_000);
}

#[tokio::test]
async fn scan_music_dir_detects_preserved_mtime_content_changes() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.flac");
    fs::write(&path, b"first").unwrap();
    set_mtime(&path, 2_000);

    scan_music_dir(state.clone()).await.unwrap();
    {
        let conn = state.db.lock().unwrap();
        conn.execute(
            "UPDATE track SET title = ?1 WHERE file_path = ?2",
            params!["Kept Title", path.to_string_lossy()],
        )
        .unwrap();
    }

    fs::write(&path, b"second").unwrap();
    set_mtime(&path, 2_000);
    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).await.unwrap();
    assert_eq!(tracks[0].title, "song");
    assert_eq!(tracks[0].updated_at, 2_000);
    assert_eq!(tracks[0].id, file_content_hash(&path).unwrap());
}

#[tokio::test]
async fn scan_music_dir_standardizes_id3_tags() {
    let (_tmp, state) = make_state();
    let path = state.music_dir.join("song.mp3");
    create_mp3(&path, "Tagged Song", "Artist");
    let mut tag = Tag::read_from_path(&path).unwrap();
    tag.set_title("Tagged Song");
    tag.write_to_path(&path, Version::Id3v24).unwrap();
    scan_music_dir(state.clone()).await.unwrap();

    let tracks = db::list_tracks(&state).await.unwrap();
    let tag = Tag::read_from_path(&tracks[0].file_path).unwrap();
    assert_eq!(tag.version(), Version::Id3v23);
}
