use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn is_valid_image(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() < 1024 {
            return false;
        }
    } else {
        return false;
    }

    if let Ok(reader) = image::ImageReader::open(path) {
        if let Ok(dimensions) = reader.into_dimensions() {
            return dimensions.0 > 0 && dimensions.1 > 0;
        }
    }

    false
}

pub fn get_image_dimensions(path: &Path) -> Option<(u32, u32)> {
    if let Ok(reader) = image::ImageReader::open(path) {
        if let Ok(dims) = reader.into_dimensions() {
            return Some(dims);
        }
    }
    None
}

pub async fn download_youtube_cdn_thumbnail(
    url: &str,
    destination: &Path,
) -> Option<PathBuf> {
    let re = Regex::new(r"(?:youtube\.com/(?:watch\?.*v=|shorts/)|youtu\.be/)([a-zA-Z0-9_-]{11})").ok()?;
    let video_id = re.captures(url)?.get(1)?.as_str();

    let qualities = ["maxresdefault", "sddefault", "hqdefault"];
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(6))
        .build()
        .ok()?;

    for q in &qualities {
        let cdn_url = format!("https://i.ytimg.com/vi/{}/{}.jpg", video_id, q);
        println!("    -> Attempting YouTube CDN thumbnail: {}.jpg...", q);

        if let Ok(resp) = client.get(&cdn_url).send().await {
            if resp.status().is_success() {
                if let Ok(bytes) = resp.bytes().await {
                    if let Ok(_) = std::fs::write(destination, &bytes) {
                        if is_valid_image(destination) {
                            println!("    [OK] YouTube CDN thumbnail acquired ({}.jpg)", q);
                            return Some(destination.to_path_buf());
                        }
                    }
                }
            }
        }
    }

    let _ = std::fs::remove_file(destination);
    None
}

pub async fn resolve_thumbnail_and_metadata(
    session_dir: &Path,
    url: &str,
    yt_dlp_path: &Path,
    ffmpeg_path: &Path,
    cookie_args: &[String],
) -> Option<PathBuf> {
    // 1. Check existing thumbnail
    if let Ok(entries) = std::fs::read_dir(session_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                let ext_lower = ext.to_lowercase();
                if ["jpg", "jpeg", "png", "webp"].contains(&ext_lower.as_str()) {
                    if is_valid_image(&path) {
                        return Some(path);
                    }
                }
            }
        }
    }

    println!("    [Cover Alert] Thumbnail missing or incomplete. Initiating recovery...");

    // 2. YouTube CDN direct download
    if url.contains("youtube.com") || url.contains("youtu.be") {
        let cdn_target = session_dir.join("cdn_thumbnail.jpg");
        if let Some(path) = download_youtube_cdn_thumbnail(url, &cdn_target).await {
            return Some(path);
        }
    }

    // 3. Client rotation retry via yt-dlp
    let clients = ["android", "ios", "tv", "web", "mweb"];
    let file_template = session_dir.join("%(title)s.%(ext)s").to_string_lossy().to_string();

    for (idx, client) in clients.iter().enumerate() {
        let attempt = idx + 1;
        println!("    -> [Retry {}/{}] Fetching thumbnail with client '{}'...", attempt, clients.len(), client);

        let mut cmd = Command::new(yt_dlp_path);
        cmd.args([
            "--skip-download",
            "--write-thumbnail",
            "--convert-thumbnails", "jpg",
            "--write-info-json",
            "--no-playlist",
            "--ffmpeg-location", &ffmpeg_path.to_string_lossy(),
            "--extractor-args", &format!("youtube:player_client={}", client),
            "-o", &file_template,
            url,
        ]);
        for c in cookie_args {
            cmd.arg(c);
        }

        let _ = cmd.output();

        if let Ok(entries) = std::fs::read_dir(session_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    let ext_lower = ext.to_lowercase();
                    if ["jpg", "jpeg", "png", "webp"].contains(&ext_lower.as_str()) {
                        if is_valid_image(&path) {
                            println!("    [OK] Thumbnail recovered via client '{}' on retry {}.", client, attempt);
                            return Some(path);
                        }
                    }
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }

    println!("    [ERROR] Failed to acquire thumbnail after all retries.");
    None
}

pub fn format_cover_art(
    ffmpeg_path: &Path,
    input_image: &Path,
    output_image: &Path,
    mode: &str,
) -> Result<PathBuf> {
    if let Some((w, h)) = get_image_dimensions(input_image) {
        println!("    [Cover Info] Dimension: {}x{}", w, h);
        if w == h {
            println!("    [Cover Action] Image is already 1:1 square! Using original without re-encoding.");
            return Ok(input_image.to_path_buf());
        }
    }

    if mode == "original" {
        println!("    [Cover Action] Keeping original non-square thumbnail.");
        return Ok(input_image.to_path_buf());
    }

    let is_complex;
    let filter = match mode {
        "crop" => {
            println!("    [Cover Action] Cropping sides to 1:1 square.");
            is_complex = false;
            "crop='min(iw,ih)':'min(iw,ih)'".to_string()
        }
        "blur" => {
            println!("    [Cover Action] Formatting on blurred background.");
            is_complex = true;
            "[0:v]scale='max(iw,ih)':'max(iw,ih)',boxblur=40:5[bg];[bg][0:v]overlay=(main_w-overlay_w)/2:(main_h-overlay_h)/2".to_string()
        }
        _ => {
            println!("    [Cover Action] Compressing/scaling to 1:1 square (stretch).");
            is_complex = false;
            "scale='max(iw,ih)':'max(iw,ih)'".to_string()
        }
    };

    let mut cmd = Command::new(ffmpeg_path);
    cmd.arg("-y").arg("-i").arg(input_image);
    if is_complex {
        cmd.arg("-filter_complex").arg(&filter);
    } else {
        cmd.arg("-vf").arg(&filter);
    }
    cmd.args(["-q:v", "2"]).arg(output_image);

    let status = cmd.status()?;
    if status.success() && output_image.is_file() {
        Ok(output_image.to_path_buf())
    } else {
        println!("    [Cover Warning] Processing failed, using original.");
        Ok(input_image.to_path_buf())
    }
}
