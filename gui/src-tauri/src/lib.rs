mod downloader;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, Manager};

use downloader::video::VideoDownloader;

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

#[tauri::command]
async fn open_folder(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
async fn open_file(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &path])
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_video_info,
            download_video,
            get_download_dir,
            open_folder,
            open_file,
            get_download_history,
            add_to_history,
            clear_history,
            delete_history_item
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
