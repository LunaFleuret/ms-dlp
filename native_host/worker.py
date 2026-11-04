#!/usr/bin/env python
# -*- coding: utf-8 -*-

import sys
import os
import re
import json
import time
import pathlib
import subprocess

PROGRESS_REGEX = re.compile(
    r'\[download\]\s+([0-9\.]+)%\s+of\s+~?([^\s]+)\s+at\s+([^\s]+)\s+ETA\s+([^\s]+)'
)
PROGRESS_100_REGEX = re.compile(
    r'\[download\]\s+100%(?:\.0%)?\s+of\s+~?([^\s]+)'
)

def update_task_file(task_file, data):
    try:
        temp_file = task_file.with_suffix(".tmp")
        with open(temp_file, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)
        temp_file.replace(task_file)
    except Exception:
        pass

def decode_line(raw_bytes):
    if not raw_bytes:
        return ""
    try:
        return raw_bytes.decode("utf-8")
    except UnicodeDecodeError:
        try:
            return raw_bytes.decode("cp932")
        except Exception:
            return raw_bytes.decode("utf-8", errors="replace")

def main():
    if len(sys.argv) < 2:
        return

    task_file = pathlib.Path(sys.argv[1]).resolve()
    if not task_file.exists():
        return

    with open(task_file, "r", encoding="utf-8") as f:
        task_data = json.load(f)

    target_url = task_data.get("url", "").strip()
    if not target_url:
        task_data["status"] = "error"
        task_data["error"] = "Empty URL"
        update_task_file(task_file, task_data)
        return

    workspace_root = pathlib.Path(__file__).resolve().parent.parent
    config_file = workspace_root / "config.json"
    ps1_script = workspace_root / "download_music.ps1"

    if not ps1_script.exists():
        task_data["status"] = "error"
        task_data["error"] = f"Script not found: {ps1_script}"
        update_task_file(task_file, task_data)
        return

    # Apply options from popup to config.json
    options = task_data.get("options", {})
    if options and config_file.exists():
        try:
            with open(config_file, "r", encoding="utf-8") as f:
                cfg = json.load(f)
            if "audio_format" in options:
                cfg["audio_format"] = options["audio_format"]
            if "cover_mode_non_square" in options:
                cfg["cover_mode_non_square"] = options["cover_mode_non_square"]
            if "auto_crop_shorts" in options:
                cfg["auto_crop_shorts"] = bool(options["auto_crop_shorts"])
            if "embed_lyrics" in options:
                cfg["embed_lyrics"] = bool(options["embed_lyrics"])
            if "concurrent_fragments" in options:
                cfg["concurrent_fragments"] = int(options["concurrent_fragments"])
            with open(config_file, "w", encoding="utf-8") as f:
                json.dump(cfg, f, ensure_ascii=False, indent=2)
        except Exception:
            pass

    task_data["status"] = "downloading"
    task_data["percent"] = 0.0
    task_data["progress"] = 0.0
    task_data["total_size"] = ""
    task_data["speed"] = ""
    task_data["eta"] = ""
    update_task_file(task_file, task_data)

    cmd = [
        "powershell.exe",
        "-NoProfile",
        "-ExecutionPolicy", "Bypass",
        "-File", str(ps1_script)
    ]

    flags = 0
    if os.name == "nt":
        CREATE_NO_WINDOW = 0x08000000
        flags = CREATE_NO_WINDOW

    try:
        input_data = f"{target_url}\n\n".encode("utf-8")
        proc = subprocess.Popen(
            cmd,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            cwd=str(workspace_root),
            creationflags=flags
        )

        proc.stdin.write(input_data)
        proc.stdin.flush()
        proc.stdin.close()

        last_save_time = 0.0
        for raw_line in iter(proc.stdout.readline, b''):
            line = decode_line(raw_line).strip()
            if not line:
                continue

            match = PROGRESS_REGEX.search(line)
            if match:
                pct = float(match.group(1))
                task_data["percent"] = pct
                task_data["progress"] = pct
                task_data["total_size"] = match.group(2)
                task_data["speed"] = match.group(3)
                task_data["eta"] = match.group(4)
                task_data["status"] = "downloading"

                now = time.time()
                if now - last_save_time >= 0.3:
                    update_task_file(task_file, task_data)
                    last_save_time = now
                continue

            match_100 = PROGRESS_100_REGEX.search(line)
            if match_100:
                task_data["percent"] = 100.0
                task_data["progress"] = 100.0
                task_data["total_size"] = match_100.group(1)
                task_data["eta"] = "00:00"
                task_data["status"] = "downloading"
                update_task_file(task_file, task_data)
                continue

            if "Title  :" in line:
                title = line.split("Title  :", 1)[1].strip()
                if title:
                    task_data["title"] = title
                    update_task_file(task_file, task_data)
            elif "Checking and formatting cover art" in line or "Fetching lyrics" in line or "Tagging audio" in line:
                task_data["status"] = "converting"
                task_data["percent"] = 100.0
                task_data["progress"] = 100.0
                update_task_file(task_file, task_data)
            elif "[SUCCESS] Saved:" in line or "[SAVED] Saved:" in line:
                task_data["status"] = "completed"
                task_data["percent"] = 100.0
                task_data["progress"] = 100.0
                task_data["eta"] = "00:00"
                update_task_file(task_file, task_data)

        proc.wait()

        if task_data.get("status") != "completed":
            if proc.returncode == 0:
                task_data["status"] = "completed"
                task_data["percent"] = 100.0
                task_data["progress"] = 100.0
                task_data["eta"] = "00:00"
            else:
                task_data["status"] = "error"
                task_data["error"] = f"Exited with code {proc.returncode}"
            update_task_file(task_file, task_data)

    except Exception as e:
        task_data["status"] = "error"
        task_data["error"] = str(e)
        update_task_file(task_file, task_data)

if __name__ == "__main__":
    main()
