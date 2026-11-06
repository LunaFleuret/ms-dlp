use anyhow::{anyhow, Result};
use regex::Regex;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

#[derive(Debug, Deserialize, Default)]
pub struct SourceMetadata {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub uploader: Option<String>,
    pub channel: Option<String>,
    pub description: Option<String>,
}

pub fn normalize_url(raw_url: &str) -> String {
    let mut u = raw_url.trim().to_string();
    if u.starts_with("ttps://") || u.starts_with("ttp://") {
        u = format!("h{}", u);
    } else if !u.contains("://") {
        u = format!("https://{}", u);
    }

    let yt_re = Regex::new(r"(?:youtube\.com|music\.youtube\.com)/watch\?.*v=([a-zA-Z0-9_-]{11})").unwrap();
    if let Some(caps) = yt_re.captures(&u) {
        if let Some(vid) = caps.get(1) {
            return format!("https://www.youtube.com/watch?v={}", vid.as_str());
        }
    }

    let youtu_be = Regex::new(r"youtu\.be/([a-zA-Z0-9_-]{11})").unwrap();
    if let Some(caps) = youtu_be.captures(&u) {
        if let Some(vid) = caps.get(1) {
            return format!("https://www.youtube.com/watch?v={}", vid.as_str());
        }
    }

    u
}

pub async fn execute_ytdlp_download<F>(
    yt_dlp_path: &Path,
    ffmpeg_path: &Path,
    session_dir: &Path,
    url: &str,
    format: &str,
    quality: &str,
    concurrent: u32,
    embed_lyrics: bool,
    cookie_args: &[String],
    mut on_progress: F,
) -> Result<PathBuf>
where
    F: FnMut(f32, &str, &str), // percent, speed, eta
{
    let file_template = session_dir.join("%(title)s.%(ext)s").to_string_lossy().to_string();

    let mut cmd = Command::new(yt_dlp_path);
    cmd.args([
        "--newline",
        "--no-playlist",
        "-N", &concurrent.to_string(),
        "--ffmpeg-location", &ffmpeg_path.to_string_lossy(),
        "-x", "--audio-format", format, "--audio-quality", quality,
        "--write-thumbnail", "--convert-thumbnails", "jpg",
        "--write-info-json", "--no-embed-chapters",
    ]);

    if embed_lyrics {
        cmd.args(["--write-subs", "--sub-langs", "ja,en,ja-JP,en-US", "--sub-format", "vtt/lrc/best"]);
    }

    if url.contains("youtube.com") || url.contains("youtu.be") {
        println!("  [Site] Detected: YouTube / YouTube Music");
        cmd.args(["--extractor-args", "youtube:player_client=default,-1"]);
    } else if url.contains("twitter.com") || url.contains("x.com") {
        println!("  [Site] Detected: X (Twitter)");
    } else if url.contains("tiktok.com") {
        println!("  [Site] Detected: TikTok");
        cmd.args([
            "--user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
            "--referer", "https://www.tiktok.com/",
        ]);
    } else if url.contains("nicovideo.jp") || url.contains("nico.ms") {
        println!("  [Site] Detected: Niconico");
    } else {
        println!("  [Site] Detected: Generic Web Source");
    }

    for c in cookie_args {
        cmd.arg(c);
    }

    cmd.args(["-o", &file_template, url]);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().ok_or_else(|| anyhow!("Failed to capture stdout"))?;
    let mut reader = BufReader::new(stdout).lines();

    let prog_re = Regex::new(r"\[download\]\s+([0-9\.]+)%\s+of\s+~?([^\s]+)\s+at\s+([^\s]+)\s+ETA\s+([^\s]+)").unwrap();

    while let Ok(Some(line)) = reader.next_line().await {
        println!("{}", line);
        if let Some(caps) = prog_re.captures(&line) {
            let pct: f32 = caps.get(1).and_then(|m| m.as_str().parse().ok()).unwrap_or(0.0);
            let speed = caps.get(3).map(|m| m.as_str()).unwrap_or("");
            let eta = caps.get(4).map(|m| m.as_str()).unwrap_or("");
            on_progress(pct, speed, eta);
        }
    }

    let status = child.wait().await?;
    if !status.success() {
        return Err(anyhow!("yt-dlp exited with error code {:?}", status.code()));
    }

    // Locate the downloaded audio file
    let extensions = [format, "m4a", "mp3", "opus", "webm", "flac"];
    if let Ok(entries) = std::fs::read_dir(session_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Some(ext) = p.extension().and_then(|s| s.to_str()) {
                if extensions.iter().any(|&e| e.eq_ignore_ascii_case(ext)) {
                    return Ok(p);
                }
            }
        }
    }

    Err(anyhow!("Downloaded audio file not found in session directory"))
}

pub fn read_source_metadata(session_dir: &Path) -> (String, String, String) {
    let mut title = String::new();
    let mut artist = "Unknown Artist".to_string();
    let mut description = String::new();

    if let Ok(entries) = std::fs::read_dir(session_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.file_name().and_then(|s| s.to_str()).map_or(false, |s| s.ends_with(".info.json")) {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    if let Ok(meta) = serde_json::from_str::<SourceMetadata>(&content) {
                        if let Some(t) = meta.title { title = t; }
                        if let Some(d) = meta.description { description = d; }
                        if let Some(a) = meta.artist {
                            artist = a;
                        } else if let Some(u) = meta.uploader {
                            artist = u;
                        } else if let Some(c) = meta.channel {
                            artist = c;
                        }
                        break;
                    }
                }
            }
        }
    }

    (title, artist, description)
}
