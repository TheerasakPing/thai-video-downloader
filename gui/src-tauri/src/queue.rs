use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum QueueItemStatus {
    Pending,
    Downloading,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: String,
    pub url: String,
    pub title: String,
    pub thumbnail: String,
    pub quality: String,
    pub output_dir: String,
    pub output_filename: String,
    pub status: QueueItemStatus,
    pub progress: f32,
    pub speed: String,
    pub eta: String,
    pub error: Option<String>,
    pub file_path: Option<String>,
    pub added_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QueueProgress {
    pub id: String,
    pub status: QueueItemStatus,
    pub progress: f32,
    pub speed: String,
    pub eta: String,
    pub message: String,
    pub file_path: Option<String>,
}

pub struct DownloadQueue {
    items: Arc<RwLock<Vec<QueueItem>>>,
    active_downloads: Arc<RwLock<HashMap<String, tokio::sync::oneshot::Sender<()>>>>,
    max_concurrent: Arc<RwLock<usize>>,
}

impl DownloadQueue {
    pub fn new() -> Self {
        Self {
            items: Arc::new(RwLock::new(Vec::new())),
            active_downloads: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent: Arc::new(RwLock::new(2)), // Default 2 concurrent downloads
        }
    }

    pub async fn add_item(
        &self,
        url: String,
        title: String,
        thumbnail: String,
        quality: String,
        output_dir: String,
        output_filename: String,
    ) -> String {
        let id = Uuid::new_v4().to_string();
        let item = QueueItem {
            id: id.clone(),
            url,
            title,
            thumbnail,
            quality,
            output_dir,
            output_filename,
            status: QueueItemStatus::Pending,
            progress: 0.0,
            speed: String::new(),
            eta: String::new(),
            error: None,
            file_path: None,
            added_at: chrono::Utc::now().to_rfc3339(),
        };

        let mut items = self.items.write().await;
        items.push(item);
        id
    }

    pub async fn get_items(&self) -> Vec<QueueItem> {
        let items = self.items.read().await;
        items.clone()
    }

    pub async fn get_item(&self, id: &str) -> Option<QueueItem> {
        let items = self.items.read().await;
        items.iter().find(|i| i.id == id).cloned()
    }

    pub async fn update_item_status(&self, id: &str, status: QueueItemStatus) {
        let mut items = self.items.write().await;
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.status = status;
        }
    }

    pub async fn update_item_progress(
        &self,
        id: &str,
        progress: f32,
        speed: String,
        eta: String,
    ) {
        let mut items = self.items.write().await;
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.progress = progress;
            item.speed = speed;
            item.eta = eta;
        }
    }

    pub async fn update_item_error(&self, id: &str, error: String) {
        let mut items = self.items.write().await;
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.status = QueueItemStatus::Failed;
            item.error = Some(error);
        }
    }

    pub async fn update_item_completed(&self, id: &str, file_path: String) {
        let mut items = self.items.write().await;
        if let Some(item) = items.iter_mut().find(|i| i.id == id) {
            item.status = QueueItemStatus::Completed;
            item.progress = 100.0;
            item.file_path = Some(file_path);
        }
    }

    pub async fn remove_item(&self, id: &str) {
        // Cancel if downloading
        self.cancel_download(id).await;

        let mut items = self.items.write().await;
        items.retain(|i| i.id != id);
    }

    pub async fn clear_completed(&self) {
        let mut items = self.items.write().await;
        items.retain(|i| {
            i.status != QueueItemStatus::Completed && i.status != QueueItemStatus::Failed
        });
    }

    pub async fn clear_all(&self) {
        // Cancel all active downloads
        let active = self.active_downloads.read().await;
        let ids: Vec<String> = active.keys().cloned().collect();
        drop(active);

        for id in ids {
            self.cancel_download(&id).await;
        }

        let mut items = self.items.write().await;
        items.clear();
    }

    pub async fn pause_download(&self, id: &str) -> bool {
        let mut active = self.active_downloads.write().await;
        if let Some(cancel_tx) = active.remove(id) {
            let _ = cancel_tx.send(());
            drop(active);
            self.update_item_status(id, QueueItemStatus::Paused).await;
            true
        } else {
            false
        }
    }

    pub async fn resume_download(&self, id: &str) -> bool {
        let items = self.items.read().await;
        if let Some(item) = items.iter().find(|i| i.id == id) {
            if item.status == QueueItemStatus::Paused {
                drop(items);
                self.update_item_status(id, QueueItemStatus::Pending).await;
                return true;
            }
        }
        false
    }

    pub async fn cancel_download(&self, id: &str) -> bool {
        let mut active = self.active_downloads.write().await;
        if let Some(cancel_tx) = active.remove(id) {
            let _ = cancel_tx.send(());
            drop(active);
            self.update_item_status(id, QueueItemStatus::Cancelled).await;
            true
        } else {
            // Just mark as cancelled if not active
            self.update_item_status(id, QueueItemStatus::Cancelled).await;
            true
        }
    }

    pub async fn set_max_concurrent(&self, max: usize) {
        let mut max_concurrent = self.max_concurrent.write().await;
        *max_concurrent = max.max(1).min(5); // Between 1 and 5
    }

    pub async fn get_max_concurrent(&self) -> usize {
        *self.max_concurrent.read().await
    }

    pub async fn register_active_download(&self, id: &str) -> tokio::sync::oneshot::Receiver<()> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut active = self.active_downloads.write().await;
        active.insert(id.to_string(), tx);
        rx
    }

    pub async fn unregister_active_download(&self, id: &str) {
        let mut active = self.active_downloads.write().await;
        active.remove(id);
    }

    pub async fn get_active_count(&self) -> usize {
        self.active_downloads.read().await.len()
    }

    pub async fn get_pending_items(&self) -> Vec<QueueItem> {
        let items = self.items.read().await;
        items
            .iter()
            .filter(|i| i.status == QueueItemStatus::Pending)
            .cloned()
            .collect()
    }

    pub async fn move_item(&self, id: &str, direction: i32) {
        let mut items = self.items.write().await;
        if let Some(pos) = items.iter().position(|i| i.id == id) {
            let new_pos = if direction > 0 {
                (pos + 1).min(items.len() - 1)
            } else {
                pos.saturating_sub(1)
            };
            if pos != new_pos {
                let item = items.remove(pos);
                items.insert(new_pos, item);
            }
        }
    }
}

impl Default for DownloadQueue {
    fn default() -> Self {
        Self::new()
    }
}
