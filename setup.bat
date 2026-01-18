@echo off
echo Installing dependencies...
pip install -r requirements.txt

echo.
echo Installing Playwright browser...
playwright install chromium

echo.
echo Setup complete!
echo.
echo Usage:
echo   python downloader.py "https://xn--82c7abb4jua0l.com/video-page/"
echo.
pause
