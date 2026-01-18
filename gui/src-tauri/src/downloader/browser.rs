use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::network::EventResponseReceived;
use futures::StreamExt;
use regex::Regex;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::{extract_quality_from_url, is_ad_url, validate_url, VideoInfo, VideoSource, DownloaderError};

pub struct BrowserAutomation {
    headless: bool,
}

impl BrowserAutomation {
    pub fn new(headless: bool) -> Self {
        Self { headless }
    }

    pub async fn get_video_info(&self, url: &str) -> Result<VideoInfo, DownloaderError> {
        // Validate URL to prevent SSRF attacks
        let validated = validate_url(url)?;

        let mut builder = BrowserConfig::builder();

        if !self.headless {
            builder = builder.with_head();
        }

        let config = builder
            .build()
            .map_err(|e| DownloaderError::Browser(e.to_string()))?;

        let (mut browser, mut handler) = Browser::launch(config)
            .await
            .map_err(|e| DownloaderError::Browser(e.to_string()))?;

        let handler_task = tokio::spawn(async move {
            while let Some(_) = handler.next().await {}
        });

        let result = self.extract_info(&browser, &validated).await;

        browser.close().await.ok();
        handler_task.abort();

        result
    }

    async fn extract_info(&self, browser: &Browser, url: &str) -> Result<VideoInfo, DownloaderError> {
        // Collect video URLs
        let video_urls: Arc<Mutex<Vec<VideoSource>>> = Arc::new(Mutex::new(Vec::new()));

        // Open main page first
        let page = browser
            .new_page(url)
            .await
            .map_err(|e| DownloaderError::Browser(e.to_string()))?;

        // Wait for page to load
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Get page title
        let title = page
            .evaluate("document.title")
            .await
            .ok()
            .and_then(|v| v.into_value::<String>().ok())
            .unwrap_or_default();

        // Get thumbnail
        let thumbnail = page
            .evaluate(r#"
                (function() {
                    var meta = document.querySelector('meta[property="og:image"]');
                    if (meta) return meta.getAttribute('content');
                    var video = document.querySelector('video');
                    if (video && video.poster) return video.poster;
                    return '';
                })()
            "#)
            .await
            .ok()
            .and_then(|v| v.into_value::<String>().ok())
            .unwrap_or_default();

        // Get iframes
        let iframes: Vec<String> = page
            .evaluate(r#"
                Array.from(document.querySelectorAll('iframe')).map(f => f.src || f.getAttribute('data-lazy-src') || f.getAttribute('data-src') || '').filter(s => s.length > 0 && s.startsWith('http'))
            "#)
            .await
            .ok()
            .and_then(|v| v.into_value().ok())
            .unwrap_or_default();

        // Process each iframe - open it with network listener
        for iframe_url in iframes {
            if is_ad_url(&iframe_url) {
                continue;
            }

            // Clone for the async task
            let urls_clone = video_urls.clone();

            // Open iframe page
            if let Ok(iframe_page) = browser.new_page(&iframe_url).await {
                // Set up network listener BEFORE waiting
                if let Ok(mut events) = iframe_page.event_listener::<EventResponseReceived>().await {
                    let urls_for_listener = urls_clone.clone();

                    let listener_task = tokio::spawn(async move {
                        while let Some(event) = events.next().await {
                            let resp_url = event.response.url.as_str();
                            let mime: String = event.response.mime_type.clone();

                            // Check for video-related responses
                            let is_video = resp_url.contains(".m3u8")
                                || resp_url.contains(".mp4")
                                || resp_url.contains(".webm")
                                || mime.contains("mpegurl")
                                || mime.contains("video/mp4");

                            if is_video && !is_ad_url(resp_url) {
                                let quality = extract_quality_from_url(resp_url);
                                let source_type = if resp_url.contains(".m3u8") || mime.contains("mpegurl") {
                                    "hls"
                                } else {
                                    "direct"
                                };

                                let mut urls = urls_for_listener.lock().await;
                                if !urls.iter().any(|s| s.url == resp_url) {
                                    urls.push(VideoSource {
                                        url: resp_url.to_string(),
                                        quality,
                                        source_type: source_type.to_string(),
                                    });
                                }
                            }
                        }
                    });

                    // Wait for iframe to load
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                    // Try to click play button
                    let _ = iframe_page.evaluate(r#"
                        (function() {
                            var playSelectors = [
                                '.play-button', '.vjs-big-play-button', '.plyr__control--overlaid',
                                '[class*="play"]', '.jwplayer', '#player', '.jw-icon-display', 'video'
                            ];
                            for (var selector of playSelectors) {
                                var elem = document.querySelector(selector);
                                if (elem) { elem.click(); break; }
                            }
                            // Also try to play video directly
                            document.querySelectorAll('video').forEach(function(v) {
                                try { v.play(); } catch(e) {}
                            });
                        })()
                    "#).await.ok();

                    // Wait for video to start loading
                    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

                    // Also try to get video URL from jwplayer directly
                    if let Some(jwplayer_sources) = iframe_page
                        .evaluate(r#"
                            (function() {
                                var sources = [];
                                if (typeof jwplayer !== 'undefined') {
                                    try {
                                        var player = jwplayer();
                                        if (player && player.getPlaylistItem) {
                                            var item = player.getPlaylistItem();
                                            if (item && item.file) sources.push(item.file);
                                            if (item && item.sources) {
                                                item.sources.forEach(function(s) {
                                                    if (s.file) sources.push(s.file);
                                                });
                                            }
                                        }
                                    } catch(e) {}
                                }
                                // Also check video elements
                                document.querySelectorAll('video').forEach(function(v) {
                                    if (v.currentSrc) sources.push(v.currentSrc);
                                    if (v.src) sources.push(v.src);
                                });
                                return sources;
                            })()
                        "#)
                        .await
                        .ok()
                        .and_then(|v| v.into_value::<Vec<String>>().ok())
                    {
                        let mut urls = urls_clone.lock().await;
                        for src in jwplayer_sources {
                            if !src.is_empty() && !is_ad_url(&src) && !urls.iter().any(|s| s.url == src) {
                                let quality = extract_quality_from_url(&src);
                                let source_type = if src.contains(".m3u8") { "hls" } else { "direct" };
                                urls.push(VideoSource {
                                    url: src,
                                    quality,
                                    source_type: source_type.to_string(),
                                });
                            }
                        }
                    }

                    listener_task.abort();
                }

                iframe_page.close().await.ok();
            }
        }

        // Also check main page for video sources (for sites without iframes)
        let m3u8_regex = Regex::new(r#"(https?://[^\s"'<>\\)]+\.m3u8[^\s"'<>\\)]*)"#).unwrap();
        let mp4_regex = Regex::new(r#"(https?://[^\s"'<>\\)]+\.mp4[^\s"'<>\\)]*)"#).unwrap();

        if let Some(content) = page
            .evaluate("document.documentElement.outerHTML")
            .await
            .ok()
            .and_then(|v| v.into_value::<String>().ok())
        {
            let mut urls = video_urls.lock().await;

            for cap in m3u8_regex.captures_iter(&content) {
                let url = &cap[1];
                if !is_ad_url(url) && !urls.iter().any(|s| s.url == url) {
                    let quality = extract_quality_from_url(url);
                    urls.push(VideoSource {
                        url: url.to_string(),
                        quality,
                        source_type: "hls".to_string(),
                    });
                }
            }

            for cap in mp4_regex.captures_iter(&content) {
                let url = &cap[1];
                if !is_ad_url(url) && !urls.iter().any(|s| s.url == url) {
                    let quality = extract_quality_from_url(url);
                    urls.push(VideoSource {
                        url: url.to_string(),
                        quality,
                        source_type: "direct".to_string(),
                    });
                }
            }
        }

        page.close().await.ok();

        // Deduplicate and filter sources
        let urls = video_urls.lock().await;
        let mut seen = HashSet::new();
        let mut unique_sources: Vec<VideoSource> = Vec::new();
        let mut qualities = HashSet::new();

        for source in urls.iter() {
            // Skip .ts segment files and ads
            if source.url.contains(".ts") && !source.url.contains(".m3u8") {
                continue;
            }
            if is_ad_url(&source.url) {
                continue;
            }
            if seen.insert(source.url.clone()) {
                qualities.insert(source.quality.clone());
                unique_sources.push(source.clone());
            }
        }

        let mut quality_list: Vec<String> = qualities.into_iter().collect();
        quality_list.sort_by(|a, b| {
            let a_num: i32 = a.replace("p", "").replace("auto", "0").parse().unwrap_or(0);
            let b_num: i32 = b.replace("p", "").replace("auto", "0").parse().unwrap_or(0);
            b_num.cmp(&a_num)
        });

        if quality_list.is_empty() {
            quality_list.push("auto".to_string());
        }

        Ok(VideoInfo {
            url: url.to_string(),
            title,
            thumbnail,
            duration: String::new(),
            qualities: quality_list,
            sources: unique_sources,
        })
    }
}
