use std::{
    env, fs,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use anyhow::Result;
use mus_backend::{
    app::app,
    db::init_db,
    scanner::{scan_music_dir, watch_music_dir},
    state::AppState,
};
use rusqlite::Connection;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into()))
        .init();

    let state = build_state()?;
    tokio::spawn({
        let state = state.clone();
        async move {
            if let Err(error) = scan_music_dir(state.clone()).await {
                tracing::warn!("failed to scan music directory: {error}");
            }
            watch_music_dir(state).await;
        }
    });

    let mut app = app(state);
    if env::var("APP_ENV").unwrap_or_default() != "production" {
        app = app.layer(CorsLayer::permissive());
    }

    let port = env::var("PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(8001);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

fn build_state() -> Result<AppState> {
    let data_dir = PathBuf::from(env::var("DATA_DIR_PATH").unwrap_or_else(|_| "./app_data".into()));
    let music_dir = data_dir.join("music");
    let covers_dir = data_dir.join("covers");
    let static_dir = env::var("STATIC_DIR_PATH")
        .ok()
        .map(PathBuf::from)
        .filter(|path| path.is_dir());
    fs::create_dir_all(&data_dir)?;
    fs::create_dir_all(&covers_dir)?;

    let mut conn = Connection::open(data_dir.join("mus.db"))?;
    init_db(&mut conn)?;

    let (events, _) = broadcast::channel(100);
    Ok(AppState {
        db: Arc::new(Mutex::new(conn)),
        data_dir,
        static_dir,
        music_dir,
        covers_dir,
        events,
        download_lock: Arc::new(Mutex::new(false)),
        app_date: env::var("BUILD_DATE")
            .ok()
            .and_then(|v| v.get(..10).map(str::to_string))
            .unwrap_or_else(|| "unknown".into()),
        commit_sha: env::var("COMMIT_SHA").ok().filter(|v| !v.is_empty()),
    })
}
