#!/bin/bash

# Watchdog script for aiDot MQTT client
# Checks if images are being generated and restarts containers if not
#
# Add to crontab (every 15 minutes, skipping first 45min after midnight):
# */15 1-23 * * * /home/lucas/aiDot-mqtt-client/watchdog.sh
# 45 0 * * * /home/lucas/aiDot-mqtt-client/watchdog.sh

CAM0_DIR="/home/lucas/home-assistant/data/cam0"
CAM1_DIR="/home/lucas/home-assistant/data/cam1"
COMPOSE_DIR="/home/lucas/aiDot-mqtt-client"
LOG_FILE="/home/lucas/aiDot-mqtt-client/watchdog.log"

# Max age in minutes for images to be considered "recent"
MAX_AGE_MINUTES=15

log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $1" >> "$LOG_FILE"
}

check_recent_images() {
    local dir="$1"

    # Find images modified in the last MAX_AGE_MINUTES minutes (excluding now.png)
    recent_count=$(find "$dir" -name "*.png" ! -name "now.png" -mmin -"$MAX_AGE_MINUTES" 2>/dev/null | wc -l)

    [ "$recent_count" -gt 0 ]
}

# Skip check during first 45 minutes after midnight (timelapse runs at 00:00)
current_hour=$(date +%H)
current_minute=$(date +%M)

if [ "$current_hour" -eq 0 ] && [ "$current_minute" -lt 45 ]; then
    log "Skipping check - too close to midnight cleanup"
    exit 0
fi

# Check both cameras
cam0_ok=true
cam1_ok=true

if ! check_recent_images "$CAM0_DIR"; then
    cam0_ok=false
    log "WARNING: No recent images in cam0"
fi

if ! check_recent_images "$CAM1_DIR"; then
    cam1_ok=false
    log "WARNING: No recent images in cam1"
fi

# If either camera has no recent images, restart containers
if [ "$cam0_ok" = false ] || [ "$cam1_ok" = false ]; then
    log "Restarting docker containers..."

    cd "$COMPOSE_DIR" || exit 1
    docker compose restart >> "$LOG_FILE" 2>&1

    if [ $? -eq 0 ]; then
        log "Containers restarted successfully"
    else
        log "ERROR: Failed to restart containers"
    fi
else
    log "OK: Both cameras generating images"
fi
