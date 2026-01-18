# Miracle Project Context: chainahone-downloader

## Project Overview

**Name**: Thai Video Downloader (chainahone-downloader)
**Type**: Desktop Application
**Purpose**: Download videos from Thai streaming sites

## Tech Stack

| Layer | Technology |
|-------|------------|
| Frontend | React + TypeScript |
| Desktop | Tauri (Rust) |
| Download Engine | Python + Playwright + yt-dlp |
| Build | Vite |

## Key Files

| File | Purpose |
|------|---------|
| `downloader.py` | Core Python video extractor/downloader |
| `gui/src/App.tsx` | Main React UI component |
| `gui/src-tauri/src/main.rs` | Tauri entry point |
| `gui/src-tauri/src/lib.rs` | Rust backend commands |

## Supported Sites

- บ้านจีน.com
- หนังสั้นจีน.online

## Architecture

```
┌─────────────────────────────────────┐
│           Tauri Window              │
│  ┌───────────────────────────────┐  │
│  │     React Frontend (TSX)      │  │
│  │  - URL Input                  │  │
│  │  - Quality Selection          │  │
│  │  - Progress Display           │  │
│  │  - Download History           │  │
│  └───────────────────────────────┘  │
│              │ invoke()             │
│  ┌───────────────────────────────┐  │
│  │     Rust Backend (Tauri)      │  │
│  │  - Command handlers           │  │
│  │  - File operations            │  │
│  │  - Python process spawn       │  │
│  └───────────────────────────────┘  │
│              │ subprocess           │
│  ┌───────────────────────────────┐  │
│  │     Python Downloader         │  │
│  │  - Playwright (extraction)    │  │
│  │  - yt-dlp (download)          │  │
│  │  - ffmpeg (HLS streams)       │  │
│  └───────────────────────────────┘  │
└─────────────────────────────────────┘
```

## Key Patterns

### Video Extraction Flow
1. User provides URL
2. Playwright launches headless browser
3. Intercept network responses for .m3u8/.mp4
4. Filter out ad URLs
5. Return video sources with quality info

### Download Flow
1. Select quality from available sources
2. Try yt-dlp first (most compatible)
3. Fallback to ffmpeg for HLS streams
4. Report progress via stdout/events

## Miracle Integration

This project is connected to Miracle Second Brain.

### Learnings Applied
- Playwright video extraction pattern
- Tauri + React desktop app pattern
- Ad URL filtering techniques

### Context Files
- `.miracle/project.md` - This file
- `.miracle/sessions/` - Session snapshots

---
*Connected to Miracle: 2025-01-18*
