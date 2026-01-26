# aiDot Screenshot Capture

Captures screenshots from aiDot camera livestreams using Selenium. Runs two browser instances (one per camera), takes a screenshot every 5 seconds, and saves them with timestamps.

## Setup

Requires ChromeDriver. Easiest way is to run everything with Docker:

```bash
docker compose up -d
```

## Environment

Create a `.env` file:

```
AIDOT_USER=your@email.com
AIDOT_PASSWORD=yourpassword
URL_CAM_0=https://app.aidot.com/...
URL_CAM_1=https://app.aidot.com/...
```

## Output

Screenshots go to `/data/cam0/` and `/data/cam1/` (mapped via docker volume):
- `now.png` - latest frame, overwritten each time
- `YYYYMMDDHHmmss.png` - timestamped frames

## Timelapse

Generate a video from the day's captures:

```bash
python3 timelapse.py -y          # yesterday, deletes images after
python3 timelapse.py 20260124    # specific date
python3 timelapse.py --no-delete # keep images
```

Add to cron for daily timelapse at midnight:
```
0 0 * * * python3 /path/to/timelapse.py -y
```

## Watchdog

Monitors that images are being generated. Restarts containers if no new images in 15 min:

```bash
chmod +x watchdog.sh
```

Cron (runs every 15 min, skips first 45 min after midnight):
```
*/15 1-23 * * * /path/to/watchdog.sh
45 0 * * * /path/to/watchdog.sh
```

## Stack

- Rust + Tokio
- thirtyfour (Selenium bindings)
- selenium/standalone-chrome
