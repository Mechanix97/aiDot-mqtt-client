import os
import subprocess
import sys
from datetime import datetime, timedelta
from pathlib import Path

import requests
from dotenv import load_dotenv

load_dotenv(dotenv_path=Path(__file__).parent / ".env")

TELEGRAM_BOT_TOKEN = os.getenv("TELEGRAM_BOT_TOKEN")
TELEGRAM_CHAT_ID = os.getenv("TELEGRAM_CHAT_ID")

CAMERAS = {
    "cam0": "/home/lucas/home-assistant/data/cam0/",
    "cam1": "/home/lucas/home-assistant/data/cam1/",
}

OUTPUT_DIR = "/mnt/hdd/timelapses/"
FPS = 30

#  Add to cron
# crontab -e
# 0 0 * * * python3 /home/lucas/aiDot-mqtt-client/timelapse.py -y

# Manual run
# python3 timelapse.py 20260124 --no-delete
# python3 timelapse.py 20260124
# python3 timelapse.py --no-delete
# python3 timelapse.py --yesterday
# python3 timelapse.py -y --no-delete

def send_telegram_video(video_path, cam, date_str):
    if not TELEGRAM_BOT_TOKEN or not TELEGRAM_CHAT_ID:
        print(f"[{cam}] Telegram not configured, skipping")
        return

    url = f"https://api.telegram.org/bot{TELEGRAM_BOT_TOKEN}/sendVideo"
    with open(video_path, "rb") as video_file:
        response = requests.post(url, data={
            "chat_id": TELEGRAM_CHAT_ID,
            "caption": f"Timelapse {cam} - {date_str}",
        }, files={"video": video_file})

    if response.ok:
        print(f"[{cam}] Video sent to Telegram")
    else:
        print(f"[{cam}] Telegram error: {response.status_code} {response.text}")


def create_timelapse(date_str, delete=True):
    for cam, directory in CAMERAS.items():
        files = sorted([
            f for f in os.listdir(directory)
            if f.startswith(date_str) and f.endswith(".png") and f != "now.png"
        ])

        if not files:
            print(f"[{cam}] No images found for {date_str}")
            continue

        print(f"[{cam}] {len(files)} images found")

        # Create output directory
        cam_output_dir = os.path.join(OUTPUT_DIR, cam)
        os.makedirs(cam_output_dir, exist_ok=True)

        # Create temp file with image list for ffmpeg
        list_file = f"/tmp/timelapse_{cam}.txt"
        with open(list_file, "w") as f:
            for filename in files:
                path = os.path.join(directory, filename)
                f.write(f"file '{path}'\nduration {1/FPS}\n")

        output_file = os.path.join(cam_output_dir, f"{date_str}.mp4")

        # Generate video with ffmpeg H.264
        cmd = [
            "ffmpeg", "-y",
            "-f", "concat", "-safe", "0",
            "-i", list_file,
            "-vcodec", "libx264",
            "-crf", "23",
            "-preset", "medium",
            "-pix_fmt", "yuv420p",
            output_file
        ]

        result = subprocess.run(cmd, capture_output=True, text=True)

        if result.returncode == 0:
            print(f"[{cam}] Video saved: {output_file}")
            send_telegram_video(output_file, cam, date_str)
            if delete:
                for filename in files:
                    os.remove(os.path.join(directory, filename))
                print(f"[{cam}] {len(files)} images deleted")
        else:
            print(f"[{cam}] Error creating video: {result.stderr}")

        # Clean up temp file
        os.remove(list_file)


if __name__ == "__main__":
    args = [a for a in sys.argv[1:] if not a.startswith("-")]
    flags = [a for a in sys.argv[1:] if a.startswith("-")]

    yesterday = "--yesterday" in flags or "-y" in flags
    delete = "--no-delete" not in flags

    if args:
        date = args[0]
    elif yesterday:
        date = (datetime.now() - timedelta(days=1)).strftime("%Y%m%d")
    else:
        date = datetime.now().strftime("%Y%m%d")

    print(f"Processing date: {date}")
    create_timelapse(date, delete=delete)
