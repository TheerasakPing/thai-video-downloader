# 🎬 Thai Video Downloader

<div align="center">

![Version](https://img.shields.io/badge/version-1.0.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey)
![Tauri](https://img.shields.io/badge/Tauri-v2-orange)
![React](https://img.shields.io/badge/React-19-61dafb)

**🚀 ดาวน์โหลดวิดีโอจากเว็บไซต์สตรีมมิ่งไทยได้ง่ายๆ ด้วยแอปเดสก์ท็อปสุดล้ำ!**

[📥 ดาวน์โหลด](#-ดาวน์โหลด) • [✨ ฟีเจอร์](#-ฟีเจอร์) • [🛠️ การติดตั้ง](#️-การติดตั้ง) • [📖 วิธีใช้งาน](#-วิธีใช้งาน) • [🤝 Contributing](#-contributing)

</div>

---

## 📸 ภาพตัวอย่าง

```
┌─────────────────────────────────────────────────────────┐
│  🎬 Thai Video Downloader                               │
│  Download videos from Thai streaming sites              │
│  [บ้านจีน.com] [หนังสั้นจีน.online]                      │
│  [Enter] Fetch  [Esc] Clear  [Ctrl+O] Open folder       │
├─────────────────────────────────────────────────────────┤
│  [📥 Download]  [📜 History]                             │
├─────────────────────────────────────────────────────────┤
│  Video URL                      [Auto-detected] ✨      │
│  ┌───────────────────────────────────────────────────┐  │
│  │ https://บ้านจีน.com/video/123     [✖] [🔍]       │  │
│  └───────────────────────────────────────────────────┘  │
│  ✅ Supported site                                      │
│                                                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │ 🖼️ [Thumbnail]  สามีเธอน่ะ ฉันขอนะ EP.5           │  │
│  │                 🎬 3 sources • 1080p, 720p, 480p  │  │
│  └───────────────────────────────────────────────────┘  │
│                                                         │
│  ┌─────────────────────────────────────────────────────┐│
│  │           💜 DOWNLOAD VIDEO                        ││
│  └─────────────────────────────────────────────────────┘│
│                                                         │
│  Download Progress              [⏳ Downloading]        │
│  ████████████████░░░░░░░░░░░░░░░░░░░░░░░░░  45.2%      │
│  15.3 MB / 34.0 MB              [2.5 MB/s] [0:45 left] │
└─────────────────────────────────────────────────────────┘
```

---

## ✨ ฟีเจอร์

### 🎯 ฟีเจอร์หลัก

| ฟีเจอร์ | รายละเอียด |
|---------|------------|
| 🌐 **รองรับเว็บไทย** | บ้านจีน.com, หนังสั้นจีน.online |
| 🎥 **เลือกคุณภาพ** | 480p, 720p, 1080p หรือ auto |
| 📊 **แสดง Progress** | Progress bar พร้อม Speed และ ETA |
| 📜 **ประวัติการดาวน์โหลด** | ดูและจัดการประวัติได้ |
| 🌙 **Dark Theme** | ธีมสีเข้มสบายตา พร้อม Glassmorphism UI |

### 🚀 ฟีเจอร์ใหม่ (v1.0.0)

| ฟีเจอร์ | รายละเอียด |
|---------|------------|
| 📋 **Clipboard Auto-Paste** | ตรวจจับ URL จาก clipboard อัตโนมัติ |
| ⌨️ **Keyboard Shortcuts** | Enter, Escape, Ctrl+O, Ctrl+V |
| 🔔 **Desktop Notification** | แจ้งเตือนเมื่อดาวน์โหลดเสร็จ |
| ⚡ **Speed & ETA** | แสดงความเร็วและเวลาที่เหลือ |
| ✅ **URL Validation** | ตรวจสอบเว็บไซต์ที่รองรับ |

### ⌨️ Keyboard Shortcuts

| ปุ่ม | การทำงาน |
|------|----------|
| `Enter` | ดึงข้อมูลวิดีโอ / เริ่มดาวน์โหลด |
| `Escape` | ล้าง URL / ปิด dropdown |
| `Ctrl+O` | เปิดโฟลเดอร์ดาวน์โหลด |
| `Ctrl+V` | วาง URL + ตรวจจับอัตโนมัติ |

---

## 📥 ดาวน์โหลด

### 💿 ไฟล์ติดตั้งสำเร็จรูป

| ระบบปฏิบัติการ | ไฟล์ | ขนาด |
|----------------|------|------|
| 🪟 **Windows** | [ThaiVideoDownloader_x64.msi](https://github.com/user/repo/releases/latest) | ~15 MB |
| 🍎 **macOS (Intel)** | [ThaiVideoDownloader_x64.dmg](https://github.com/user/repo/releases/latest) | ~12 MB |
| 🍎 **macOS (Apple Silicon)** | [ThaiVideoDownloader_aarch64.dmg](https://github.com/user/repo/releases/latest) | ~12 MB |
| 🐧 **Linux (Debian/Ubuntu)** | [ThaiVideoDownloader_amd64.deb](https://github.com/user/repo/releases/latest) | ~10 MB |
| 🐧 **Linux (AppImage)** | [ThaiVideoDownloader_x86_64.AppImage](https://github.com/user/repo/releases/latest) | ~15 MB |

> 📝 **หมายเหตุ**: ต้องติดตั้ง Chrome/Chromium สำหรับการดึงข้อมูลวิดีโอ

---

## 🛠️ การติดตั้ง

### 📋 ความต้องการของระบบ

- 🪟 Windows 10/11 หรือ 🍎 macOS 10.15+ หรือ 🐧 Linux (Ubuntu 20.04+)
- 🌐 Chrome หรือ Chromium browser
- 🎞️ FFmpeg (แนะนำ - สำหรับการแปลงไฟล์)

### 🔧 สำหรับนักพัฒนา

#### 1️⃣ Clone โปรเจค

```bash
git clone https://github.com/user/thai-video-downloader.git
cd thai-video-downloader
```

#### 2️⃣ ติดตั้ง Dependencies

```bash
# Frontend
cd gui
npm install

# Rust (Tauri จะติดตั้งอัตโนมัติ)
```

#### 3️⃣ รัน Development Mode

```bash
npm run tauri dev
```

#### 4️⃣ Build สำหรับ Production

```bash
npm run tauri build
```

---

## 📖 วิธีใช้งาน

### 🎬 ดาวน์โหลดวิดีโอ

1. 📋 **คัดลอก URL** จากเว็บไซต์ที่รองรับ
2. 🚀 **เปิดแอป** - URL จะถูกตรวจจับอัตโนมัติ!
3. 🔍 **กด Enter** หรือคลิก 🔍 เพื่อดึงข้อมูลวิดีโอ
4. 📊 **เลือกคุณภาพ** ที่ต้องการ (480p, 720p, 1080p)
5. 📥 **กด Download Video** หรือกด Enter อีกครั้ง
6. ⏳ **รอจนเสร็จ** - ดู Progress, Speed และ ETA แบบ Real-time
7. 🔔 **ได้รับแจ้งเตือน** เมื่อดาวน์โหลดเสร็จ!

### 📜 ดูประวัติ

1. 📜 คลิกแท็บ **History**
2. 🎬 ดูรายการวิดีโอที่เคยดาวน์โหลด
3. ▶️ คลิก **Play** เพื่อเปิดวิดีโอ
4. 📁 คลิก **Folder** เพื่อเปิดโฟลเดอร์
5. 🗑️ คลิก **Delete** เพื่อลบจากประวัติ

---

## 🏗️ โครงสร้างโปรเจค

```
📁 thai-video-downloader/
├── 📁 gui/                          # 🖥️ Desktop App
│   ├── 📁 src/                      # ⚛️ React Frontend
│   │   ├── 📄 App.tsx               # 🎨 Main Component (~850 lines)
│   │   ├── 📄 App.css               # 💅 Styles (~1050 lines)
│   │   └── 📄 main.tsx              # 🚀 Entry Point
│   └── 📁 src-tauri/                # 🦀 Rust Backend
│       ├── 📁 src/
│       │   ├── 📄 lib.rs            # 📡 Tauri Commands
│       │   ├── 📄 main.rs           # 🏁 Entry Point
│       │   └── 📁 downloader/       # ⬇️ Download Engine
│       │       ├── 📄 mod.rs        # 📦 Module Definitions
│       │       ├── 📄 browser.rs    # 🌐 Browser Automation
│       │       ├── 📄 hls.rs        # 📹 HLS/M3U8 Handler
│       │       └── 📄 video.rs      # 🎬 Video Downloader
│       └── 📄 Cargo.toml            # 📋 Rust Dependencies
├── 📁 .github/workflows/            # 🔄 CI/CD
│   └── 📄 release.yml               # 🚀 Auto Release
├── 📁 .miracle/                     # 🧠 Miracle Second Brain
├── 📄 README.md                     # 📖 This File!
└── 📄 TODO.md                       # ✅ Task List
```

---

## 🔧 เทคโนโลยีที่ใช้

<div align="center">

| Frontend | Backend | Tools |
|----------|---------|-------|
| ⚛️ React 19 | 🦀 Rust | 🔨 Tauri v2 |
| 📘 TypeScript | 🌐 Chromiumoxide | ⚡ Vite 7 |
| 💅 CSS3 | 📹 m3u8-rs | 🎨 Lucide Icons |
| 🎨 Glassmorphism | 🌐 Reqwest | 📦 npm |

</div>

---

## 🤝 Contributing

เรายินดีต้อนรับการมีส่วนร่วม! 🎉

### 📝 วิธีการ Contribute

1. 🍴 **Fork** โปรเจคนี้
2. 🌿 **สร้าง Branch** (`git checkout -b feature/amazing-feature`)
3. 💾 **Commit** การเปลี่ยนแปลง (`git commit -m 'Add amazing feature'`)
4. 📤 **Push** ไปยัง Branch (`git push origin feature/amazing-feature`)
5. 🔀 **สร้าง Pull Request**

### 🐛 รายงาน Bug

พบปัญหา? [สร้าง Issue](https://github.com/user/repo/issues/new) พร้อมข้อมูล:
- 📝 รายละเอียดปัญหา
- 🔄 ขั้นตอนการ reproduce
- 💻 ระบบปฏิบัติการและเวอร์ชัน
- 📸 Screenshot (ถ้ามี)

---

## 📜 License

โปรเจคนี้อยู่ภายใต้ **MIT License** - ดูไฟล์ [LICENSE](LICENSE) สำหรับรายละเอียด

---

## 🙏 ขอบคุณ

- 🦀 [Tauri](https://tauri.app/) - สำหรับ Framework ที่ยอดเยี่ยม
- ⚛️ [React](https://react.dev/) - สำหรับ UI Library
- 🎨 [Lucide](https://lucide.dev/) - สำหรับ Icons สวยๆ
- 🧠 **Miracle Second Brain** - สำหรับการจัดการความรู้

---

<div align="center">

**สร้างด้วย ❤️ โดยทีม Thai Video Downloader**

⭐ **ถ้าชอบโปรเจคนี้ อย่าลืมกด Star!** ⭐

</div>
