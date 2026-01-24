import os
import subprocess
import sys
from datetime import datetime, timedelta

CAMERAS = {
    "cam0": "/home/lucas/home-assistant/data/cam0/",
    "cam1": "/home/lucas/home-assistant/data/cam1/",
}

OUTPUT_DIR = "/mnt/hdd/timelapses/"
FPS = 30

#  Add to cron
# crontab -e
# 0 0 * * * python3 /home/lucas/aiDot-mqtt-client/timelapse.py

# Manual run
# python3 timelapse.py 20260124

def create_timelapse(date_str):
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
            # Delete processed images
            for filename in files:
                os.remove(os.path.join(directory, filename))
            print(f"[{cam}] {len(files)} images deleted")
        else:
            print(f"[{cam}] Error creating video: {result.stderr}")

        # Clean up temp file
        os.remove(list_file)


if __name__ == "__main__":
    if len(sys.argv) > 1:
        date = sys.argv[1]
    else:
        # Default: process yesterday
        date = (datetime.now() - timedelta(days=1)).strftime("%Y%m%d")

    print(f"Processing date: {date}")
    create_timelapse(date)
