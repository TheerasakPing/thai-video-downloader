use std::path::PathBuf;

use super::{VideoInfo, VideoSource, DownloaderError};
use super::browser::BrowserAutomation;
use super::hls::{HlsDownloader, DirectDownloader};

pub struct VideoDownloader {
    headless: bool,
}

impl VideoDownloader {
    pub fn new(headless: bool) -> Self {
        Self { headless }
    }

    pub async fn get_info(&self, url: &str) -> Result<VideoInfo, DownloaderError> {
        let browser = BrowserAutomation::new(self.headless);
        browser.get_video_info(url).await
    }

    pub async fn download(
        &self,
        url: &str,
        output_dir: &str,
        filename: Option<&str>,
        quality: Option<&str>,
        progress_callback: impl Fn(f32, String) + Send + Clone + 'static,
    ) -> Result<PathBuf, DownloaderError> {
        // Get video info first
        let info = self.get_info(url).await?;

        if info.sources.is_empty() {
            return Err(DownloaderError::NoSources);
        }

        // Select source based on quality
        let source = self.select_source(&info.sources, quality);

        // Determine output path
        let output_filename = filename.unwrap_or("video");
        let output_path = PathBuf::from(output_dir).join(output_filename);

        // Download based on source type
        if source.source_type == "hls" || source.url.contains(".m3u8") {
            let downloader = HlsDownloader::new(Some(url.to_string()));
            downloader.download(&source.url, &output_path, progress_callback).await
        } else {
            let downloader = DirectDownloader::new(Some(url.to_string()));
            downloader.download(&source.url, &output_path, progress_callback).await
        }
    }

    fn select_source<'a>(&self, sources: &'a [VideoSource], quality: Option<&str>) -> &'a VideoSource {
        if let Some(q) = quality {
            if q != "auto" && q != "best" {
                if let Some(source) = sources.iter().find(|s| s.quality == q) {
                    return source;
                }
            }
        }

        // Return first source (usually best quality)
        &sources[0]
    }
}
