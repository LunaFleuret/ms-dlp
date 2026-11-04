#!/usr/bin/env python
# -*- coding: utf-8 -*-

import sys
import os
import json
import time
import struct
import uuid
import pathlib
import subprocess

def send_message(message_dict):
    """Send a message back to Chrome/Edge via standard output."""
    encoded_content = json.dumps(message_dict, ensure_ascii=False).encode("utf-8")
    length = len(encoded_content)
    sys.stdout.buffer.write(struct.pack("I", length))
    sys.stdout.buffer.write(encoded_content)
    sys.stdout.buffer.flush()

def read_message():
    """Read a message from Chrome/Edge via standard input."""
    raw_length = sys.stdin.buffer.read(4)
    if not raw_length:
        return None
    length = struct.unpack("I", raw_length)[0]
    raw_content = sys.stdin.buffer.read(length)
    return json.loads(raw_content.decode("utf-8"))

def get_base_dir():
    return pathlib.Path(__file__).resolve().parent

def get_tasks_dir():
    tasks_dir = get_base_dir() / "tasks"
    tasks_dir.mkdir(parents=True, exist_ok=True)
    return tasks_dir

def handle_start_download(msg):
    url = msg.get("url")
    if not url:
        return {"status": "error", "message": "Missing URL"}

    task_id = str(uuid.uuid4())[:8]
    tasks_dir = get_tasks_dir()
    task_file = tasks_dir / f"task_{task_id}.json"

    options = msg.get("options", {})

    task_data = {
        "id": task_id,
        "url": url,
        "title": msg.get("title", url),
        "status": "queued",
        "progress": 0.0,
        "speed": "",
        "eta": "",
        "error": "",
        "created_at": time.time(),
        "options": options
    }

    with open(task_file, "w", encoding="utf-8") as f:
        json.dump(task_data, f, ensure_ascii=False, indent=2)

    # Spawn worker.py in detached background process
    worker_script = get_base_dir() / "worker.py"
    flags = 0
    if os.name == "nt":
        DETACHED_PROCESS = 0x00000008
        CREATE_NO_WINDOW = 0x08000000
        flags = DETACHED_PROCESS | CREATE_NO_WINDOW

    cmd = [sys.executable, str(worker_script), str(task_file)]
    subprocess.Popen(
        cmd,
        creationflags=flags,
        close_fds=True
    )

    return {"status": "success", "task_id": task_id}

def handle_get_tasks():
    tasks_dir = get_tasks_dir()
    tasks = []
    now = time.time()

    for item in tasks_dir.glob("task_*.json"):
        try:
            with open(item, "r", encoding="utf-8") as f:
                data = json.load(f)
                tasks.append(data)
        except Exception:
            continue

    # Cleanup completed tasks older than 12 hours
    for t in tasks:
        if t.get("status") in ("completed", "error"):
            if now - t.get("created_at", now) > 43200:
                tid = t.get("id")
                try:
                    (tasks_dir / f"task_{tid}.json").unlink(missing_ok=True)
                except Exception:
                    pass

    tasks.sort(key=lambda x: x.get("created_at", 0), reverse=True)
    return {"status": "success", "tasks": tasks}

def handle_clear_finished():
    tasks_dir = get_tasks_dir()
    for item in tasks_dir.glob("task_*.json"):
        try:
            with open(item, "r", encoding="utf-8") as f:
                data = json.load(f)
            if data.get("status") in ("completed", "error"):
                item.unlink(missing_ok=True)
        except Exception:
            pass
    return {"status": "success"}

def handle_open_output_dir():
    try:
        base_dir = get_base_dir()
        workspace_root = base_dir.parent
        config_path = workspace_root / "config.json"
        
        output_folder_name = "output"
        if config_path.exists():
            try:
                with open(config_path, "r", encoding="utf-8") as f:
                    cfg = json.load(f)
                    output_folder_name = cfg.get("output_folder", "output")
            except Exception:
                pass
                
        target_path = pathlib.Path(output_folder_name)
        if not target_path.is_absolute():
            target_path = (workspace_root / target_path).resolve()
            
        target_path.mkdir(parents=True, exist_ok=True)
        
        CREATE_NO_WINDOW = 0x08000000
        opened = False
        
        if sys.platform == "win32":
            # 1. Priority: Files Preview (files-preview.exe)
            try:
                subprocess.Popen(["files-preview.exe", str(target_path)], creationflags=CREATE_NO_WINDOW)
                opened = True
            except Exception:
                pass
                
            # 2. Priority: Files Stable (files-stable.exe)
            if not opened:
                try:
                    subprocess.Popen(["files-stable.exe", str(target_path)], creationflags=CREATE_NO_WINDOW)
                    opened = True
                except Exception:
                    pass

            # 3. Priority: Files URI Protocol
            if not opened:
                for uri_scheme in ["files-preview-uwp:", "files-uwp:"]:
                    try:
                        subprocess.Popen(
                            ["cmd.exe", "/c", "start", "", f"{uri_scheme}{target_path}"],
                            creationflags=CREATE_NO_WINDOW
                        )
                        opened = True
                        break
                    except Exception:
                        pass
                        
            # 4. Fallback: OS Explorer
            if not opened:
                try:
                    os.startfile(str(target_path))
                    opened = True
                except Exception:
                    subprocess.Popen(["explorer.exe", str(target_path)])
                    opened = True
        else:
            if sys.platform == "darwin":
                subprocess.Popen(["open", str(target_path)])
            else:
                subprocess.Popen(["xdg-open", str(target_path)])
            opened = True
            
        return {"status": "success", "opened": opened}
    except Exception as e:
        return {"status": "error", "message": str(e)}

def main():
    while True:
        try:
            msg = read_message()
            if msg is None:
                break

            action = msg.get("action")
            if action == "start_download":
                resp = handle_start_download(msg)
            elif action == "get_tasks":
                resp = handle_get_tasks()
            elif action == "clear_finished":
                resp = handle_clear_finished()
            elif action == "open_output_dir":
                resp = handle_open_output_dir()
            else:
                resp = {"status": "error", "message": f"Unknown action: {action}"}

            send_message(resp)
        except Exception as e:
            send_message({"status": "error", "message": str(e)})

if __name__ == "__main__":
    main()
