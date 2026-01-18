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

/// Sanitize filename to prevent path traversal and other attacks
/// - Removes path separators (/, \)
/// - Removes directory traversal components (..)
/// - Removes null bytes
/// - Replaces other invalid characters with underscores
pub fn sanitize_filename(filename: &str) -> String {
    let mut result = String::with_capacity(filename.len());

    for ch in filename.chars() {
        match ch {
            // Remove dangerous characters completely
            '/' | '\\' | '\0' => continue,
            // Replace other invalid filename characters with underscore
            '<' | '>' | ':' | '"' | '|' | '?' | '*' => result.push('_'),
            // Allow safe characters
            _ => result.push(ch),
        }
    }

    // Remove any remaining .. sequences to prevent traversal
    let cleaned = result.replace("..", "");

    // Remove leading/trailing dots and spaces
    cleaned.trim_matches(|c| c == '.' || c == ' ').to_string()
}

/// Validate and sanitize output directory path
/// Returns error if path contains suspicious patterns
pub fn validate_output_dir(dir: &str) -> Result<String, DownloaderError> {
    // Reject paths with command injection characters
    if dir.contains('\0') || dir.contains('\n') || dir.contains('\r') {
        return Err(DownloaderError::DownloadFailed("Invalid directory path".to_string()));
    }

    #[cfg(target_os = "windows")]
    {
        // Additional Windows-specific checks
        if dir.contains('&') || dir.contains('|') || dir.contains(';') {
            return Err(DownloaderError::DownloadFailed("Invalid directory path".to_string()));
        }
    }

    Ok(dir.to_string())
}

/// Validate URL to prevent SSRF (Server-Side Request Forgery) attacks
/// - Only allows http/https schemes
/// - Blocks private/local network addresses
/// - Blocks file:// and other dangerous schemes
pub fn validate_url(url: &str) -> Result<String, DownloaderError> {
    use url::Url;

    let parsed = Url::parse(url)
        .map_err(|_| DownloaderError::DownloadFailed("Invalid URL format".to_string()))?;

    // Only allow http and https schemes
    match parsed.scheme() {
        "http" | "https" => {
            // OK
        }
        "file" | "data" | "javascript" | "vbscript" => {
            return Err(DownloaderError::DownloadFailed(
                format!("Dangerous URL scheme not allowed: {}", parsed.scheme())
            ));
        }
        _ => {
            return Err(DownloaderError::DownloadFailed(
                format!("Unsupported URL scheme: {}", parsed.scheme())
            ));
        }
    }

    // Block private and local network addresses to prevent SSRF
    if let Some(host) = parsed.host_str() {
        let host_lower = host.to_lowercase();

        // Block localhost variants
        if host_lower == "localhost" || host == "127.0.0.1" || host == "::1" || host == "0.0.0.0" {
            return Err(DownloaderError::DownloadFailed(
                "Localhost addresses are not allowed".to_string()
            ));
        }

        // Block private IP ranges
        if host_lower.starts_with("127.") || host_lower.starts_with("10.") ||
           host_lower.starts_with("192.168.") || host_lower.starts_with("172.16.") ||
           host_lower.starts_with("169.254.") || host_lower.starts_with("fc00:") ||
           host_lower.starts_with("fe80:") || host_lower.starts_with("::1") ||
           host_lower.starts_with("::ffff:") {
            return Err(DownloaderError::DownloadFailed(
                "Private network addresses are not allowed".to_string()
            ));
        }

        // Block local hostname variants
        if host_lower.contains("localhost") || host_lower.contains(".local") {
            return Err(DownloaderError::DownloadFailed(
                "Local hostnames are not allowed".to_string()
            ));
        }
    }

    Ok(url.to_string())
}
