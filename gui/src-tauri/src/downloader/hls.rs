use futures::StreamExt;
use m3u8_rs::{MediaPlaylist, MasterPlaylist, Playlist};
use reqwest::Client;
use std::path::{Path, PathBuf};
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use url::Url;

use super::DownloaderError;

pub struct HlsDownloader {
    client: Client,
    referer: Option<String>,
}

impl HlsDownloader {
    pub fn new(referer: Option<String>) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap();

        Self { client, referer }
    }

    pub async fn download(
        &self,
        m3u8_url: &str,
        output_path: &Path,
        progress_callback: impl Fn(f32, String) + Send + 'static,
    ) -> Result<PathBuf, DownloaderError> {
        let base_url = Url::parse(m3u8_url)
            .map_err(|e| DownloaderError::Parse(e.to_string()))?;

        // Fetch the m3u8 playlist
        let mut request = self.client.get(m3u8_url);
        if let Some(ref referer) = self.referer {
            request = request.header("Referer", referer);
        }

        let response = request.send().await?;
        let content = response.text().await?;

        // Parse the playlist
        let playlist = m3u8_rs::parse_playlist_res(content.as_bytes())
            .map_err(|e| DownloaderError::Parse(format!("Failed to parse m3u8: {:?}", e)))?;

        match playlist {
            Playlist::MasterPlaylist(master) => {
                // Find the best quality stream
                let stream_url = self.get_best_stream(&master, &base_url)?;
                self.download_media_playlist(&stream_url, output_path, progress_callback).await
            }
            Playlist::MediaPlaylist(media) => {
                self.download_segments(&media, &base_url, output_path, progress_callback).await
            }
        }
    }

    fn get_best_stream(&self, master: &MasterPlaylist, base_url: &Url) -> Result<String, DownloaderError> {
        let best = master
            .variants
            .iter()
            .max_by_key(|v| v.bandwidth)
            .ok_or(DownloaderError::NoSources)?;

        let stream_url = if best.uri.starts_with("http") {
            best.uri.clone()
        } else {
            base_url.join(&best.uri)
                .map_err(|e| DownloaderError::Parse(e.to_string()))?
                .to_string()
        };

        Ok(stream_url)
    }

    async fn download_media_playlist(
        &self,
        url: &str,
        output_path: &Path,
        progress_callback: impl Fn(f32, String) + Send + 'static,
    ) -> Result<PathBuf, DownloaderError> {
        let base_url = Url::parse(url)
            .map_err(|e| DownloaderError::Parse(e.to_string()))?;

        let mut request = self.client.get(url);
        if let Some(ref referer) = self.referer {
            request = request.header("Referer", referer);
        }

        let response = request.send().await?;
        let content = response.text().await?;

        let playlist = m3u8_rs::parse_media_playlist_res(content.as_bytes())
            .map_err(|e| DownloaderError::Parse(format!("Failed to parse media playlist: {:?}", e)))?;

        self.download_segments(&playlist, &base_url, output_path, progress_callback).await
    }

    async fn download_segments(
        &self,
        playlist: &MediaPlaylist,
        base_url: &Url,
        output_path: &Path,
        progress_callback: impl Fn(f32, String) + Send + 'static,
    ) -> Result<PathBuf, DownloaderError> {
        let total_segments = playlist.segments.len();

        // Use a temp file with safe ASCII name for ffmpeg compatibility
        let temp_dir = std::env::temp_dir();
        let temp_id = uuid::Uuid::new_v4().to_string();
        let temp_ts_path = temp_dir.join(format!("video_{}.ts", temp_id));

        let mut output_file = File::create(&temp_ts_path).await?;

        for (i, segment) in playlist.segments.iter().enumerate() {
            let segment_url = if segment.uri.starts_with("http") {
                segment.uri.clone()
            } else {
                base_url.join(&segment.uri)
                    .map_err(|e| DownloaderError::Parse(e.to_string()))?
                    .to_string()
            };

            let progress = ((i + 1) as f32 / total_segments as f32) * 100.0;
            progress_callback(progress, format!("Downloading segment {}/{}", i + 1, total_segments));

            let mut request = self.client.get(&segment_url);
            if let Some(ref referer) = self.referer {
                request = request.header("Referer", referer);
            }

            let response = request.send().await?;
            let bytes = response.bytes().await?;

            output_file.write_all(&bytes).await?;
        }

        output_file.flush().await?;

        // Convert TS to MP4 using ffmpeg with temp files
        let temp_mp4_path = temp_dir.join(format!("video_{}.mp4", temp_id));
        self.convert_to_mp4(&temp_ts_path, &temp_mp4_path).await?;

        // Clean up temp TS file
        tokio::fs::remove_file(&temp_ts_path).await.ok();

        // Move final MP4 to target location with original name
        let mp4_path = output_path.with_extension("mp4");
        if let Err(_) = tokio::fs::rename(&temp_mp4_path, &mp4_path).await {
            // If rename fails (cross-device), copy and delete
            tokio::fs::copy(&temp_mp4_path, &mp4_path).await?;
            tokio::fs::remove_file(&temp_mp4_path).await.ok();
        }

        Ok(mp4_path)
    }

    async fn convert_to_mp4(&self, ts_path: &Path, mp4_path: &Path) -> Result<(), DownloaderError> {
        let output = tokio::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-i", ts_path.to_str().unwrap(),
                "-c", "copy",
                "-bsf:a", "aac_adtstoasc",
                mp4_path.to_str().unwrap(),
            ])
            .output()
            .await
            .map_err(|e| DownloaderError::DownloadFailed(format!("ffmpeg not found: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(DownloaderError::DownloadFailed(format!("ffmpeg failed: {}", stderr)));
        }

        Ok(())
    }
}

pub struct DirectDownloader {
    client: Client,
    referer: Option<String>,
}

impl DirectDownloader {
    pub fn new(referer: Option<String>) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap();

        Self { client, referer }
    }

    pub async fn download(
        &self,
        url: &str,
        output_path: &Path,
        progress_callback: impl Fn(f32, String) + Send + 'static,
    ) -> Result<PathBuf, DownloaderError> {
        let mut request = self.client.get(url);
        if let Some(ref referer) = self.referer {
            request = request.header("Referer", referer);
        }

        let response = request.send().await?;
        let total_size = response.content_length().unwrap_or(0);

        let mp4_path = output_path.with_extension("mp4");
        let mut output_file = File::create(&mp4_path).await?;

        let mut downloaded: u64 = 0;
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            output_file.write_all(&chunk).await?;

            downloaded += chunk.len() as u64;

            if total_size > 0 {
                let progress = (downloaded as f32 / total_size as f32) * 100.0;
                progress_callback(progress, format!("Downloaded {} / {} bytes", downloaded, total_size));
            }
        }

        output_file.flush().await?;

        Ok(mp4_path)
    }
}
