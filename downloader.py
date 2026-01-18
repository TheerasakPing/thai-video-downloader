#!/usr/bin/env python3
"""
Video Downloader for Thai streaming sites:
- บ้านจีน.com
- หนังสั้นจีน.online

Uses Playwright for dynamic content extraction and yt-dlp for downloading.
"""

import argparse
import asyncio
import json
import os
import re
import subprocess
import sys
import io
from pathlib import Path
from urllib.parse import urlparse, unquote

# Fix Windows encoding issues
if sys.platform == 'win32':
    sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
    sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace')

try:
    from playwright.async_api import async_playwright
except ImportError:
    print("Playwright not installed. Run: pip install playwright && playwright install chromium")
    sys.exit(1)


class VideoDownloader:
    # Known ad URL patterns to filter out
    AD_PATTERNS = [
        "adSrc",
        "/ad/",
        "advertisement",
        # Known ad video IDs
        "b7e06ea0-c18b-4b1e-9cba-2f7a9891f52f",
        "01dd0f98-3b37-40a5-ad47-20935908b632",
        "cf953e68-8e67-4135-9b39-746fe7557c10",
    ]

    # Patterns to skip (fragments, segments, etc.)
    SKIP_PATTERNS = [
        ".dts",
        ".ts",
        "/480p/video",
        "/720p/video",
        "/1080p/video",
    ]

    def __init__(self, output_dir: str = "./downloads", headless: bool = True):
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)
        self.headless = headless
        self.video_sources = []
        self.page_referer = None
        self.video_info = {}

    def is_ad_url(self, url: str) -> bool:
        """Check if URL is likely an advertisement."""
        url_lower = url.lower()
        return any(pattern.lower() in url_lower for pattern in self.AD_PATTERNS)

    def is_segment_url(self, url: str) -> bool:
        """Check if URL is a video segment (not the main playlist)."""
        url_lower = url.lower()
        return any(pattern.lower() in url_lower for pattern in self.SKIP_PATTERNS)

    async def get_video_info(self, url: str) -> dict:
        """Get video information without downloading."""
        print(f"[*] Getting video info: {url}")

        self.page_referer = url
        info = {
            "url": url,
            "title": "",
            "thumbnail": "",
            "duration": "",
            "qualities": [],
            "sources": []
        }

        async with async_playwright() as p:
            browser = await p.chromium.launch(headless=self.headless)
            context = await browser.new_context(
                user_agent="Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36"
            )
            page = await context.new_page()

            video_urls = []

            async def handle_response(response):
                resp_url = response.url
                content_type = response.headers.get("content-type", "")

                if self.is_segment_url(resp_url) or self.is_ad_url(resp_url):
                    return

                if any(ext in resp_url.lower() for ext in [".m3u8", ".mp4", ".webm"]):
                    quality = "unknown"
                    if "1080" in resp_url:
                        quality = "1080p"
                    elif "720" in resp_url:
                        quality = "720p"
                    elif "480" in resp_url:
                        quality = "480p"
                    elif "360" in resp_url:
                        quality = "360p"
                    video_urls.append({"url": resp_url, "quality": quality, "type": "direct"})

            page.on("response", handle_response)

            try:
                await page.goto(url, wait_until="networkidle", timeout=60000)
                await page.wait_for_timeout(3000)

                # Get page title
                info["title"] = await page.title()

                # Try to get video title from common selectors
                title_selectors = [
                    "h1", ".video-title", ".title", "#title",
                    "[class*='title']", "meta[property='og:title']"
                ]
                for selector in title_selectors:
                    try:
                        if selector.startswith("meta"):
                            elem = await page.query_selector(selector)
                            if elem:
                                content = await elem.get_attribute("content")
                                if content:
                                    info["title"] = content
                                    break
                        else:
                            elem = await page.query_selector(selector)
                            if elem:
                                text = await elem.inner_text()
                                if text and len(text) > 3:
                                    info["title"] = text.strip()
                                    break
                    except:
                        pass

                # Get thumbnail
                thumb_selectors = [
                    "meta[property='og:image']",
                    "video[poster]",
                    ".video-thumbnail img",
                    ".poster img"
                ]
                for selector in thumb_selectors:
                    try:
                        elem = await page.query_selector(selector)
                        if elem:
                            if selector.startswith("meta"):
                                info["thumbnail"] = await elem.get_attribute("content") or ""
                            elif "poster" in selector:
                                info["thumbnail"] = await elem.get_attribute("poster") or ""
                            else:
                                info["thumbnail"] = await elem.get_attribute("src") or ""
                            if info["thumbnail"]:
                                break
                    except:
                        pass

                # Try to click play buttons to trigger video loading
                play_buttons = await page.query_selector_all('[class*="play"], [id*="play"], .jw-icon-display, .vjs-big-play-button')
                for btn in play_buttons:
                    try:
                        await btn.click()
                        await page.wait_for_timeout(2000)
                    except:
                        pass

                # Extract iframe sources
                iframes = await page.query_selector_all("iframe")
                for iframe in iframes:
                    src = await iframe.get_attribute("src")
                    lazy_src = await iframe.get_attribute("data-lazy-src")
                    iframe_url = src or lazy_src

                    if iframe_url and iframe_url.startswith("http"):
                        try:
                            iframe_page = await context.new_page()
                            iframe_page.on("response", handle_response)
                            await iframe_page.goto(iframe_url, wait_until="networkidle", timeout=30000)
                            await iframe_page.wait_for_timeout(2000)

                            # Extract video sources from iframe content
                            content = await iframe_page.content()

                            # Find m3u8 URLs with quality
                            m3u8_matches = re.findall(r'(https?://[^\s"\'<>]+\.m3u8[^\s"\'<>]*)', content)
                            for m3u8 in m3u8_matches:
                                if not self.is_ad_url(m3u8):
                                    quality = "auto"
                                    if "1080" in m3u8:
                                        quality = "1080p"
                                    elif "720" in m3u8:
                                        quality = "720p"
                                    elif "480" in m3u8:
                                        quality = "480p"
                                    video_urls.append({"url": m3u8, "quality": quality, "type": "hls"})

                            await iframe_page.close()
                        except:
                            pass

                # Also check main page content
                content = await page.content()

                video_src_matches = re.findall(
                    r'(?:videoSrc|mainSrc|video_src|hlsUrl|streamUrl)\s*[=:]\s*["\']([^"\']+)["\']',
                    content,
                    re.IGNORECASE
                )
                for src in video_src_matches:
                    if src.startswith("http") and not self.is_ad_url(src):
                        quality = "auto"
                        if "1080" in src:
                            quality = "1080p"
                        elif "720" in src:
                            quality = "720p"
                        elif "480" in src:
                            quality = "480p"
                        video_urls.append({"url": src, "quality": quality, "type": "main_video"})

            except Exception as e:
                print(f"[!] Error getting video info: {e}")
            finally:
                await browser.close()

        # Deduplicate
        seen = set()
        unique_sources = []
        qualities = set()

        for item in video_urls:
            if item["url"] not in seen:
                seen.add(item["url"])
                unique_sources.append(item)
                if item["quality"] != "unknown":
                    qualities.add(item["quality"])

        info["sources"] = unique_sources
        info["qualities"] = sorted(list(qualities), key=lambda x: int(x.replace("p", "").replace("auto", "0")) if x != "auto" else 0, reverse=True)

        if not info["qualities"]:
            info["qualities"] = ["auto"]

        self.video_info = info
        self.video_sources = unique_sources

        return info

    async def extract_video_sources(self, url: str) -> list[dict]:
        """Extract video sources from the page using Playwright."""
        info = await self.get_video_info(url)
        return info.get("sources", [])

    def download_with_ytdlp(self, url: str, filename: str = None, quality: str = "best") -> bool:
        """Download video using yt-dlp."""
        output_template = str(self.output_dir / (filename if filename else "%(title)s.%(ext)s"))

        cmd = [
            "yt-dlp",
            "--no-check-certificate",
            "-o", output_template,
            "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
            "--newline",  # For progress parsing
        ]

        # Quality selection
        if quality and quality != "best" and quality != "auto":
            height = quality.replace("p", "")
            cmd.extend(["-f", f"bestvideo[height<={height}]+bestaudio/best[height<={height}]/best"])

        # Add referer if available
        if self.page_referer:
            cmd.extend(["--referer", self.page_referer])

        cmd.append(url)

        print(f"[*] Downloading with yt-dlp: {url[:80]}...")

        try:
            result = subprocess.run(cmd, capture_output=False)
            return result.returncode == 0
        except FileNotFoundError:
            print("[!] yt-dlp not found. Install it with: pip install yt-dlp")
            return False
        except Exception as e:
            print(f"[!] Download error: {e}")
            return False

    def download_with_ffmpeg(self, url: str, filename: str) -> bool:
        """Download HLS stream using ffmpeg."""
        output_path = self.output_dir / filename

        cmd = [
            "ffmpeg",
            "-user_agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        ]

        # Add referer if available
        if self.page_referer:
            cmd.extend(["-referer", self.page_referer])

        cmd.extend([
            "-i", url,
            "-c", "copy",
            "-bsf:a", "aac_adtstoasc",
            str(output_path)
        ])

        print(f"[*] Downloading with ffmpeg: {url[:80]}...")

        try:
            result = subprocess.run(cmd, capture_output=True)
            return result.returncode == 0
        except FileNotFoundError:
            print("[!] ffmpeg not found. Please install ffmpeg.")
            return False
        except Exception as e:
            print(f"[!] Download error: {e}")
            return False

    async def download(self, page_url: str, output_filename: str = None, quality: str = "best"):
        """Main download method."""
        # Extract video sources if not already done
        if not self.video_sources:
            sources = await self.extract_video_sources(page_url)
        else:
            sources = self.video_sources

        if not sources:
            print("[!] No video sources found. Trying direct yt-dlp...")
            if self.download_with_ytdlp(page_url, output_filename, quality):
                print("[+] Download completed!")
                return True
            return False

        print(f"\n[*] Found {len(sources)} video source(s):")
        for i, src in enumerate(sources):
            print(f"  {i+1}. [{src.get('quality', 'unknown')}] {src['url'][:100]}...")

        # Find matching quality source
        selected_source = None
        if quality and quality != "best" and quality != "auto":
            for src in sources:
                if src.get("quality") == quality:
                    selected_source = src
                    break

        if not selected_source:
            selected_source = sources[0]

        url = selected_source["url"].replace("\\", "")

        if ".m3u8" in url:
            fname = output_filename or "video.mp4"
            if self.download_with_ytdlp(url, fname, quality):
                print("[+] Download completed!")
                return True
            elif self.download_with_ffmpeg(url, fname):
                print("[+] Download completed!")
                return True
        else:
            if self.download_with_ytdlp(url, output_filename, quality):
                print("[+] Download completed!")
                return True

        print("[!] Could not download any video source.")
        return False


def extract_title_from_url(url: str) -> str:
    """Extract a filename from the URL path."""
    path = urlparse(url).path
    path = unquote(path)
    segments = [s for s in path.split("/") if s]
    if segments:
        title = segments[-1]
        title = re.sub(r'[<>:"/\\|?*]', '_', title)
        return title
    return "video"


async def main():
    parser = argparse.ArgumentParser(
        description="Download videos from Thai streaming sites",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Supported sites:
  - บ้านจีน.com
  - หนังสั้นจีน.online

Examples:
  python downloader.py "URL"
  python downloader.py "URL" -o my_video.mp4
  python downloader.py "URL" -q 720p
  python downloader.py "URL" --info
        """
    )
    parser.add_argument("url", help="URL of the video page")
    parser.add_argument("-o", "--output", help="Output filename", default=None)
    parser.add_argument("-d", "--dir", help="Output directory", default="./downloads")
    parser.add_argument("-q", "--quality", help="Video quality (480p, 720p, 1080p, best)", default="best")
    parser.add_argument("--info", action="store_true", help="Only get video info, don't download")
    parser.add_argument("--no-headless", action="store_true", help="Show browser window")

    args = parser.parse_args()

    downloader = VideoDownloader(
        output_dir=args.dir,
        headless=not args.no_headless
    )

    if args.info:
        info = await downloader.get_video_info(args.url)
        print(json.dumps(info, indent=2, ensure_ascii=False))
        return

    output_name = args.output
    if not output_name:
        output_name = extract_title_from_url(args.url) + ".mp4"

    success = await downloader.download(args.url, output_name, args.quality)
    sys.exit(0 if success else 1)


if __name__ == "__main__":
    asyncio.run(main())
