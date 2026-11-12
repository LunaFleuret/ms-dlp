use anyhow::Result;
use std::path::Path;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn embed_tags_and_artwork(
    atomic_parsley_path: Option<&Path>,
    ffmpeg_path: &Path,
    audio_file: &Path,
    cover_image: Option<&Path>,
    lyrics_file: Option<&Path>,
    title: &str,
    artist: &str,
    comment_url: &str,
    format: &str,
    track: Option<(u32, Option<u32>)>,
) -> Result<bool> {
    if format == "m4a" {
        if let Some(ap_path) = atomic_parsley_path {
            if ap_path.is_file() {
                let mut cmd = Command::new(ap_path);
                cmd.arg(audio_file)
                    .args(["--title", title])
                    .args(["--artist", artist])
                    .args(["--comment", comment_url])
                    .arg("--overWrite");

                if let Some((num, tot_opt)) = track {
                    let track_str = match tot_opt {
                        Some(tot) => format!("{}/{}", num, tot),
                        None => format!("{}", num),
                    };
                    cmd.args(["--tracknum", &track_str]);
                }

                if let Some(cover) = cover_image {
                    if cover.is_file() {
                        cmd.args(["--artwork", &cover.to_string_lossy()]);
                    }
                }

                if let Some(lyrics) = lyrics_file {
                    if lyrics.is_file() {
                        cmd.args(["--lyricsFile", &lyrics.to_string_lossy()]);
                    }
                }

                #[cfg(windows)]
                cmd.creation_flags(CREATE_NO_WINDOW);

                match cmd.status() {
                    Ok(status) if status.success() => {
                        return Ok(true);
                    }
                    Ok(status) => {
                        println!("    [Warning] AtomicParsley exited with status: {}. Falling back to ffmpeg...", status);
                    }
                    Err(e) => {
                        println!("    [Warning] AtomicParsley failed to execute: {}. Falling back to ffmpeg...", e);
                    }
                }
            }
        }
    }

    // Fallback or non-m4a: ffmpeg
    let ext = audio_file.extension().and_then(|s| s.to_str()).unwrap_or("m4a");
    let temp_tagged = audio_file.with_extension(format!("tagged.{}", ext));

    let mut cmd = Command::new(ffmpeg_path);
    cmd.arg("-y").arg("-i").arg(audio_file);

    if let Some(cover) = cover_image {
        if cover.is_file() {
            cmd.arg("-i").arg(cover)
                .args(["-map", "0:a", "-map", "1:v"])
                .args(["-c:a", "copy", "-c:v", "mjpeg"])
                .args(["-disposition:v", "attached_pic"]);
        } else {
            cmd.args(["-c", "copy"]);
        }
    } else {
        cmd.args(["-c", "copy"]);
    }

    cmd.args([
        "-metadata", &format!("title={}", title),
        "-metadata", &format!("artist={}", artist),
        "-metadata", &format!("comment={}", comment_url),
    ]);

    if let Some((num, tot_opt)) = track {
        let track_str = match tot_opt {
            Some(tot) => format!("{}/{}", num, tot),
            None => format!("{}", num),
        };
        cmd.args(["-metadata", &format!("track={}", track_str)]);
    }

    if let Some(lyrics) = lyrics_file {
        if let Ok(lyr_content) = std::fs::read_to_string(lyrics) {
            cmd.args(["-metadata", &format!("lyrics={}", lyr_content)]);
        }
    }

    cmd.arg(&temp_tagged);

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    let status = cmd.status()?;
    if status.success() && temp_tagged.is_file() {
        if audio_file.is_file() {
            let _ = std::fs::remove_file(audio_file);
        }
        std::fs::rename(&temp_tagged, audio_file)?;
        Ok(true)
    } else {
        let _ = std::fs::remove_file(&temp_tagged);
        Ok(false)
    }
}
