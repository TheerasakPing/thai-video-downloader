mod downloader;
mod queue;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};
use tokio::sync::RwLock;

use queue::{DownloadQueue, QueueItem, QueueItemStatus, QueueProgress};

use downloader::video::VideoDownloader;

// App Settings
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSettings {
    pub default_download_dir: String,
    pub default_quality: String,
    pub max_concurrent_downloads: usize,
    pub auto_start_queue: bool,
    pub show_notifications: bool,
    pub minimize_to_tray: bool,
    pub theme: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let download_dir = dirs::download_dir()
            .or_else(dirs::home_dir)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        Self {
            default_download_dir: download_dir,
            default_quality: "auto".to_string(),
            max_concurrent_downloads: 2,
            auto_start_queue: true,
            show_notifications: true,
            minimize_to_tray: false,
            theme: "dark".to_string(),
        }
    }
}

// Shared state wrapper
pub struct AppState {
    pub queue: DownloadQueue,
    pub settings: RwLock<AppSettings>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            queue: DownloadQueue::new(),
            settings: RwLock::new(AppSettings::default()),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub status: String,
    pub progress: f32,
    pub message: String,
    pub filename: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VideoInfoResponse {
    pub url: String,
    pub title: String,
    pub thumbnail: String,
    pub duration: String,
    pub qualities: Vec<String>,
    pub sources: Vec<VideoSourceResponse>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct VideoSourceResponse {
    pub url: String,
    pub quality: String,
    #[serde(rename = "type")]
    pub source_type: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    pub url: String,
    pub title: String,
    pub thumbnail: String,
    pub filename: String,
    pub quality: String,
    pub downloaded_at: String,
    pub file_path: String,
    pub file_size: Option<u64>,
}

fn get_history_path(app: &tauri::AppHandle) -> PathBuf {
    let app_dir = app.path().app_data_dir().unwrap_or_default();
    fs::create_dir_all(&app_dir).ok();
    app_dir.join("download_history.json")
}

#[tauri::command]
async fn get_video_info(app: tauri::AppHandle, url: String) -> Result<VideoInfoResponse, String> {
    let _ = app.emit("download-progress", DownloadProgress {
        status: "info".to_string(),
        progress: 0.0,
        message: "กำลังดึงข้อมูลวิดีโอ...".to_string(),
        filename: None,
    });

    let downloader = VideoDownloader::new(true); // headless mode

    let info = downloader
        .get_info(&url)
        .await
        .map_err(|e| format!("Failed to get video info: {}", e))?;

    let sources: Vec<VideoSourceResponse> = info.sources
        .iter()
        .map(|s| VideoSourceResponse {
            url: s.url.clone(),
            quality: s.quality.clone(),
            source_type: s.source_type.clone(),
        })
        .collect();

    let _ = app.emit("download-progress", DownloadProgress {
        status: "info".to_string(),
        progress: 100.0,
        message: format!("พบ {} แหล่งวิดีโอ", sources.len()),
        filename: None,
    });

    Ok(VideoInfoResponse {
        url: info.url,
        title: info.title,
        thumbnail: info.thumbnail,
        duration: info.duration,
        qualities: info.qualities,
        sources,
    })
}

#[tauri::command]
async fn download_video(
    app: tauri::AppHandle,
    url: String,
    output_dir: String,
    output_filename: Option<String>,
    quality: Option<String>,
) -> Result<String, String> {
    let app_clone = Arc::new(app.clone());

    let _ = app.emit("download-progress", DownloadProgress {
        status: "starting".to_string(),
        progress: 0.0,
        message: "เริ่มต้นดาวน์โหลด...".to_string(),
        filename: output_filename.clone(),
    });

    let downloader = VideoDownloader::new(true);

    let app_for_callback = app_clone.clone();
    let filename_for_callback = output_filename.clone();

    let progress_callback = move |progress: f32, message: String| {
        let _ = app_for_callback.emit("download-progress", DownloadProgress {
            status: "downloading".to_string(),
            progress,
            message,
            filename: filename_for_callback.clone(),
        });
    };

    let result = downloader
        .download(
            &url,
            &output_dir,
            output_filename.as_deref(),
            quality.as_deref(),
            progress_callback,
        )
        .await;

    match result {
        Ok(output_path) => {
            let _ = app.emit("download-progress", DownloadProgress {
                status: "completed".to_string(),
                progress: 100.0,
                message: "ดาวน์โหลดเสร็จสมบูรณ์!".to_string(),
                filename: Some(output_path.to_string_lossy().to_string()),
            });
            Ok(output_path.to_string_lossy().to_string())
        }
        Err(e) => {
            let _ = app.emit("download-progress", DownloadProgress {
                status: "error".to_string(),
                progress: 0.0,
                message: format!("ดาวน์โหลดล้มเหลว: {}", e),
                filename: None,
            });
            Err(format!("Download failed: {}", e))
        }
    }
}

#[tauri::command]
async fn get_download_dir() -> Result<String, String> {
    let downloads = dirs::download_dir()
        .or_else(dirs::home_dir)
        .ok_or("Could not find downloads directory")?;

    Ok(downloads.to_string_lossy().to_string())
}

// Sanitize path to prevent command injection
fn sanitize_path(path: &str) -> Result<String, String> {
    // Reject empty paths
    if path.trim().is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    // Reject paths with shell metacharacters that could be used for injection
    let dangerous_chars = ['\0', '\n', '\r', '\t'];
    for char in dangerous_chars {
        if path.contains(char) {
            return Err(format!("Path contains invalid character: {}", char.escape_default()));
        }
    }

    // On Windows, reject paths with command separators that could be abused
    #[cfg(target_os = "windows")]
    {
        if path.contains('&') || path.contains('|') || path.contains(';') {
            return Err("Path contains invalid characters".to_string());
        }
    }

    Ok(path.to_string())
}

// Validate path exists and is within allowed bounds
fn validate_path(path: &str, must_exist: bool, require_directory: bool) -> Result<std::path::PathBuf, String> {
    let path_obj = std::path::Path::new(path);

    if must_exist && !path_obj.exists() {
        return Err("Path does not exist".to_string());
    }

    if require_directory && path_obj.exists() && !path_obj.is_dir() {
        return Err("Path must be a directory".to_string());
    }

    if !require_directory && path_obj.exists() && !path_obj.is_file() {
        return Err("Path must be a file".to_string());
    }

    // Resolve to absolute path to prevent directory traversal issues
    let canonical = if path_obj.exists() {
        path_obj.canonicalize()
            .map_err(|e| format!("Invalid path: {}", e))?
    } else {
        path_obj.to_path_buf()
    };

    Ok(canonical)
}

#[tauri::command]
async fn open_folder(path: String) -> Result<(), String> {
    let sanitized = sanitize_path(&path)?;
    let validated = validate_path(&sanitized, true, true)?;

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&validated)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&validated)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&validated)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn open_file(path: String) -> Result<(), String> {
    let sanitized = sanitize_path(&path)?;
    let validated = validate_path(&sanitized, true, false)?;

    // Use opener crate for safe cross-platform file opening
    opener::open(&validated)
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn get_download_history(app: tauri::AppHandle) -> Result<Vec<HistoryItem>, String> {
    let history_path = get_history_path(&app);

    if !history_path.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&history_path)
        .map_err(|e| format!("Failed to read history: {}", e))?;

    let history: Vec<HistoryItem> = serde_json::from_str(&content)
        .unwrap_or_default();

    Ok(history)
}

#[tauri::command]
async fn add_to_history(app: tauri::AppHandle, item: HistoryItem) -> Result<(), String> {
    let history_path = get_history_path(&app);

    let mut history: Vec<HistoryItem> = if history_path.exists() {
        let content = fs::read_to_string(&history_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        vec![]
    };

    // Add new item at the beginning
    history.insert(0, item);

    // Keep only last 100 items
    history.truncate(100);

    let content = serde_json::to_string_pretty(&history)
        .map_err(|e| format!("Failed to serialize history: {}", e))?;

    fs::write(&history_path, content)
        .map_err(|e| format!("Failed to write history: {}", e))?;

    Ok(())
}

#[tauri::command]
async fn clear_history(app: tauri::AppHandle) -> Result<(), String> {
    let history_path = get_history_path(&app);

    if history_path.exists() {
        fs::remove_file(&history_path)
            .map_err(|e| format!("Failed to clear history: {}", e))?;
    }

    Ok(())
}

#[tauri::command]
async fn delete_history_item(app: tauri::AppHandle, id: String) -> Result<(), String> {
    let history_path = get_history_path(&app);

    if !history_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&history_path)
        .map_err(|e| format!("Failed to read history: {}", e))?;

    let mut history: Vec<HistoryItem> = serde_json::from_str(&content).unwrap_or_default();

    history.retain(|item| item.id != id);

    let content = serde_json::to_string_pretty(&history)
        .map_err(|e| format!("Failed to serialize history: {}", e))?;

    fs::write(&history_path, content)
        .map_err(|e| format!("Failed to write history: {}", e))?;

    Ok(())
}

// ==================== Queue Commands ====================

#[tauri::command]
async fn queue_add(
    state: State<'_, Arc<AppState>>,
    url: String,
    title: String,
    thumbnail: String,
    quality: String,
    output_dir: String,
    output_filename: String,
) -> Result<String, String> {
    let id = state.queue.add_item(url, title, thumbnail, quality, output_dir, output_filename).await;
    Ok(id)
}

#[tauri::command]
async fn queue_get_items(state: State<'_, Arc<AppState>>) -> Result<Vec<QueueItem>, String> {
    Ok(state.queue.get_items().await)
}

#[tauri::command]
async fn queue_remove(state: State<'_, Arc<AppState>>, id: String) -> Result<(), String> {
    state.queue.remove_item(&id).await;
    Ok(())
}

#[tauri::command]
async fn queue_pause(state: State<'_, Arc<AppState>>, id: String) -> Result<bool, String> {
    Ok(state.queue.pause_download(&id).await)
}

#[tauri::command]
async fn queue_resume(state: State<'_, Arc<AppState>>, id: String) -> Result<bool, String> {
    Ok(state.queue.resume_download(&id).await)
}

#[tauri::command]
async fn queue_cancel(state: State<'_, Arc<AppState>>, id: String) -> Result<bool, String> {
    Ok(state.queue.cancel_download(&id).await)
}

#[tauri::command]
async fn queue_clear_completed(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.queue.clear_completed().await;
    Ok(())
}

#[tauri::command]
async fn queue_clear_all(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.queue.clear_all().await;
    Ok(())
}

#[tauri::command]
async fn queue_move_item(state: State<'_, Arc<AppState>>, id: String, direction: i32) -> Result<(), String> {
    state.queue.move_item(&id, direction).await;
    Ok(())
}

#[tauri::command]
async fn queue_start_download(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    id: String,
) -> Result<(), String> {
    let item = state.queue.get_item(&id).await
        .ok_or("Item not found")?;

    if item.status != QueueItemStatus::Pending && item.status != QueueItemStatus::Paused {
        return Err("Item is not in a downloadable state".to_string());
    }

    state.queue.update_item_status(&id, QueueItemStatus::Downloading).await;

    let app_clone = app.clone();
    let state_clone = Arc::clone(&*state);
    let id_clone = id.clone();

    tokio::spawn(async move {
        let cancel_rx = state_clone.queue.register_active_download(&id_clone).await;

        let downloader = VideoDownloader::new(true);

        let app_for_cb = app_clone.clone();
        let state_for_cb = state_clone.clone();
        let id_for_cb = id_clone.clone();

        let progress_callback = move |progress: f32, message: String| {
            let speed = if message.contains("KB/s") || message.contains("MB/s") {
                message.split_whitespace().last().unwrap_or("").to_string()
            } else {
                String::new()
            };

            let progress_data = QueueProgress {
                id: id_for_cb.clone(),
                status: QueueItemStatus::Downloading,
                progress,
                speed: speed.clone(),
                eta: String::new(),
                message: message.clone(),
                file_path: None,
            };

            let _ = app_for_cb.emit("queue-progress", progress_data);

            // Update queue item
            let state_clone = state_for_cb.clone();
            let id_clone = id_for_cb.clone();
            tokio::spawn(async move {
                state_clone.queue.update_item_progress(&id_clone, progress, speed, String::new()).await;
            });
        };

        // Use select to handle cancellation
        tokio::select! {
            result = downloader.download(
                &item.url,
                &item.output_dir,
                Some(&item.output_filename),
                Some(&item.quality),
                progress_callback,
            ) => {
                state_clone.queue.unregister_active_download(&id_clone).await;

                match result {
                    Ok(path) => {
                        let path_str = path.to_string_lossy().to_string();
                        state_clone.queue.update_item_completed(&id_clone, path_str.clone()).await;

                        let _ = app_clone.emit("queue-progress", QueueProgress {
                            id: id_clone,
                            status: QueueItemStatus::Completed,
                            progress: 100.0,
                            speed: String::new(),
                            eta: String::new(),
                            message: "ดาวน์โหลดเสร็จสมบูรณ์".to_string(),
                            file_path: Some(path_str),
                        });
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        state_clone.queue.update_item_error(&id_clone, error_msg.clone()).await;

                        let _ = app_clone.emit("queue-progress", QueueProgress {
                            id: id_clone,
                            status: QueueItemStatus::Failed,
                            progress: 0.0,
                            speed: String::new(),
                            eta: String::new(),
                            message: format!("ดาวน์โหลดล้มเหลว: {}", error_msg),
                            file_path: None,
                        });
                    }
                }
            }
            _ = cancel_rx => {
                state_clone.queue.unregister_active_download(&id_clone).await;
                // Download was cancelled/paused
            }
        }
    });

    Ok(())
}

// ==================== Settings Commands ====================

fn get_settings_path(app: &tauri::AppHandle) -> PathBuf {
    let app_dir = app.path().app_data_dir().unwrap_or_default();
    fs::create_dir_all(&app_dir).ok();
    app_dir.join("settings.json")
}

#[tauri::command]
async fn get_settings(app: tauri::AppHandle, state: State<'_, Arc<AppState>>) -> Result<AppSettings, String> {
    let settings_path = get_settings_path(&app);

    if settings_path.exists() {
        let content = fs::read_to_string(&settings_path).unwrap_or_default();
        if let Ok(settings) = serde_json::from_str::<AppSettings>(&content) {
            let mut state_settings = state.settings.write().await;
            *state_settings = settings.clone();
            return Ok(settings);
        }
    }

    Ok(state.settings.read().await.clone())
}

#[tauri::command]
async fn save_settings(
    app: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
    settings: AppSettings,
) -> Result<(), String> {
    let settings_path = get_settings_path(&app);

    // Update state
    {
        let mut state_settings = state.settings.write().await;
        *state_settings = settings.clone();
    }

    // Update queue max concurrent
    state.queue.set_max_concurrent(settings.max_concurrent_downloads).await;

    // Save to file
    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| format!("Failed to serialize settings: {}", e))?;

    fs::write(&settings_path, content)
        .map_err(|e| format!("Failed to save settings: {}", e))?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(Arc::new(AppState::new()))
        .invoke_handler(tauri::generate_handler![
            get_video_info,
            download_video,
            get_download_dir,
            open_folder,
            open_file,
            get_download_history,
            add_to_history,
            clear_history,
            delete_history_item,
            // Queue commands
            queue_add,
            queue_get_items,
            queue_remove,
            queue_pause,
            queue_resume,
            queue_cancel,
            queue_clear_completed,
            queue_clear_all,
            queue_move_item,
            queue_start_download,
            // Settings commands
            get_settings,
            save_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
