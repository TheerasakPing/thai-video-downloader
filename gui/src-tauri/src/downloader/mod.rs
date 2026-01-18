pub mod browser;
pub mod hls;
pub mod video;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DownloaderError {
    #[error("Browser error: {0}")]
    Browser(String),
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("No video sources found")]
    NoSources,
    #[error("Download failed: {0}")]
    DownloadFailed(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoSource {
    pub url: String,
    pub quality: String,
    pub source_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VideoInfo {
    pub url: String,
    pub title: String,
    pub thumbnail: String,
    pub duration: String,
    pub qualities: Vec<String>,
    pub sources: Vec<VideoSource>,
}

impl Default for VideoInfo {
    fn default() -> Self {
        Self {
            url: String::new(),
            title: String::new(),
            thumbnail: String::new(),
            duration: String::new(),
            qualities: vec!["auto".to_string()],
            sources: Vec::new(),
        }
    }
}

// Ad patterns to filter
pub const AD_PATTERNS: &[&str] = &[
    "adSrc",
    "/ad/",
    "advertisement",
    "b7e06ea0-c18b-4b1e-9cba-2f7a9891f52f",
    "01dd0f98-3b37-40a5-ad47-20935908b632",
    "cf953e68-8e67-4135-9b39-746fe7557c10",
];

pub fn is_ad_url(url: &str) -> bool {
    let url_lower = url.to_lowercase();
    AD_PATTERNS.iter().any(|pattern| url_lower.contains(&pattern.to_lowercase()))
}

pub fn extract_quality_from_url(url: &str) -> String {
    if url.contains("1080") {
        "1080p".to_string()
    } else if url.contains("720") {
        "720p".to_string()
    } else if url.contains("480") {
        "480p".to_string()
    } else if url.contains("360") {
        "360p".to_string()
    } else {
        "auto".to_string()
    }
}
