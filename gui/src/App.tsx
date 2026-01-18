import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Download,
  FolderOpen,
  Trash2,
  Loader2,
  CheckCircle,
  XCircle,
  Search,
  Play,
  Clock,
  Film,
  History,
  Settings,
  ChevronDown,
  X,
  Image,
  ClipboardCheck,
  Pause,
  List,
  ChevronUp,
  RotateCcw,
} from "lucide-react";

// Supported site patterns for URL validation
const SUPPORTED_SITES = [
  /xn--12ca1ddhqak6ecxc9b\.com/i,  // บ้านจีน.com (punycode)
  /บ้านจีน\.com/i,
  /xn--12cg1cxchd0a2a4c5c5b\.online/i,  // หนังสั้นจีน.online (punycode)
  /หนังสั้นจีน\.online/i,
];

// Check if URL is from a supported site
const isSupportedUrl = (url: string): boolean => {
  try {
    const urlObj = new URL(url);
    return SUPPORTED_SITES.some(pattern => pattern.test(urlObj.hostname));
  } catch {
    return false;
  }
};

// Check if string is a valid URL
const isValidUrl = (str: string): boolean => {
  try {
    const url = new URL(str);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
};

// Format bytes to human readable
const formatBytes = (bytes: number): string => {
  if (bytes === 0) return "0 B";
  const k = 1024;
  const sizes = ["B", "KB", "MB", "GB"];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${(bytes / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
};

// Format speed (bytes per second) to human readable
const formatSpeed = (bps: number): string => {
  if (bps === 0) return "0 B/s";
  const k = 1024;
  const sizes = ["B/s", "KB/s", "MB/s", "GB/s"];
  const i = Math.floor(Math.log(bps) / Math.log(k));
  return `${(bps / Math.pow(k, i)).toFixed(1)} ${sizes[i]}`;
};

// Format seconds to human readable time
const formatEta = (seconds: number | null): string => {
  if (seconds === null || seconds <= 0 || !isFinite(seconds)) return "--:--";

  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = Math.floor(seconds % 60);

  if (hours > 0) {
    return `${hours}:${minutes.toString().padStart(2, "0")}:${secs.toString().padStart(2, "0")}`;
  }
  return `${minutes}:${secs.toString().padStart(2, "0")}`;
};

import "./App.css";

interface DownloadProgress {
  status: string;
  progress: number;
  message: string;
  filename: string | null;
  downloaded_bytes?: number;
  total_bytes?: number;
  speed_bps?: number;
}

interface VideoInfo {
  url: string;
  title: string;
  thumbnail: string;
  duration: string;
  qualities: string[];
  sources: { url: string; quality: string; type: string }[];
}

interface HistoryItem {
  id: string;
  url: string;
  title: string;
  thumbnail: string;
  filename: string;
  quality: string;
  downloaded_at: string;
  file_path: string;
  file_size: number | null;
}

interface LogEntry {
  type: "info" | "success" | "error" | "progress";
  message: string;
  timestamp: Date;
}

// Queue types
interface QueueItem {
  id: string;
  url: string;
  title: string;
  thumbnail: string;
  quality: string;
  output_dir: string;
  output_filename: string;
  status: "Pending" | "Downloading" | "Paused" | "Completed" | "Failed" | "Cancelled";
  progress: number;
  speed: string;
  eta: string;
  error: string | null;
  file_path: string | null;
  added_at: string;
}

interface QueueProgress {
  id: string;
  status: "Pending" | "Downloading" | "Paused" | "Completed" | "Failed" | "Cancelled";
  progress: number;
  speed: string;
  eta: string;
  message: string;
  file_path: string | null;
}

// Settings types
interface AppSettings {
  default_download_dir: string;
  default_quality: string;
  max_concurrent_downloads: number;
  auto_start_queue: boolean;
  show_notifications: boolean;
  minimize_to_tray: boolean;
  theme: string;
}

type TabType = "download" | "queue" | "history" | "settings";

function App() {
  const [activeTab, setActiveTab] = useState<TabType>("download");
  const [url, setUrl] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [filename, setFilename] = useState("");
  const [quality, setQuality] = useState("auto");
  const [availableQualities, setAvailableQualities] = useState<string[]>(["auto"]);
  const [isDownloading, setIsDownloading] = useState(false);
  const [isFetchingInfo, setIsFetchingInfo] = useState(false);
  const [progress, setProgress] = useState(0);
  const [status, setStatus] = useState<"idle" | "downloading" | "completed" | "error">("idle");
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [videoInfo, setVideoInfo] = useState<VideoInfo | null>(null);
  const [history, setHistory] = useState<HistoryItem[]>([]);

  // Queue state
  const [queue, setQueue] = useState<QueueItem[]>([]);
  const [isProcessingQueue, setIsProcessingQueue] = useState(false);

  // Settings state
  const [settings, setSettings] = useState<AppSettings>({
    default_download_dir: "",
    default_quality: "auto",
    max_concurrent_downloads: 2,
    auto_start_queue: true,
    show_notifications: true,
    minimize_to_tray: false,
    theme: "dark",
  });
  const [showQualityDropdown, setShowQualityDropdown] = useState(false);
  const [clipboardDetected, setClipboardDetected] = useState(false);
  const [, setUrlSource] = useState<"manual" | "clipboard" | null>(null);

  // Speed & ETA tracking
  const [downloadSpeed, setDownloadSpeed] = useState(0); // bytes per second
  const [downloadedBytes, setDownloadedBytes] = useState(0);
  const [totalBytes, setTotalBytes] = useState(0);
  const [eta, setEta] = useState<number | null>(null); // seconds remaining
  const downloadStartTime = useRef<number | null>(null);
  const lastProgressUpdate = useRef<{ time: number; bytes: number } | null>(null);

  const logEndRef = useRef<HTMLDivElement>(null);
  const urlInputRef = useRef<HTMLInputElement>(null);

  // Check clipboard for valid URL
  const checkClipboard = useCallback(async () => {
    // Don't check if already has URL or is busy
    if (url.trim() || isDownloading || isFetchingInfo) return;

    try {
      const clipboardText = await navigator.clipboard.readText();
      const trimmed = clipboardText.trim();

      if (isValidUrl(trimmed)) {
        const isSupported = isSupportedUrl(trimmed);
        setUrl(trimmed);
        setUrlSource("clipboard");
        setClipboardDetected(true);

        if (isSupported) {
          addLog("info", `Detected URL from clipboard (supported site)`);
        } else {
          addLog("info", `Detected URL from clipboard (may not be supported)`);
        }

        // Auto-hide the clipboard indicator after 3 seconds
        setTimeout(() => setClipboardDetected(false), 3000);
      }
    } catch (error) {
      // Clipboard access denied or empty - silently ignore
      console.debug("Clipboard access:", error);
    }
  }, [url, isDownloading, isFetchingInfo]);

  // Check clipboard on app start and window focus
  useEffect(() => {
    // Check on initial load
    checkClipboard();

    // Check when window gains focus
    const handleFocus = () => {
      checkClipboard();
    };

    window.addEventListener("focus", handleFocus);

    return () => {
      window.removeEventListener("focus", handleFocus);
    };
  }, [checkClipboard]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyDown = async (e: KeyboardEvent) => {
      // Ctrl+V or Cmd+V - Paste and auto-fetch
      if ((e.ctrlKey || e.metaKey) && e.key === "v") {
        // Let default paste happen first, then check clipboard
        setTimeout(() => checkClipboard(), 100);
      }

      // Enter - Start download or fetch info
      if (e.key === "Enter" && !e.ctrlKey && !e.metaKey) {
        const activeElement = document.activeElement;
        const isInputFocused = activeElement?.tagName === "INPUT";

        if (isInputFocused && url.trim()) {
          e.preventDefault();
          if (videoInfo && !isDownloading) {
            // If we have video info, start download
            handleDownload();
          } else if (!isFetchingInfo && !videoInfo) {
            // Otherwise fetch info first
            handleFetchInfo();
          }
        }
      }

      // Escape - Clear/Cancel
      if (e.key === "Escape") {
        if (showQualityDropdown) {
          setShowQualityDropdown(false);
        } else if (url.trim() && !isDownloading) {
          setUrl("");
          setVideoInfo(null);
          setUrlSource(null);
          urlInputRef.current?.focus();
        }
      }

      // Ctrl+O - Open download folder
      if ((e.ctrlKey || e.metaKey) && e.key === "o") {
        e.preventDefault();
        if (outputDir) {
          handleOpenFolder();
        }
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [url, videoInfo, isDownloading, isFetchingInfo, showQualityDropdown, outputDir, checkClipboard]);

  useEffect(() => {
    invoke<string>("get_download_dir").then(setOutputDir).catch(console.error);
    loadHistory();
    loadSettings();
    loadQueue();

    // Listen for queue progress updates
    const unlistenQueue = listen<QueueProgress>("queue-progress", (event) => {
      const data = event.payload;
      setQueue(prev => prev.map(item =>
        item.id === data.id
          ? {
              ...item,
              status: data.status,
              progress: data.progress,
              speed: data.speed,
              eta: data.eta,
              file_path: data.file_path || item.file_path,
            }
          : item
      ));

      // Show notification on completion
      if (data.status === "Completed") {
        showNotification("Download Complete", data.message);
        loadHistory();
      }
    });

    const unlisten = listen<DownloadProgress>("download-progress", (event) => {
      const data = event.payload;
      const now = Date.now();

      if (data.status === "downloading") {
        setProgress(data.progress);

        // Calculate speed and ETA
        const currentBytes = data.downloaded_bytes || 0;
        const total = data.total_bytes || 0;

        if (currentBytes > 0) {
          setDownloadedBytes(currentBytes);
          setTotalBytes(total);

          // Calculate speed using time difference
          if (lastProgressUpdate.current) {
            const timeDiff = (now - lastProgressUpdate.current.time) / 1000; // seconds
            const bytesDiff = currentBytes - lastProgressUpdate.current.bytes;

            if (timeDiff > 0.1) { // Update speed at least every 100ms
              const instantSpeed = bytesDiff / timeDiff;
              // Smooth the speed with exponential moving average
              setDownloadSpeed(prev => prev === 0 ? instantSpeed : prev * 0.7 + instantSpeed * 0.3);

              lastProgressUpdate.current = { time: now, bytes: currentBytes };
            }
          } else {
            lastProgressUpdate.current = { time: now, bytes: currentBytes };
          }

          // Calculate ETA
          if (downloadSpeed > 0 && total > 0) {
            const remainingBytes = total - currentBytes;
            setEta(remainingBytes / downloadSpeed);
          }
        } else if (data.speed_bps) {
          // Use speed from backend if available
          setDownloadSpeed(data.speed_bps);
        }

        // Only log occasionally to avoid spam
        if (Math.floor(data.progress) % 10 === 0) {
          addLog("progress", data.message);
        }
      } else if (data.status === "completed") {
        setProgress(100);
        setStatus("completed");
        setIsDownloading(false);
        // Reset speed/ETA
        setDownloadSpeed(0);
        setEta(null);
        setDownloadedBytes(0);
        setTotalBytes(0);
        downloadStartTime.current = null;
        lastProgressUpdate.current = null;

        addLog("success", data.message);
        loadHistory();

        // Show desktop notification
        showNotification(
          "Download Complete",
          data.filename || "Video downloaded successfully"
        );
      } else if (data.status === "info") {
        addLog("info", data.message);
      } else if (data.status === "error") {
        setStatus("error");
        setIsDownloading(false);
        setDownloadSpeed(0);
        setEta(null);
        addLog("error", data.message);
      }
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenQueue.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  // Load settings from backend
  const loadSettings = async () => {
    try {
      const savedSettings = await invoke<AppSettings>("get_settings");
      setSettings(savedSettings);
      if (savedSettings.default_download_dir) {
        setOutputDir(savedSettings.default_download_dir);
      }
    } catch (error) {
      console.error("Failed to load settings:", error);
    }
  };

  // Save settings to backend
  const saveSettings = async (newSettings: AppSettings) => {
    try {
      await invoke("save_settings", { settings: newSettings });
      setSettings(newSettings);
      addLog("success", "Settings saved");
    } catch (error) {
      addLog("error", `Failed to save settings: ${error}`);
    }
  };

  // Load queue from backend
  const loadQueue = async () => {
    try {
      const items = await invoke<QueueItem[]>("queue_get_items");
      setQueue(items);
    } catch (error) {
      console.error("Failed to load queue:", error);
    }
  };

  // Queue management functions
  const addToQueue = async () => {
    if (!videoInfo) {
      addLog("error", "Please fetch video info first");
      return;
    }

    try {
      await invoke<string>("queue_add", {
        url: url.trim(),
        title: videoInfo.title || "Unknown",
        thumbnail: videoInfo.thumbnail || "",
        quality: quality,
        outputDir: outputDir,
        outputFilename: filename || videoInfo.title?.replace(/[<>:"/\\|?*]/g, "_") + ".mp4" || "video.mp4",
      });

      addLog("success", `Added to queue: ${videoInfo.title}`);
      loadQueue();

      // Clear form
      setUrl("");
      setVideoInfo(null);
      setFilename("");

      // Auto-start if enabled
      if (settings.auto_start_queue) {
        processQueue();
      }
    } catch (error) {
      addLog("error", `Failed to add to queue: ${error}`);
    }
  };

  const removeFromQueue = async (id: string) => {
    try {
      await invoke("queue_remove", { id });
      loadQueue();
    } catch (error) {
      addLog("error", `Failed to remove from queue: ${error}`);
    }
  };

  const pauseQueueItem = async (id: string) => {
    try {
      await invoke("queue_pause", { id });
      loadQueue();
    } catch (error) {
      addLog("error", `Failed to pause: ${error}`);
    }
  };

  const resumeQueueItem = async (id: string) => {
    try {
      await invoke("queue_resume", { id });
      loadQueue();
      // Restart processing
      processQueue();
    } catch (error) {
      addLog("error", `Failed to resume: ${error}`);
    }
  };

  const cancelQueueItem = async (id: string) => {
    try {
      await invoke("queue_cancel", { id });
      loadQueue();
    } catch (error) {
      addLog("error", `Failed to cancel: ${error}`);
    }
  };

  const clearCompletedQueue = async () => {
    try {
      await invoke("queue_clear_completed");
      loadQueue();
    } catch (error) {
      addLog("error", `Failed to clear completed: ${error}`);
    }
  };

  const moveQueueItem = async (id: string, direction: number) => {
    try {
      await invoke("queue_move_item", { id, direction });
      loadQueue();
    } catch (error) {
      console.error("Failed to move item:", error);
    }
  };

  const processQueue = async () => {
    if (isProcessingQueue) return;

    setIsProcessingQueue(true);

    try {
      // Find pending items
      const pendingItems = queue.filter(item => item.status === "Pending");
      const activeCount = queue.filter(item => item.status === "Downloading").length;
      const availableSlots = settings.max_concurrent_downloads - activeCount;

      // Start downloads for available slots
      for (let i = 0; i < Math.min(pendingItems.length, availableSlots); i++) {
        const item = pendingItems[i];
        await invoke("queue_start_download", { id: item.id });
      }

      loadQueue();
    } catch (error) {
      addLog("error", `Queue processing error: ${error}`);
    } finally {
      setIsProcessingQueue(false);
    }
  };

  // Auto-process queue when items are added or status changes
  useEffect(() => {
    const pendingCount = queue.filter(item => item.status === "Pending").length;
    const activeCount = queue.filter(item => item.status === "Downloading").length;

    if (pendingCount > 0 && activeCount < settings.max_concurrent_downloads && settings.auto_start_queue) {
      processQueue();
    }
  }, [queue, settings.max_concurrent_downloads, settings.auto_start_queue]);

  const loadHistory = async () => {
    try {
      const items = await invoke<HistoryItem[]>("get_download_history");
      setHistory(items);
    } catch (error) {
      console.error("Failed to load history:", error);
    }
  };

  const addLog = (type: LogEntry["type"], message: string) => {
    setLogs((prev) => [...prev, { type, message, timestamp: new Date() }]);
  };

  // Show desktop notification
  const showNotification = async (title: string, body: string) => {
    try {
      // Request permission if not granted
      if (Notification.permission === "default") {
        await Notification.requestPermission();
      }

      if (Notification.permission === "granted") {
        const notification = new Notification(title, {
          body,
          icon: "/tauri.svg",
          badge: "/tauri.svg",
          tag: "download-complete",
          requireInteraction: false,
        });

        // Auto-close after 5 seconds
        setTimeout(() => notification.close(), 5000);

        // Focus window when clicked
        notification.onclick = () => {
          window.focus();
          notification.close();
        };
      }
    } catch (error) {
      console.debug("Notification error:", error);
    }
  };

  // Request notification permission on mount
  useEffect(() => {
    if ("Notification" in window && Notification.permission === "default") {
      Notification.requestPermission();
    }
  }, []);

  const handleSelectFolder = async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Select Download Folder",
    });
    if (selected) {
      setOutputDir(selected as string);
    }
  };

  const handleFetchInfo = async () => {
    if (!url.trim()) {
      addLog("error", "Please enter a video URL");
      return;
    }

    setIsFetchingInfo(true);
    setVideoInfo(null);
    addLog("info", `Fetching video info: ${url}`);

    try {
      const info = await invoke<VideoInfo>("get_video_info", { url: url.trim() });
      setVideoInfo(info);
      setAvailableQualities(info.qualities.length > 0 ? info.qualities : ["auto"]);
      setQuality(info.qualities[0] || "auto");
      if (info.title) {
        setFilename(info.title.replace(/[<>:"/\\|?*]/g, "_") + ".mp4");
      }
      addLog("success", `Found: ${info.title}`);
    } catch (error) {
      addLog("error", `Error: ${error}`);
    } finally {
      setIsFetchingInfo(false);
    }
  };

  const handleDownload = async () => {
    if (!url.trim()) {
      addLog("error", "Please enter a video URL");
      return;
    }

    setIsDownloading(true);
    setStatus("downloading");
    setProgress(0);

    // Reset speed/ETA tracking
    setDownloadSpeed(0);
    setDownloadedBytes(0);
    setTotalBytes(0);
    setEta(null);
    downloadStartTime.current = Date.now();
    lastProgressUpdate.current = null;

    addLog("info", `Starting download: ${url}`);

    try {
      const result = await invoke<string>("download_video", {
        url: url.trim(),
        outputDir: outputDir,
        outputFilename: filename.trim() || null,
        quality: quality,
      });

      // Add to history
      const historyItem: HistoryItem = {
        id: Date.now().toString(),
        url: url.trim(),
        title: videoInfo?.title || filename || "Unknown",
        thumbnail: videoInfo?.thumbnail || "",
        filename: filename || "video.mp4",
        quality: quality,
        downloaded_at: new Date().toISOString(),
        file_path: `${outputDir}/${filename || "video.mp4"}`,
        file_size: null,
      };

      await invoke("add_to_history", { item: historyItem });
      addLog("success", result);
    } catch (error) {
      setStatus("error");
      addLog("error", `Error: ${error}`);
    } finally {
      setIsDownloading(false);
    }
  };

  const handleOpenFolder = async () => {
    try {
      await invoke("open_folder", { path: outputDir });
    } catch (error) {
      addLog("error", `Failed to open folder: ${error}`);
    }
  };

  const handleOpenFile = async (path: string) => {
    try {
      await invoke("open_file", { path });
    } catch (error) {
      addLog("error", `Failed to open file: ${error}`);
    }
  };

  const handleDeleteHistoryItem = async (id: string) => {
    try {
      await invoke("delete_history_item", { id });
      loadHistory();
    } catch (error) {
      console.error("Failed to delete history item:", error);
    }
  };

  const handleClearHistory = async () => {
    try {
      await invoke("clear_history");
      setHistory([]);
    } catch (error) {
      console.error("Failed to clear history:", error);
    }
  };

  const clearLogs = () => {
    setLogs([]);
  };

  const getStatusIcon = () => {
    switch (status) {
      case "downloading":
        return <Loader2 className="animate-spin" size={20} />;
      case "completed":
        return <CheckCircle size={20} />;
      case "error":
        return <XCircle size={20} />;
      default:
        return null;
    }
  };

  const formatDate = (dateStr: string) => {
    const date = new Date(dateStr);
    return date.toLocaleDateString("th-TH", {
      day: "numeric",
      month: "short",
      year: "numeric",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  return (
    <div className="app">
      <header className="header">
        <h1>Thai Video Downloader</h1>
        <p>Download videos from Thai streaming sites</p>
        <div className="supported-sites">
          <span className="site-badge">บ้านจีน.com</span>
          <span className="site-badge">หนังสั้นจีน.online</span>
        </div>
        <div className="keyboard-hints">
          <span className="hint"><kbd>Enter</kbd> Fetch/Download</span>
          <span className="hint"><kbd>Esc</kbd> Clear</span>
          <span className="hint"><kbd>Ctrl+O</kbd> Open folder</span>
        </div>
      </header>

      {/* Tab Navigation */}
      <div className="tab-nav">
        <button
          className={`tab-btn ${activeTab === "download" ? "active" : ""}`}
          onClick={() => setActiveTab("download")}
        >
          <Download size={18} />
          Download
        </button>
        <button
          className={`tab-btn ${activeTab === "queue" ? "active" : ""}`}
          onClick={() => setActiveTab("queue")}
        >
          <List size={18} />
          Queue
          {queue.filter(i => i.status === "Pending" || i.status === "Downloading").length > 0 && (
            <span className="badge">{queue.filter(i => i.status === "Pending" || i.status === "Downloading").length}</span>
          )}
        </button>
        <button
          className={`tab-btn ${activeTab === "history" ? "active" : ""}`}
          onClick={() => setActiveTab("history")}
        >
          <History size={18} />
          History
          {history.length > 0 && <span className="badge">{history.length}</span>}
        </button>
        <button
          className={`tab-btn ${activeTab === "settings" ? "active" : ""}`}
          onClick={() => setActiveTab("settings")}
        >
          <Settings size={18} />
          Settings
        </button>
      </div>

      <main className="main-content">
        {activeTab === "download" && (
          <>
            {/* URL Input Section */}
            <section className="input-section">
              <div className="input-group">
                <label>
                  Video URL
                  {clipboardDetected && (
                    <span className="clipboard-badge">
                      <ClipboardCheck size={12} />
                      Auto-detected
                    </span>
                  )}
                </label>
                <div className={`input-wrapper url-input ${clipboardDetected ? "clipboard-highlight" : ""}`}>
                  <input
                    ref={urlInputRef}
                    type="text"
                    value={url}
                    onChange={(e) => {
                      setUrl(e.target.value);
                      setUrlSource("manual");
                      setClipboardDetected(false);
                    }}
                    placeholder="Paste video URL here..."
                    disabled={isDownloading || isFetchingInfo}
                  />
                  {url.trim() && (
                    <button
                      onClick={() => {
                        setUrl("");
                        setVideoInfo(null);
                        setUrlSource(null);
                        urlInputRef.current?.focus();
                      }}
                      className="clear-url-btn"
                      title="Clear URL"
                    >
                      <X size={16} />
                    </button>
                  )}
                  <button
                    onClick={handleFetchInfo}
                    disabled={isDownloading || isFetchingInfo || !url.trim()}
                    className="fetch-btn"
                    title="Fetch video info"
                  >
                    {isFetchingInfo ? (
                      <Loader2 className="animate-spin" size={18} />
                    ) : (
                      <Search size={18} />
                    )}
                  </button>
                </div>
                {url.trim() && (
                  <div className="url-status">
                    {isSupportedUrl(url) ? (
                      <span className="url-supported">
                        <CheckCircle size={12} />
                        Supported site
                      </span>
                    ) : (
                      <span className="url-unknown">
                        <XCircle size={12} />
                        Unknown site (may still work)
                      </span>
                    )}
                  </div>
                )}
              </div>

              {/* Video Preview */}
              {videoInfo && (
                <div className="video-preview">
                  <div className="preview-thumbnail">
                    {videoInfo.thumbnail ? (
                      <img src={videoInfo.thumbnail} alt={videoInfo.title} />
                    ) : (
                      <div className="no-thumbnail">
                        <Image size={40} />
                      </div>
                    )}
                    {videoInfo.duration && (
                      <span className="duration">
                        <Clock size={12} />
                        {videoInfo.duration}
                      </span>
                    )}
                  </div>
                  <div className="preview-info">
                    <h3>{videoInfo.title || "Unknown Title"}</h3>
                    <div className="preview-meta">
                      <span className="meta-item">
                        <Film size={14} />
                        {videoInfo.sources.length} source(s)
                      </span>
                      {videoInfo.qualities.length > 0 && (
                        <span className="meta-item quality-available">
                          {videoInfo.qualities.join(", ")}
                        </span>
                      )}
                    </div>
                  </div>
                  <button className="close-preview" onClick={() => setVideoInfo(null)}>
                    <X size={16} />
                  </button>
                </div>
              )}

              {/* Options Row */}
              <div className="options-row">
                <div className="input-group">
                  <label>Save to</label>
                  <div className="input-wrapper">
                    <input
                      type="text"
                      value={outputDir}
                      onChange={(e) => setOutputDir(e.target.value)}
                      placeholder="Download folder"
                      disabled={isDownloading}
                    />
                    <button onClick={handleSelectFolder} disabled={isDownloading}>
                      <FolderOpen size={18} />
                    </button>
                  </div>
                </div>

                <div className="input-group quality-group">
                  <label>Quality</label>
                  <div className="quality-selector" onClick={() => !isDownloading && setShowQualityDropdown(!showQualityDropdown)}>
                    <span>{quality}</span>
                    <ChevronDown size={16} className={showQualityDropdown ? "rotate" : ""} />
                    {showQualityDropdown && (
                      <div className="quality-dropdown">
                        {availableQualities.map((q) => (
                          <button
                            key={q}
                            className={quality === q ? "active" : ""}
                            onClick={(e) => {
                              e.stopPropagation();
                              setQuality(q);
                              setShowQualityDropdown(false);
                            }}
                          >
                            {q}
                          </button>
                        ))}
                      </div>
                    )}
                  </div>
                </div>
              </div>

              <div className="input-group">
                <label>Filename (optional)</label>
                <div className="input-wrapper">
                  <input
                    type="text"
                    value={filename}
                    onChange={(e) => setFilename(e.target.value)}
                    placeholder="video.mp4"
                    disabled={isDownloading}
                  />
                </div>
              </div>

              <button
                className="download-btn"
                onClick={handleDownload}
                disabled={isDownloading || !url.trim()}
              >
                {isDownloading ? (
                  <>
                    <Loader2 className="animate-spin" size={20} />
                    Downloading...
                  </>
                ) : (
                  <>
                    <Download size={20} />
                    Download Video
                  </>
                )}
              </button>

              {/* Add to Queue Button */}
              {videoInfo && !isDownloading && (
                <button
                  className="queue-btn"
                  onClick={addToQueue}
                >
                  <List size={20} />
                  Add to Queue
                </button>
              )}
            </section>

            {/* Progress Section */}
            {(isDownloading || status !== "idle") && (
              <section className="progress-section">
                <div className="progress-header">
                  <h3>Download Progress</h3>
                  <div className="progress-status">
                    {getStatusIcon()}
                    <span className={`status-badge ${status}`}>
                      {status === "downloading"
                        ? "Downloading"
                        : status === "completed"
                        ? "Completed"
                        : status === "error"
                        ? "Error"
                        : ""}
                    </span>
                  </div>
                </div>
                <div className="progress-bar-container">
                  <div className="progress-bar" style={{ width: `${progress}%` }} />
                </div>
                <div className="progress-info">
                  <div className="progress-left">
                    <span className="progress-percent">{progress.toFixed(1)}%</span>
                    {totalBytes > 0 && (
                      <span className="progress-bytes">
                        {formatBytes(downloadedBytes)} / {formatBytes(totalBytes)}
                      </span>
                    )}
                  </div>
                  <div className="progress-right">
                    {isDownloading && downloadSpeed > 0 && (
                      <span className="progress-speed">
                        {formatSpeed(downloadSpeed)}
                      </span>
                    )}
                    {isDownloading && eta !== null && eta > 0 && (
                      <span className="progress-eta">
                        {formatEta(eta)} remaining
                      </span>
                    )}
                    {status === "completed" && (
                      <button className="open-folder-btn" onClick={handleOpenFolder}>
                        <FolderOpen size={14} />
                        Open Folder
                      </button>
                    )}
                  </div>
                </div>
              </section>
            )}

            {/* Log Section */}
            <section className="log-section">
              <div className="log-header">
                <h3>Log</h3>
                <button className="clear-btn" onClick={clearLogs}>
                  <Trash2 size={14} />
                  Clear
                </button>
              </div>
              <div className="log-content">
                {logs.length === 0 ? (
                  <div className="log-empty">No logs yet. Start a download to see progress.</div>
                ) : (
                  logs.map((log, index) => (
                    <div key={index} className={`log-entry ${log.type}`}>
                      [{log.timestamp.toLocaleTimeString()}] {log.message}
                    </div>
                  ))
                )}
                <div ref={logEndRef} />
              </div>
            </section>
          </>
        )}

        {activeTab === "history" && (
          <section className="history-section">
            <div className="history-header">
              <h3>
                <History size={20} />
                Download History
              </h3>
              {history.length > 0 && (
                <button className="clear-btn" onClick={handleClearHistory}>
                  <Trash2 size={14} />
                  Clear All
                </button>
              )}
            </div>

            {history.length === 0 ? (
              <div className="history-empty">
                <Film size={48} />
                <p>No download history yet</p>
                <span>Your downloaded videos will appear here</span>
              </div>
            ) : (
              <div className="history-list">
                {history.map((item) => (
                  <div key={item.id} className="history-item">
                    <div className="history-thumbnail">
                      {item.thumbnail ? (
                        <img src={item.thumbnail} alt={item.title} />
                      ) : (
                        <div className="no-thumbnail">
                          <Film size={24} />
                        </div>
                      )}
                    </div>
                    <div className="history-info">
                      <h4>{item.title}</h4>
                      <div className="history-meta">
                        <span className="quality-badge">{item.quality}</span>
                        <span className="date">{formatDate(item.downloaded_at)}</span>
                      </div>
                      <p className="filename">{item.filename}</p>
                    </div>
                    <div className="history-actions">
                      <button
                        className="action-btn play"
                        onClick={() => handleOpenFile(item.file_path)}
                        title="Play video"
                      >
                        <Play size={16} />
                      </button>
                      <button
                        className="action-btn folder"
                        onClick={() => handleOpenFolder()}
                        title="Open folder"
                      >
                        <FolderOpen size={16} />
                      </button>
                      <button
                        className="action-btn delete"
                        onClick={() => handleDeleteHistoryItem(item.id)}
                        title="Remove from history"
                      >
                        <Trash2 size={16} />
                      </button>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>
        )}

        {/* Queue Tab */}
        {activeTab === "queue" && (
          <section className="queue-section">
            <div className="queue-header">
              <h3>
                <List size={20} />
                Download Queue
              </h3>
              <div className="queue-actions">
                {queue.some(i => i.status === "Completed" || i.status === "Failed") && (
                  <button className="clear-btn" onClick={clearCompletedQueue}>
                    <Trash2 size={14} />
                    Clear Completed
                  </button>
                )}
                {queue.some(i => i.status === "Pending") && (
                  <button className="start-btn" onClick={processQueue} disabled={isProcessingQueue}>
                    <Play size={14} />
                    Start Queue
                  </button>
                )}
              </div>
            </div>

            {queue.length === 0 ? (
              <div className="queue-empty">
                <List size={48} />
                <p>Queue is empty</p>
                <span>Add videos from the Download tab to start a queue</span>
              </div>
            ) : (
              <div className="queue-list">
                {queue.map((item, index) => (
                  <div key={item.id} className={`queue-item status-${item.status.toLowerCase()}`}>
                    <div className="queue-thumbnail">
                      {item.thumbnail ? (
                        <img src={item.thumbnail} alt={item.title} />
                      ) : (
                        <div className="no-thumbnail">
                          <Film size={24} />
                        </div>
                      )}
                      {item.status === "Downloading" && (
                        <div className="thumbnail-overlay">
                          <Loader2 className="animate-spin" size={20} />
                        </div>
                      )}
                    </div>
                    <div className="queue-info">
                      <h4>{item.title}</h4>
                      <div className="queue-meta">
                        <span className={`status-badge ${item.status.toLowerCase()}`}>
                          {item.status === "Downloading" && <Loader2 className="animate-spin" size={12} />}
                          {item.status === "Completed" && <CheckCircle size={12} />}
                          {item.status === "Failed" && <XCircle size={12} />}
                          {item.status === "Paused" && <Pause size={12} />}
                          {item.status}
                        </span>
                        <span className="quality-badge">{item.quality}</span>
                        {item.speed && <span className="speed">{item.speed}</span>}
                        {item.eta && <span className="eta">{item.eta}</span>}
                      </div>
                      {(item.status === "Downloading" || item.status === "Paused") && (
                        <div className="queue-progress">
                          <div className="progress-bar-container">
                            <div className="progress-bar" style={{ width: `${item.progress}%` }} />
                          </div>
                          <span className="progress-text">{item.progress.toFixed(1)}%</span>
                        </div>
                      )}
                      {item.error && (
                        <p className="error-text">{item.error}</p>
                      )}
                    </div>
                    <div className="queue-item-actions">
                      {/* Move buttons */}
                      {item.status === "Pending" && (
                        <>
                          <button
                            className="action-btn"
                            onClick={() => moveQueueItem(item.id, -1)}
                            disabled={index === 0}
                            title="Move up"
                          >
                            <ChevronUp size={16} />
                          </button>
                          <button
                            className="action-btn"
                            onClick={() => moveQueueItem(item.id, 1)}
                            disabled={index === queue.length - 1}
                            title="Move down"
                          >
                            <ChevronDown size={16} />
                          </button>
                        </>
                      )}
                      {/* Pause/Resume */}
                      {item.status === "Downloading" && (
                        <button
                          className="action-btn pause"
                          onClick={() => pauseQueueItem(item.id)}
                          title="Pause"
                        >
                          <Pause size={16} />
                        </button>
                      )}
                      {item.status === "Paused" && (
                        <button
                          className="action-btn play"
                          onClick={() => resumeQueueItem(item.id)}
                          title="Resume"
                        >
                          <Play size={16} />
                        </button>
                      )}
                      {/* Retry */}
                      {item.status === "Failed" && (
                        <button
                          className="action-btn retry"
                          onClick={() => resumeQueueItem(item.id)}
                          title="Retry"
                        >
                          <RotateCcw size={16} />
                        </button>
                      )}
                      {/* Open file */}
                      {item.status === "Completed" && item.file_path && (
                        <button
                          className="action-btn play"
                          onClick={() => handleOpenFile(item.file_path!)}
                          title="Play"
                        >
                          <Play size={16} />
                        </button>
                      )}
                      {/* Cancel/Remove */}
                      {(item.status === "Pending" || item.status === "Paused") && (
                        <button
                          className="action-btn delete"
                          onClick={() => cancelQueueItem(item.id)}
                          title="Cancel"
                        >
                          <X size={16} />
                        </button>
                      )}
                      {(item.status === "Completed" || item.status === "Failed" || item.status === "Cancelled") && (
                        <button
                          className="action-btn delete"
                          onClick={() => removeFromQueue(item.id)}
                          title="Remove"
                        >
                          <Trash2 size={16} />
                        </button>
                      )}
                    </div>
                  </div>
                ))}
              </div>
            )}
          </section>
        )}

        {/* Settings Tab */}
        {activeTab === "settings" && (
          <section className="settings-section">
            <div className="settings-header">
              <h3>
                <Settings size={20} />
                Settings
              </h3>
            </div>

            <div className="settings-content">
              <div className="settings-group">
                <h4>Download Settings</h4>

                <div className="setting-item">
                  <label>Default Download Folder</label>
                  <div className="input-wrapper">
                    <input
                      type="text"
                      value={settings.default_download_dir}
                      onChange={(e) => setSettings({ ...settings, default_download_dir: e.target.value })}
                      placeholder="Select folder..."
                    />
                    <button onClick={async () => {
                      const selected = await open({ directory: true, multiple: false });
                      if (selected) {
                        setSettings({ ...settings, default_download_dir: selected as string });
                      }
                    }}>
                      <FolderOpen size={18} />
                    </button>
                  </div>
                </div>

                <div className="setting-item">
                  <label>Default Quality</label>
                  <select
                    value={settings.default_quality}
                    onChange={(e) => setSettings({ ...settings, default_quality: e.target.value })}
                  >
                    <option value="auto">Auto (Best)</option>
                    <option value="1080p">1080p</option>
                    <option value="720p">720p</option>
                    <option value="480p">480p</option>
                    <option value="360p">360p</option>
                  </select>
                </div>

                <div className="setting-item">
                  <label>Max Concurrent Downloads</label>
                  <select
                    value={settings.max_concurrent_downloads}
                    onChange={(e) => setSettings({ ...settings, max_concurrent_downloads: parseInt(e.target.value) })}
                  >
                    <option value={1}>1</option>
                    <option value={2}>2</option>
                    <option value={3}>3</option>
                    <option value={4}>4</option>
                    <option value={5}>5</option>
                  </select>
                </div>
              </div>

              <div className="settings-group">
                <h4>Queue Settings</h4>

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.auto_start_queue}
                      onChange={(e) => setSettings({ ...settings, auto_start_queue: e.target.checked })}
                    />
                    Auto-start queue when items are added
                  </label>
                </div>
              </div>

              <div className="settings-group">
                <h4>Notifications</h4>

                <div className="setting-item checkbox">
                  <label>
                    <input
                      type="checkbox"
                      checked={settings.show_notifications}
                      onChange={(e) => setSettings({ ...settings, show_notifications: e.target.checked })}
                    />
                    Show desktop notifications
                  </label>
                </div>
              </div>

              <div className="settings-actions">
                <button className="save-btn" onClick={() => saveSettings(settings)}>
                  <CheckCircle size={18} />
                  Save Settings
                </button>
              </div>
            </div>
          </section>
        )}
      </main>
    </div>
  );
}

export default App;
