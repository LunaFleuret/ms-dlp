use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ToolPaths {
    pub yt_dlp: PathBuf,
    pub ffmpeg: PathBuf,
    pub ffprobe: Option<PathBuf>,
    pub atomic_parsley: Option<PathBuf>,
}

impl ToolPaths {
    pub fn locate(root_dir: &Path, bin_folder: &str) -> Result<Self> {
        let bin_dir = root_dir.join(bin_folder);

        let yt_dlp = find_binary(&bin_dir, "yt-dlp")
            .ok_or_else(|| anyhow!("yt-dlp.exe not found in {:?} or PATH", bin_dir))?;

        let ffmpeg = find_binary(&bin_dir, "ffmpeg")
            .ok_or_else(|| anyhow!("ffmpeg.exe not found in {:?} or PATH", bin_dir))?;

        let ffprobe = find_binary(&bin_dir, "ffprobe");
        let atomic_parsley = find_binary(&bin_dir, "AtomicParsley");

        Ok(Self {
            yt_dlp,
            ffmpeg,
            ffprobe,
            atomic_parsley,
        })
    }
}

fn find_binary(bin_dir: &Path, name: &str) -> Option<PathBuf> {
    let exe_name = if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };

    let local_path = bin_dir.join(&exe_name);
    if local_path.is_file() {
        return Some(local_path);
    }

    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(&exe_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn get_cookie_args(root_dir: &Path, private_folder: &str) -> Vec<String> {
    let priv_dir = root_dir.join(private_folder);
    if !priv_dir.is_dir() {
        return Vec::new();
    }

    let yt_cookie = priv_dir.join("www.youtube.com_cookies.txt");
    if yt_cookie.is_file() {
        return vec!["--cookies".to_string(), yt_cookie.to_string_lossy().to_string()];
    }

    let global_cookie = priv_dir.join("cookies.txt");
    if global_cookie.is_file() {
        return vec!["--cookies".to_string(), global_cookie.to_string_lossy().to_string()];
    }

    Vec::new()
}
