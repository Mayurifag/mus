use std::{path::Path, process::Stdio, time::Duration};

use anyhow::{anyhow, Result};
use reqwest::{redirect::Policy, Url};
use tempfile::TempPath;
use tokio::{fs, io::AsyncWriteExt, process::Command};

use super::USER_AGENT;
use crate::process::run_command_status;

const MAX_ARTWORK_BYTES: u64 = 16 * 1024 * 1024;

fn validate_artwork_url(url: &Url) -> Result<()> {
    // Restrict requests and redirects to the image providers used by artwork search.
    // HTTPS binds these trusted names to their servers, not to caller-owned DNS.
    let trusted_host = url.host_str().is_some_and(|host| {
        [
            "mzstatic.com",
            "ytimg.com",
            "coverartarchive.org",
            "archive.org",
        ]
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
    });
    if url.scheme() != "https"
        || !trusted_host
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(anyhow!(
            "Artwork requires an HTTPS image URL from Apple, YouTube, or Internet Archive without credentials"
        ));
    }
    Ok(())
}

async fn save_response(mut response: reqwest::Response, source: &Path) -> Result<()> {
    if response
        .content_length()
        .is_some_and(|size| size > MAX_ARTWORK_BYTES)
    {
        return Err(anyhow!("Artwork exceeds the 16 MiB limit"));
    }
    let mut file = fs::File::create(source).await?;
    let mut size = 0;
    while let Some(chunk) = response.chunk().await? {
        size += chunk.len() as u64;
        if size > MAX_ARTWORK_BYTES {
            return Err(anyhow!("Artwork exceeds the 16 MiB limit"));
        }
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    Ok(())
}

/// The caller owns the converted image and removes it by dropping the returned path.
pub async fn download_artwork_as_jpeg(url: &str, cache_dir: &Path) -> Result<TempPath> {
    let parsed_url = Url::parse(url)?;
    validate_artwork_url(&parsed_url)?;
    fs::create_dir_all(cache_dir).await?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .https_only(true)
        .redirect(Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                attempt.error("Too many artwork redirects")
            } else if let Err(error) = validate_artwork_url(attempt.url()) {
                attempt.error(error.to_string())
            } else {
                attempt.follow()
            }
        }))
        .build()?;
    let response = client
        .get(parsed_url)
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await?
        .error_for_status()?;
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
    save_response(response, &source).await?;

    let filter = if is_youtube_thumbnail_url(url) {
        "crop='min(iw,ih)':'min(iw,ih)',scale=1200:1200"
    } else {
        "scale='if(gt(a,1),1200,-2)':'if(gt(a,1),-2,1200)',pad=1200:1200:(ow-iw)/2:(oh-ih)/2"
    };
    let mut command = Command::new("ffmpeg");
    command
        .args([
            "-y",
            "-protocol_whitelist",
            "file,pipe",
            "-format_whitelist",
            "image2,jpeg_pipe,png_pipe,webp_pipe,gif,bmp_pipe,tiff_pipe",
            "-i",
        ])
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

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn accepts_provider_images_and_rejects_untrusted_redirect_destinations() {
        for url in [
            "https://is1-ssl.mzstatic.com/cover.jpg",
            "https://i.ytimg.com/cover.jpg",
            "https://coverartarchive.org/release/id/image",
            "https://archive.org/download/cover.jpg",
            "https://ia800001.us.archive.org/cover.jpg",
        ] {
            assert!(
                validate_artwork_url(&Url::parse(url).unwrap()).is_ok(),
                "{url}"
            );
        }
        for url in [
            "https://example.com/cover.jpg",
            "https://ytimg.com.evil.example/cover.jpg",
            "https://evilytimg.com/cover.jpg",
            "https://i.ytimg.com:8443/cover.jpg",
            "http://i.ytimg.com/cover.jpg",
            "https://user:pass@archive.org/cover.jpg",
        ] {
            assert!(
                validate_artwork_url(&Url::parse(url).unwrap()).is_err(),
                "{url}"
            );
        }
    }

    #[tokio::test]
    async fn rejects_untrusted_urls_before_fetching_artwork() {
        let directory = tempfile::TempDir::new().unwrap();
        for url in [
            "http://example.com/cover.jpg",
            "file:///etc/passwd",
            "https://user:pass@example.com/",
            "https://127.0.0.1/",
            "https://[::1]/",
            "https://[::ffff:127.0.0.1]/",
        ] {
            assert!(
                download_artwork_as_jpeg(url, directory.path())
                    .await
                    .is_err(),
                "{url}"
            );
        }
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn bounds_chunked_artwork_without_trusting_content_length() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 2048];
            assert!(socket.read(&mut request).await.unwrap() > 0);
            socket
                .write_all(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n")
                .await
                .unwrap();
            let chunk = vec![b'x'; 1024 * 1024];
            for _ in 0..17 {
                if socket.write_all(b"100000\r\n").await.is_err()
                    || socket.write_all(&chunk).await.is_err()
                    || socket.write_all(b"\r\n").await.is_err()
                {
                    return;
                }
            }
            let _ = socket.write_all(b"0\r\n\r\n").await;
        });
        let response = reqwest::get(url).await.unwrap();
        let directory = tempfile::TempDir::new().unwrap();
        let path = directory.path().join("source");
        let error = save_response(response, &path).await.unwrap_err();
        assert!(error.to_string().contains("16 MiB"), "{error}");
        assert!(std::fs::metadata(path).unwrap().len() <= MAX_ARTWORK_BYTES);
        server.await.unwrap();
    }
}
