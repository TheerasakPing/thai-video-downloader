use std::path::PathBuf;

use super::{VideoInfo, VideoSource, DownloaderError, sanitize_filename, validate_output_dir, validate_url};
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
        // Validate URL to prevent SSRF attacks
        let validated = validate_url(url)?;
        let browser = BrowserAutomation::new(self.headless);
        browser.get_video_info(&validated).await
    }

    pub async fn download(
        &self,
        url: &str,
        output_dir: &str,
        filename: Option<&str>,
        quality: Option<&str>,
        progress_callback: impl Fn(f32, String) + Send + Clone + 'static,
    ) -> Result<PathBuf, DownloaderError> {
        // Validate and sanitize output directory
        let validated_dir = validate_output_dir(output_dir)?;

        // Get video info first
        let info = self.get_info(url).await?;

        if info.sources.is_empty() {
            return Err(DownloaderError::NoSources);
        }

        // Select source based on quality
        let source = self.select_source(&info.sources, quality);

        // Sanitize filename to prevent path traversal
        let sanitized_filename = filename
            .map(sanitize_filename)
            .unwrap_or_else(|| "video".to_string());

        // Ensure the filename is not empty after sanitization
        let output_filename = if sanitized_filename.is_empty() || sanitized_filename == "." {
            "video".to_string()
        } else {
            sanitized_filename
        };

        let output_path = PathBuf::from(&validated_dir).join(&output_filename);

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
