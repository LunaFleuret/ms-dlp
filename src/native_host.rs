use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct NativeMessage {
    pub action: String,
    pub url: Option<String>,
    pub title: Option<String>,
    pub options: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TaskData {
    pub id: String,
    pub url: String,
    pub title: String,
    pub status: String, // "queued", "downloading", "completed", "error"
    pub progress: f32,
    pub percent: f32,
    pub speed: String,
    pub eta: String,
    pub error: String,
    pub created_at: f64,
    pub options: Option<serde_json::Value>,
}

pub fn read_native_message() -> Result<Option<NativeMessage>> {
    let mut stdin = std::io::stdin().lock();
    let mut len_buf = [0u8; 4];

    if stdin.read_exact(&mut len_buf).is_err() {
        return Ok(None);
    }

    let msg_len = u32::from_le_bytes(len_buf) as usize;
    if msg_len == 0 || msg_len > 10 * 1024 * 1024 {
        return Ok(None);
    }

    let mut body = vec![0u8; msg_len];
    stdin.read_exact(&mut body)?;

    let msg: NativeMessage = serde_json::from_slice(&body)?;
    Ok(Some(msg))
}

pub fn write_native_message<T: Serialize>(val: &T) -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    let body = serde_json::to_vec(val)?;
    let len = (body.len() as u32).to_le_bytes();

    stdout.write_all(&len)?;
    stdout.write_all(&body)?;
    stdout.flush()?;
    Ok(())
}

pub fn update_task_file(tasks_dir: &Path, task: &TaskData) {
    let file_path = tasks_dir.join(format!("task_{}.json", task.id));
    let tmp_path = tasks_dir.join(format!("task_{}.tmp", task.id));

    if let Ok(json_str) = serde_json::to_string_pretty(task) {
        if std::fs::write(&tmp_path, json_str).is_ok() {
            let _ = std::fs::rename(tmp_path, file_path);
        }
    }
}

pub fn load_all_tasks(tasks_dir: &Path) -> Vec<TaskData> {
    let mut tasks = Vec::new();
    if let Ok(entries) = std::fs::read_dir(tasks_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    if let Ok(t) = serde_json::from_str::<TaskData>(&content) {
                        tasks.push(t);
                    }
                }
            }
        }
    }
    tasks.sort_by(|a, b| b.created_at.partial_cmp(&a.created_at).unwrap_or(std::cmp::Ordering::Equal));
    tasks
}
