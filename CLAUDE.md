# aiDot MQTT Client

## Description

Rust MQTT client that captures screenshots from aiDot platform cameras. Connects to a local MQTT broker, listens for messages on camera topics, and uses Selenium (Chrome WebDriver) to navigate to the livestream and capture frames.

## Stack

- **Language**: Rust (edition 2024)
- **Async runtime**: Tokio (features = "full")
- **MQTT**: rumqttc 0.24
- **Web automation**: thirtyfour 0.35 (Selenium WebDriver)
- **Serialization**: serde + serde_json
- **Date/time**: chrono (Argentina timezone UTC-3)
- **Config**: dotenv

## Structure

```
src/main.rs       -- All application logic (~256 lines)
timelapse.py      -- Python script to generate timelapse video with OpenCV
Dockerfile        -- Containerization
.env              -- Environment variables (credentials, camera URLs)
```

## Build & Run

```bash
cargo build --release
cargo run --release
```

Requires ChromeDriver running on `localhost:9515` and MQTT broker on `localhost:1883`.

## Environment Variables (.env)

- `AIDOT_USER` - Login email for app.aidot.com
- `AIDOT_PASSWORD` - Password
- `URL_CAM_0` - Camera 0 livestream URL
- `URL_CAM_1` - Camera 1 livestream URL

## Architecture

1. **MQTT listener**: Subscribes to `aidot/get/cam0` and `aidot/get/cam1` (QoS: AtLeastOnce)
2. **Heartbeat**: Publishes a message every 10 seconds
3. **2 Tokio tasks** (one per camera): Each task controls a Chrome WebDriver, authenticates to aiDot, navigates to the livestream, and waits for MQTT messages to capture screenshots
4. **Broadcast channel**: Distributes MQTT messages to both tasks
5. **Auto-refresh**: Reloads the page if video doesn't load within 30s

## Output Paths

Captures are saved to:
- `/home/lucas/home-assistant/data/cam0/` (camera 0)
- `/home/lucas/home-assistant/data/cam1/` (camera 1)

Each capture generates:
- `now.png` - Latest frame (overwritten each time)
- `{YYYYMMDDHHmmss}.png` - Timestamped frame

## Conventions

- All code lives in `src/main.rs` (single file)
- Output paths are hardcoded in source
- Credentials are managed via .env (never commit)
- `.gitignore` excludes: `/target`, `.env`, `aiDot.env`, `data/*`
