# Thai Video Downloader - TODO

## สถานะโปรเจค: Active Development

---

## เสร็จแล้ว (Completed)

### Phase 1: Core Architecture
- [x] สร้างโครงสร้าง Tauri + React GUI
- [x] ตั้งค่า Dark Theme พร้อม Glassmorphism UI
- [x] สร้าง Glowing icons และ buttons

### Phase 2: Python Backend (Deprecated)
- [x] ~~Python script สำหรับ extract video sources~~
- [x] ~~Playwright browser automation~~
- [x] ~~yt-dlp integration~~

### Phase 3: Rust Migration (Current)
- [x] วางแผน Rust-only architecture
- [x] เพิ่ม dependencies: chromiumoxide, m3u8-rs, reqwest, futures
- [x] สร้าง `src/downloader/mod.rs` - Types และ error handling
- [x] สร้าง `src/downloader/browser.rs` - Browser automation (chromiumoxide)
- [x] สร้าง `src/downloader/hls.rs` - HLS/M3U8 downloader
- [x] สร้าง `src/downloader/video.rs` - Main VideoDownloader interface
- [x] อัปเดต `lib.rs` ให้ใช้ Rust modules แทน Python

### Phase 4: UI Features
- [x] Quality Selection dropdown (480p/720p/1080p)
- [x] Video Preview (thumbnail, title, sources)
- [x] Download History (play, folder, delete buttons)
- [x] Tab navigation (Download/History)
- [x] Progress bar with shimmer animation

### Phase 5: Miracle Integration
- [x] เชื่อมต่อ Miracle Second Brain
- [x] สร้าง project context (`.miracle/project.md`)
- [x] สร้าง session snapshot

### Phase 6: UX Enhancements (NEW - 2025-01-18)
- [x] **Clipboard auto-paste** - Auto-detect URL เมื่อเปิดแอปหรือ focus
- [x] **URL validation** - แสดง "Supported site" / "Unknown site"
- [x] **Clear URL button** - ปุ่ม X เพื่อล้าง URL
- [x] **Keyboard shortcuts** - Enter (fetch/download), Escape (clear), Ctrl+O (open folder)
- [x] **Keyboard hints** - แสดง shortcuts ที่ header
- [x] **Download notification** - Windows notification เมื่อ download เสร็จ
- [x] **Download speed display** - แสดง speed (e.g., 2.5 MB/s)
- [x] **ETA display** - แสดง estimated time remaining
- [x] **Bytes counter** - แสดง downloaded / total bytes

---

## กำลังทำ (In Progress)

### Testing & Bug Fixes
- [ ] ทดสอบ download จากเว็บไซต์จริง
- [ ] ทดสอบ HLS streaming download
- [ ] ทดสอบ Direct MP4 download
- [ ] แก้ไข bugs ที่พบระหว่างทดสอบ
- [ ] ทดสอบ Speed & ETA กับ real downloads

---

## รอดำเนินการ (Pending)

### High Priority

#### Performance Improvements
- [ ] Parallel segment downloads สำหรับ HLS
- [ ] Resume interrupted downloads
- [ ] Download queue management
- [ ] Add retry logic for network errors

#### UI Enhancements
- [ ] Drag & drop URL support
- [ ] Paste button (สำหรับผู้ใช้ที่ไม่คุ้น keyboard shortcuts)

### Medium Priority

#### Features
- [ ] Batch download (multiple URLs)
- [ ] Scheduled downloads
- [ ] Custom output filename template
- [ ] Subtitle download support
- [ ] Audio-only extraction
- [ ] Export history to CSV/JSON

#### Technical Improvements
- [ ] Cache extracted video URLs
- [ ] Add proper error types in Rust backend
- [ ] Add unit tests for downloader modules
- [ ] Improve ad URL detection patterns
- [ ] Send `downloaded_bytes` และ `total_bytes` จาก Rust backend

#### UI Polish
- [ ] Loading skeleton for video preview
- [ ] Tooltip for truncated titles
- [ ] Animation for download completion
- [ ] Dark/Light theme toggle

### Low Priority

#### Build & Distribution
- [ ] Bundle ffmpeg with app
- [ ] Windows installer (NSIS/WiX)
- [ ] Auto-update functionality
- [ ] Code signing
- [ ] System tray integration

#### Localization
- [ ] Multi-language support (EN/TH)
- [ ] Settings page (default quality, folder)

---

## Technical Debt

- [ ] Refactor App.tsx into smaller components (currently ~850 lines)
- [ ] Extract state management to custom hooks
- [ ] Add TypeScript strict mode
- [ ] Document API between React and Tauri
- [ ] Remove deprecated Python backend files

---

## โครงสร้างไฟล์หลัก

```
gui/src-tauri/src/
├── lib.rs              # Tauri commands
├── main.rs             # Entry point
└── downloader/
    ├── mod.rs          # Types, errors, utilities
    ├── browser.rs      # chromiumoxide browser automation
    ├── hls.rs          # HLS/M3U8 parser & downloader
    └── video.rs        # Main VideoDownloader interface

gui/src/
├── App.tsx             # Main React component (~850 lines)
├── App.css             # Styles - dark theme, glassmorphism (~1050 lines)
└── main.tsx            # React entry point

.miracle/               # Miracle Second Brain integration
├── project.md          # Project context & architecture
└── sessions/           # Session snapshots
```

---

## New Features Added (2025-01-18)

### Clipboard Auto-Paste
- ตรวจจับ URL จาก clipboard เมื่อเปิดแอป
- ตรวจจับเมื่อ focus กลับมาที่แอป
- แสดง "Auto-detected" badge
- Validate supported sites

### Keyboard Shortcuts
| Shortcut | Action |
|----------|--------|
| `Enter` | Fetch info หรือ Start download |
| `Escape` | Clear URL / Close dropdown |
| `Ctrl+O` | Open download folder |
| `Ctrl+V` | Paste + auto-detect |

### Speed & ETA
- คำนวณ speed แบบ real-time ด้วย Exponential Moving Average
- แสดง ETA (estimated time remaining)
- แสดง downloaded bytes / total bytes

### Download Notification
- Windows notification เมื่อ download เสร็จ
- Click notification เพื่อ focus กลับมาที่แอป

---

## Dependencies

### Rust (src-tauri/Cargo.toml)
- `tauri` v2 - Desktop app framework
- `chromiumoxide` v0.7 - Headless Chrome automation
- `m3u8-rs` v6 - M3U8 playlist parser
- `reqwest` v0.12 - HTTP client
- `futures` v0.3 - Async utilities
- `tokio` v1 - Async runtime

### Frontend (package.json)
- React 19
- TypeScript
- Vite 7
- Lucide React (icons)

---

## Notes

- ffmpeg ต้องติดตั้งแยกและอยู่ใน PATH สำหรับการแปลง TS → MP4
- Chrome/Chromium ต้องติดตั้งสำหรับ browser automation
- รองรับเว็บไซต์: บ้านจีน.com, หนังสั้นจีน.online
- Speed & ETA ต้องการ `downloaded_bytes` และ `total_bytes` จาก backend

---

## Miracle Commands

```bash
/mrec                    # Load previous context
/msnap                   # Save current context
/mrrr                    # End of session retrospective
/miracle:research [topic]  # Research any topic
/miracle:sync            # Sync to git
```

---

*Last updated: 2025-01-18*
*Connected to Miracle Second Brain*
