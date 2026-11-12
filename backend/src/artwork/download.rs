use std::{path::Path, process::Stdio, time::Duration};

use anyhow::{anyhow, Result};
use reqwest::Url;
use tempfile::TempPath;
use tokio::{fs, process::Command};

use super::USER_AGENT;
use crate::process::run_command_status;

/// The caller owns the converted image and removes it by dropping the returned path.
pub async fn download_artwork_as_jpeg(url: &str, cache_dir: &Path) -> Result<TempPath> {
    fs::create_dir_all(cache_dir).await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()?;
    let bytes = client
        .get(url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let directory = cache_dir.to_path_buf();
    let (source, jpeg) = tokio::task::spawn_blocking(move || -> Result<_> {
        let source = tempfile::Builder::new()
            .prefix("artwork-")
            .tempfile_in(&directory)?
            .into_temp_path();
        let jpeg = tempfile::Builder::new()
            .prefix("artwork-")
            .suffix(".jpg")
            .tempfile_in(&directory)?
            .into_temp_path();
        Ok((source, jpeg))
    })
    .await??;
    fs::write(&source, bytes).await?;

    let filter = if is_youtube_thumbnail_url(url) {
        "crop='min(iw,ih)':'min(iw,ih)',scale=1200:1200"
    } else {
        "scale='if(gt(a,1),1200,-2)':'if(gt(a,1),-2,1200)',pad=1200:1200:(ow-iw)/2:(oh-ih)/2"
    };
    let mut command = Command::new("ffmpeg");
    command
        .args(["-y", "-i"])
        .arg(&source)
        .args(["-frames:v", "1", "-vf", filter, "-q:v", "2"])
        .arg(&jpeg)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = run_command_status(command, Duration::from_secs(60)).await?;
    if !status.success() || !fs::metadata(&jpeg).await.is_ok_and(|meta| meta.is_file()) {
        return Err(anyhow!("failed to convert artwork to JPEG"));
    }
    Ok(jpeg)
}

fn is_youtube_thumbnail_url(url: &str) -> bool {
    Url::parse(url).ok().is_some_and(|url| {
        url.host_str()
            .is_some_and(|host| host.ends_with(".ytimg.com"))
    })
}
