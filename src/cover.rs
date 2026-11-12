use anyhow::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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

        #[cfg(windows)]
        cmd.creation_flags(CREATE_NO_WINDOW);

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

fn row_brightness_stats(img: &image::RgbImage, y: u32) -> (u8, f32) {
    let w = img.width();
    let mut max_rgb: u8 = 0;
    let mut sum: u64 = 0;
    for x in 0..w {
        let p = img.get_pixel(x, y);
        let m = p[0].max(p[1]).max(p[2]);
        if m > max_rgb {
            max_rgb = m;
        }
        sum += m as u64;
    }
    (max_rgb, sum as f32 / w as f32)
}

fn col_brightness_stats(img: &image::RgbImage, x: u32) -> (u8, f32) {
    let h = img.height();
    let mut max_rgb: u8 = 0;
    let mut sum: u64 = 0;
    for y in 0..h {
        let p = img.get_pixel(x, y);
        let m = p[0].max(p[1]).max(p[2]);
        if m > max_rgb {
            max_rgb = m;
        }
        sum += m as u64;
    }
    (max_rgb, sum as f32 / h as f32)
}

fn scan_black_bar(count: u32, reverse: bool, line_stats: &impl Fn(u32) -> (u8, f32)) -> u32 {
    if count == 0 {
        return 0;
    }

    let idx_at = |offset: u32| -> u32 {
        if reverse { count - 1 - offset } else { offset }
    };

    // Outermost edge must be nearly pure black to prevent cropping dark artwork
    let (edge_max, edge_avg) = line_stats(idx_at(0));
    if edge_max > 16 || edge_avg > 5.0 {
        return 0;
    }

    let mut bar_len: u32 = 0;
    for offset in 0..count {
        let (line_max, line_avg) = line_stats(idx_at(offset));
        if line_max <= 32 && line_avg <= 12.0 {
            bar_len = offset + 1;
        } else {
            break;
        }
    }

    // Step past at most 1px of JPEG/WebP ringing directly bordering bright content
    if bar_len > 0 && bar_len + 1 < count {
        let (boundary_max, boundary_avg) = line_stats(idx_at(bar_len));
        let (_, next_avg) = line_stats(idx_at(bar_len + 1));
        if boundary_max <= 75 && boundary_avg <= 26.0 && next_avg >= 40.0 {
            bar_len += 1;
        }
    }

    bar_len
}

fn symmetrical_pair(first: u32, second: u32, axis_len: u32) -> (u32, u32) {
    let min_bar = (axis_len / 20).max(8);
    if first >= min_bar && second >= min_bar && first.abs_diff(second) <= 6 {
        (first, second)
    } else {
        (0, 0)
    }
}

pub fn detect_youtube_black_bars_from_rgb(img: &image::RgbImage) -> Option<(u32, u32, u32, u32)> {
    let (w, h) = (img.width(), img.height());
    if w < 120 || h < 120 {
        return None;
    }

    let top_raw = scan_black_bar(h, false, &|y| row_brightness_stats(img, y));
    let bottom_raw = scan_black_bar(h, true, &|y| row_brightness_stats(img, y));
    let (top, bottom) = symmetrical_pair(top_raw, bottom_raw, h);

    let left_raw = scan_black_bar(w, false, &|x| col_brightness_stats(img, x));
    let right_raw = scan_black_bar(w, true, &|x| col_brightness_stats(img, x));
    let (left, right) = symmetrical_pair(left_raw, right_raw, w);

    if top == 0 && bottom == 0 && left == 0 && right == 0 {
        return None;
    }

    let cropped_w = w.checked_sub(left + right)?;
    let cropped_h = h.checked_sub(top + bottom)?;

    // Ensure a substantial image remains (avoid collapsing mostly-black images)
    if cropped_w < 120 || cropped_h < 120 || cropped_w * 10 < w * 4 || cropped_h * 10 < h * 4 {
        return None;
    }

    Some((left, top, cropped_w, cropped_h))
}

pub fn detect_youtube_black_bars(path: &Path) -> Option<(u32, u32, u32, u32)> {
    let img = image::ImageReader::open(path).ok()?.decode().ok()?.to_rgb8();
    detect_youtube_black_bars_from_rgb(&img)
}

pub fn format_cover_art(
    ffmpeg_path: &Path,
    input_image: &Path,
    output_image: &Path,
    mode: &str,
    is_youtube: bool,
) -> Result<PathBuf> {
    let orig_dims = get_image_dimensions(input_image);
    if let Some((w, h)) = orig_dims {
        println!("    [Cover Info] Dimension: {}x{}", w, h);
    }

    let crop_rect = if is_youtube {
        detect_youtube_black_bars(input_image)
    } else {
        None
    };

    if let (Some((ow, oh)), Some((cx, cy, cw, ch))) = (orig_dims, crop_rect) {
        println!(
            "    [Cover Action] YouTube black bars detected ({}x{} -> {}x{} at +{},+{}). Removing...",
            ow, oh, cw, ch, cx, cy
        );
    }

    let effective_dims = crop_rect.map(|(_, _, cw, ch)| (cw, ch)).or(orig_dims);
    let crop_prefix = crop_rect.map(|(x, y, w, h)| format!("crop={}:{}:{}:{}", w, h, x, y));

    let is_complex;
    let filter = if effective_dims.map_or(false, |(w, h)| w == h) {
        if let Some(cp) = crop_prefix {
            println!("    [Cover Action] Cropped image is already 1:1 square!");
            is_complex = false;
            cp
        } else {
            println!("    [Cover Action] Image is already 1:1 square! Using original without re-encoding.");
            return Ok(input_image.to_path_buf());
        }
    } else if mode == "original" {
        if let Some(cp) = crop_prefix {
            println!("    [Cover Action] Keeping original aspect ratio (black bars removed).");
            is_complex = false;
            cp
        } else {
            println!("    [Cover Action] Keeping original non-square thumbnail.");
            return Ok(input_image.to_path_buf());
        }
    } else {
        match mode {
            "crop" => {
                println!("    [Cover Action] Cropping sides to 1:1 square.");
                is_complex = false;
                match crop_prefix {
                    Some(cp) => format!("{},crop='min(iw,ih)':'min(iw,ih)'", cp),
                    None => "crop='min(iw,ih)':'min(iw,ih)'".to_string(),
                }
            }
            "blur" => {
                println!("    [Cover Action] Formatting on blurred background.");
                is_complex = true;
                match crop_prefix {
                    Some(cp) => format!(
                        "[0:v]{}[cropped];[cropped]split[fg][bg_src];[bg_src]scale='max(iw,ih)':'max(iw,ih)',boxblur=40:5[bg];[bg][fg]overlay=(main_w-overlay_w)/2:(main_h-overlay_h)/2",
                        cp
                    ),
                    None => "[0:v]scale='max(iw,ih)':'max(iw,ih)',boxblur=40:5[bg];[bg][0:v]overlay=(main_w-overlay_w)/2:(main_h-overlay_h)/2".to_string(),
                }
            }
            _ => {
                println!("    [Cover Action] Compressing/scaling to 1:1 square (stretch).");
                is_complex = false;
                match crop_prefix {
                    Some(cp) => format!("{},scale='max(iw,ih)':'max(iw,ih)'", cp),
                    None => "scale='max(iw,ih)':'max(iw,ih)'".to_string(),
                }
            }
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

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let status = cmd.status()?;
    if status.success() && output_image.is_file() {
        Ok(output_image.to_path_buf())
    } else {
        println!("    [Cover Warning] Processing failed, using original.");
        Ok(input_image.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    #[test]
    fn detects_symmetrical_letterbox_with_ringing() {
        let mut img = RgbImage::from_pixel(640, 480, Rgb([200, 120, 80]));
        for y in 0..60 {
            for x in 0..640 {
                let v = if y == 59 { 18 } else if y >= 55 { 4 } else { 0 };
                img.put_pixel(x, y, Rgb([v, v, v]));
            }
        }
        for y in 420..480 {
            for x in 0..640 {
                let v = if y == 420 { 18 } else if y <= 424 { 4 } else { 0 };
                img.put_pixel(x, y, Rgb([v, v, v]));
            }
        }
        assert_eq!(detect_youtube_black_bars_from_rgb(&img), Some((0, 60, 640, 360)));
    }

    #[test]
    fn detects_symmetrical_pillarbox() {
        let mut img = RgbImage::from_pixel(1280, 720, Rgb([100, 180, 220]));
        for y in 0..720 {
            for x in 0..280 {
                img.put_pixel(x, y, Rgb([0, 0, 0]));
            }
            for x in 1000..1280 {
                img.put_pixel(x, y, Rgb([0, 0, 0]));
            }
        }
        assert_eq!(detect_youtube_black_bars_from_rgb(&img), Some((280, 0, 720, 720)));
    }

    #[test]
    fn ignores_asymmetric_or_dark_artwork() {
        // Top-only black sky (asymmetric) must not be cropped
        let mut top_only = RgbImage::from_pixel(640, 480, Rgb([180, 180, 180]));
        for y in 0..80 {
            for x in 0..640 {
                top_only.put_pixel(x, y, Rgb([0, 0, 0]));
            }
        }
        assert_eq!(detect_youtube_black_bars_from_rgb(&top_only), None);

        // Dark blue night scene (not pure black) must not be cropped
        let night = RgbImage::from_pixel(640, 480, Rgb([10, 15, 35]));
        assert_eq!(detect_youtube_black_bars_from_rgb(&night), None);

        // Black background with a bright star near top must not be cropped
        let mut starry = RgbImage::from_pixel(640, 480, Rgb([120, 120, 120]));
        for y in 0..60 {
            for x in 0..640 {
                starry.put_pixel(x, y, Rgb([0, 0, 0]));
                starry.put_pixel(x, 479 - y, Rgb([0, 0, 0]));
            }
        }
        starry.put_pixel(320, 20, Rgb([255, 255, 255]));
        assert_eq!(detect_youtube_black_bars_from_rgb(&starry), None);
    }

    #[test]
    fn ignores_all_black_image() {
        let all_black = RgbImage::from_pixel(640, 480, Rgb([0, 0, 0]));
        assert_eq!(detect_youtube_black_bars_from_rgb(&all_black), None);
    }
}
