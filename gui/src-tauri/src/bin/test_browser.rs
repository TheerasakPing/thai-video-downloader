// Test script to debug video extraction - with iframe support
// Run with: cargo run --bin test_browser

use chromiumoxide::browser::{Browser, BrowserConfig};
use chromiumoxide::cdp::browser_protocol::network::EventResponseReceived;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "https://xn--82c7abb4jua0l.com/%e0%b8%96%e0%b8%a5%e0%b8%b3%e0%b8%a3%e0%b8%b1%e0%b8%81%e0%b9%83%e0%b8%99%e0%b9%82%e0%b8%a5%e0%b8%81%e0%b8%82%e0%b8%ad%e0%b8%87%e0%b9%80%e0%b8%98%e0%b8%ad%e0%b8%9e%e0%b8%b2%e0%b8%81%e0%b8%a2%e0%b9%8c/";

    println!("Testing video extraction from: {}", url);
    println!("==========================================\n");

    // Launch browser in non-headless mode
    let config = BrowserConfig::builder()
        .with_head()
        .build()
        .map_err(|e| format!("Config error: {}", e))?;

    println!("Launching browser...");
    let (mut browser, mut handler) = Browser::launch(config).await?;

    let handler_task = tokio::spawn(async move {
        while let Some(_) = handler.next().await {}
    });

    // Open main page first to get iframe URL
    let page = browser.new_page(url).await?;
    println!("Main page opened...");

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Get iframe URL
    let iframes: Vec<String> = page
        .evaluate(r#"
            Array.from(document.querySelectorAll('iframe')).map(f => f.src || f.getAttribute('data-lazy-src') || f.getAttribute('data-src') || '').filter(s => s.length > 0)
        "#)
        .await
        .ok()
        .and_then(|v| v.into_value().ok())
        .unwrap_or_default();

    println!("Found {} iframes:", iframes.len());
    for iframe in &iframes {
        println!("  - {}", iframe);
    }

    if iframes.is_empty() {
        println!("No iframes found!");
        return Ok(());
    }

    // Now open the iframe URL directly with network monitoring
    let iframe_url = &iframes[0];
    println!("\n==========================================");
    println!("Opening iframe URL directly: {}", iframe_url);
    println!("==========================================\n");

    // Collect network responses
    let video_urls: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let urls_clone = video_urls.clone();

    let iframe_page = browser.new_page(iframe_url).await?;
    println!("Iframe page opened, setting up network listener...");

    let mut events = iframe_page.event_listener::<EventResponseReceived>().await?;

    let listener_task = tokio::spawn(async move {
        while let Some(event) = events.next().await {
            let resp_url = event.response.url.as_str();
            let mime: String = event.response.mime_type.clone();

            // Check for video-related URLs
            let is_video = resp_url.contains(".m3u8")
                || resp_url.contains(".mp4")
                || resp_url.contains(".webm")
                || resp_url.contains(".ts")
                || resp_url.contains("/hls/")
                || resp_url.contains("/video/")
                || resp_url.contains("master")
                || resp_url.contains("index")
                || resp_url.contains("playlist")
                || mime.contains("video")
                || mime.contains("mpegurl");

            if is_video {
                println!("[VIDEO RESPONSE] {} (mime: {})", resp_url, mime);
                let mut urls = urls_clone.lock().await;
                if !urls.contains(&resp_url.to_string()) {
                    urls.push(resp_url.to_string());
                }
            }
        }
    });

    // Wait for iframe page to load
    println!("\nWaiting 5 seconds for iframe page load...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Try to click play button in iframe
    println!("Trying to click play button in iframe...");
    let click_result = iframe_page.evaluate(r#"
        (function() {
            var clicked = [];

            // Try various play button selectors
            var playSelectors = [
                '.play-button', '.vjs-big-play-button', '.plyr__control--overlaid',
                '[class*="play"]', 'button[aria-label*="play"]', 'button[aria-label*="Play"]',
                '.video-js', 'video', '.jwplayer', '.player', '#player',
                '.vjs-poster', '.video-container', '.jw-icon-display'
            ];

            for (var selector of playSelectors) {
                var elem = document.querySelector(selector);
                if (elem) {
                    elem.click();
                    clicked.push(selector);
                }
            }

            // Also try to play video directly
            var videos = document.querySelectorAll('video');
            videos.forEach(function(v) {
                try { v.play(); clicked.push('video.play()'); } catch(e) {}
            });

            return clicked;
        })()
    "#).await.ok().and_then(|v| v.into_value::<Vec<String>>().ok());

    println!("Clicked: {:?}", click_result);

    // Wait for video to load after clicking
    println!("\nWaiting 10 seconds for video to load...");
    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

    // Check video elements in iframe
    let video_info = iframe_page
        .evaluate(r#"
            (function() {
                var info = {
                    videoElements: [],
                    jwplayer: null,
                    videojs: null,
                    windowVars: [],
                    allScriptVars: []
                };

                // Check video elements
                document.querySelectorAll('video').forEach(function(v, i) {
                    info.videoElements.push({
                        index: i,
                        src: v.src || '',
                        currentSrc: v.currentSrc || '',
                        sources: Array.from(v.querySelectorAll('source')).map(s => s.src)
                    });
                });

                // Check jwplayer
                if (typeof jwplayer !== 'undefined') {
                    try {
                        var player = jwplayer();
                        if (player) {
                            info.jwplayer = {
                                state: player.getState ? player.getState() : 'unknown',
                                playlist: player.getPlaylistItem ? player.getPlaylistItem() : null,
                                file: player.getPlaylistItem ? (player.getPlaylistItem() || {}).file : null
                            };
                        }
                    } catch(e) {
                        info.jwplayer = { error: e.toString() };
                    }
                }

                // Check videojs
                if (typeof videojs !== 'undefined') {
                    try {
                        var players = document.querySelectorAll('.video-js');
                        info.videojs = [];
                        players.forEach(function(el) {
                            var p = videojs(el);
                            if (p) {
                                info.videojs.push({
                                    currentSrc: p.currentSrc ? p.currentSrc() : null
                                });
                            }
                        });
                    } catch(e) {
                        info.videojs = { error: e.toString() };
                    }
                }

                // Check window variables
                ['playerUrl', 'videoUrl', 'hlsUrl', 'streamUrl', 'source', 'file', 'video_url', 'stream_url', 'src', 'videoSrc'].forEach(function(key) {
                    if (window[key]) {
                        info.windowVars.push({ key: key, value: window[key] });
                    }
                });

                return info;
            })()
        "#)
        .await
        .ok()
        .and_then(|v| v.into_value::<serde_json::Value>().ok());

    println!("\nVideo elements info in iframe:");
    if let Some(info) = video_info {
        println!("{}", serde_json::to_string_pretty(&info).unwrap_or_default());
    }

    // Check collected network URLs
    let urls = video_urls.lock().await;
    println!("\n==========================================");
    println!("Collected {} video URLs from network:", urls.len());
    for url in urls.iter() {
        println!("  - {}", url);
    }

    // Search iframe page source for m3u8/mp4 URLs
    println!("\nSearching iframe page source for video URLs...");
    let content = iframe_page
        .evaluate("document.documentElement.outerHTML")
        .await
        .ok()
        .and_then(|v| v.into_value::<String>().ok())
        .unwrap_or_default();

    let m3u8_regex = regex::Regex::new(r#"(https?://[^\s"'<>\\)]+\.m3u8[^\s"'<>\\)]*)"#).unwrap();
    let mp4_regex = regex::Regex::new(r#"(https?://[^\s"'<>\\)]+\.mp4[^\s"'<>\\)]*)"#).unwrap();

    let mut found_urls = Vec::new();
    for cap in m3u8_regex.captures_iter(&content) {
        found_urls.push(format!("[m3u8] {}", &cap[1]));
    }
    for cap in mp4_regex.captures_iter(&content) {
        found_urls.push(format!("[mp4] {}", &cap[1]));
    }

    println!("Found {} URLs in iframe page source:", found_urls.len());
    for url in &found_urls {
        println!("  {}", url);
    }

    // Also search scripts
    println!("\nSearching script contents...");
    let scripts_content = iframe_page
        .evaluate(r#"
            Array.from(document.querySelectorAll('script')).map(s => s.textContent).join('\n')
        "#)
        .await
        .ok()
        .and_then(|v| v.into_value::<String>().ok())
        .unwrap_or_default();

    let mut script_urls = Vec::new();
    for cap in m3u8_regex.captures_iter(&scripts_content) {
        script_urls.push(format!("[m3u8 in script] {}", &cap[1]));
    }
    for cap in mp4_regex.captures_iter(&scripts_content) {
        script_urls.push(format!("[mp4 in script] {}", &cap[1]));
    }

    println!("Found {} URLs in scripts:", script_urls.len());
    for url in &script_urls {
        println!("  {}", url);
    }

    // Keep browser open for manual inspection
    println!("\n==========================================");
    println!("Browser will stay open for 30 seconds for manual inspection...");
    println!("Check the browser window to see if video is playing.");
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

    // Cleanup
    listener_task.abort();
    page.close().await.ok();
    iframe_page.close().await.ok();
    browser.close().await.ok();
    handler_task.abort();

    println!("\nDone!");
    Ok(())
}
