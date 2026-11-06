mod config;
mod cover;
mod downloader;
mod loudness;
mod lyrics;
mod native_host;
mod tagger;
mod tools;

use anyhow::Result;
use config::Config;
use std::path::{Path, PathBuf};
use tools::ToolPaths;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let current_exe = std::env::current_exe()?;
    let root_dir = current_exe.parent().unwrap_or(Path::new("."));

    // Check if running in Chrome Native Messaging Host mode
    if args.iter().any(|a| a == "--native-host" || a.starts_with("chrome-extension://")) {
        return run_native_host(root_dir).await;
    }

    // CLI mode
    run_cli(root_dir, &args[1..]).await
}

async fn run_cli(root_dir: &Path, args: &[String]) -> Result<()> {
    println!("========================================");
    println!("       Universal Music Downloader (Rust)");
    println!("    (YouTube / X / TikTok / Niconico)   ");
    println!("========================================");

    let config_path = root_dir.join("config.json");
    let config = Config::load_or_create(&config_path);

    println!(
        "Config : Format = {}, Normalize = {} ({:.1} LUFS), NonSquareCover = {}, Lyrics = {}, Threads = {}",
        config.audio_format,
        config.normalize_audio,
        config.target_loudness_lufs,
        config.cover_mode_non_square,
        config.embed_lyrics,
        config.concurrent_fragments
    );
    println!();

    println!("Checking required tools...");
    let tools = match ToolPaths::locate(root_dir, &config.bin_folder) {
        Ok(t) => {
            println!("  * yt-dlp: {:?}", t.yt_dlp);
            println!("  * ffmpeg: {:?}", t.ffmpeg);
            if let Some(ref p) = t.ffprobe { println!("  * ffprobe: {:?}", p); }
            if let Some(ref p) = t.atomic_parsley { println!("  * AtomicParsley: {:?}", p); }
            println!("Tools are ready!\n");
            t
        }
        Err(e) => {
            eprintln!("[FATAL] Tools check failed: {}", e);
            eprintln!("Please ensure yt-dlp.exe and ffmpeg.exe exist in bin folder.");
            return Ok(());
        }
    };

    let cookie_args = tools::get_cookie_args(root_dir, &config.private_folder);

    if !args.is_empty() {
        for arg in args {
            let norm_url = downloader::normalize_url(arg);
            if norm_url.starts_with("http") {
                let _ = process_single_download(root_dir, &config, &tools, &cookie_args, &norm_url).await;
            }
        }
        return Ok(());
    }

    // Interactive REPL loop
    loop {
        println!("--------------------------------------------------");
        println!("Enter Media / Music URL (or press Enter to exit):");
        print!("> ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let trimmed = input.trim();

        if trimmed.is_empty() {
            println!("Exiting. Have a great day!");
            break;
        }

        let tokens: Vec<&str> = trimmed.split_whitespace().collect();
        for token in tokens {
            let norm_url = downloader::normalize_url(token);
            if norm_url.starts_with("http") {
                println!("\nTarget: {}", norm_url);
                let _ = process_single_download(root_dir, &config, &tools, &cookie_args, &norm_url).await;
            }
        }
    }

    Ok(())
}

async fn process_single_download(
    root_dir: &Path,
    config: &Config,
    tools: &ToolPaths,
    cookie_args: &[String],
    url: &str,
) -> Result<PathBuf> {
    let session_id = format!("{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_millis());
    let session_dir = config.get_temp_dir(root_dir).join(&session_id);
    std::fs::create_dir_all(&session_dir)?;

    println!("  [1/5] Downloading audio ({}), thumbnail & metadata (-N {})...", config.audio_format, config.concurrent_fragments);

    let audio_file = match downloader::execute_ytdlp_download(
        &tools.yt_dlp,
        &tools.ffmpeg,
        &session_dir,
        url,
        &config.audio_format,
        &config.audio_quality,
        config.concurrent_fragments,
        config.embed_lyrics,
        cookie_args,
        |_pct, _speed, _eta| {},
    ).await {
        Ok(f) => f,
        Err(e) => {
            eprintln!("  [ERROR] Download failed: {}", e);
            let _ = std::fs::remove_dir_all(&session_dir);
            return Err(e);
        }
    };

    println!("  [2/5] Reading source metadata & verifying thumbnail...");
    let resolved_thumb = cover::resolve_thumbnail_and_metadata(
        &session_dir,
        url,
        &tools.yt_dlp,
        &tools.ffmpeg,
        cookie_args,
    ).await;

    let (mut title, artist, _description) = downloader::read_source_metadata(&session_dir);
    if title.is_empty() {
        title = audio_file.file_stem().and_then(|s| s.to_str()).unwrap_or("Track").to_string();
    }

    println!("    Title  : {}", title);
    println!("    Artist : {}", artist);

    // Loudness Normalization
    if config.normalize_audio {
        println!("  [Audio] Normalizing loudness (EBU R128 to {:.1} LUFS)...", config.target_loudness_lufs);
        let _ = loudness::normalize_audio_loudness(&tools.ffmpeg, &audio_file, config.target_loudness_lufs);
    }

    println!("  [3/5] Checking and formatting cover art...");
    let mut processed_cover = None;
    if let Some(thumb_path) = resolved_thumb {
        let cover_out = session_dir.join("cover_square.jpg");
        let mut mode = config.cover_mode_non_square.as_str();
        if url.contains("youtube.com/shorts/") && config.auto_crop_shorts {
            println!("    [Shorts] YouTube Shorts detected: Automatically applying 'crop' mode.");
            mode = "crop";
        }

        if let Ok(p) = cover::format_cover_art(&tools.ffmpeg, &thumb_path, &cover_out, mode) {
            processed_cover = Some(p);
        }
    } else {
        println!("    [Cover Warning] No valid thumbnail acquired; skipping cover art.");
    }

    let mut lyrics_file = None;
    if config.embed_lyrics {
        println!("  [4/5] Fetching lyrics...");
        // Check VTT
        let mut lyrics_text = None;
        if let Ok(entries) = std::fs::read_dir(&session_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|s| s.to_str()) == Some("vtt") {
                    if let Some(lyr) = lyrics::parse_vtt_lyrics(&p) {
                        println!("    [Lyrics Source] YouTube subtitle ({:?})", p.file_name().unwrap());
                        lyrics_text = Some(lyr);
                        break;
                    }
                }
            }
        }

        if lyrics_text.is_none() {
            println!("    [Lyrics Source] Searching online database (LRCLIB)...");
            lyrics_text = lyrics::fetch_lyrics_from_lrclib(&title, &artist).await;
        }

        if let Some(text) = lyrics_text {
            let lyr_path = session_dir.join("lyrics.txt");
            if std::fs::write(&lyr_path, text).is_ok() {
                lyrics_file = Some(lyr_path);
            }
        } else {
            println!("    [Lyrics Source] No lyrics found (skipping).");
        }
    }

    println!("  [5/5] Tagging audio and embedding cover & lyrics...");

    let output_dir = config.get_output_dir(root_dir);
    std::fs::create_dir_all(&output_dir)?;

    let clean_title = title.replace(['\\', '/', ':', '*', '?', '"', '<', '>', '|'], "_");
    let ext = audio_file.extension().and_then(|s| s.to_str()).unwrap_or(&config.audio_format);
    let mut final_dest = output_dir.join(format!("{}.{}", clean_title, ext));

    let mut counter = 1;
    while final_dest.exists() {
        final_dest = output_dir.join(format!("{} ({}).{}", clean_title, counter, ext));
        counter += 1;
    }

    let _ = tagger::embed_tags_and_artwork(
        tools.atomic_parsley.as_deref(),
        &tools.ffmpeg,
        &audio_file,
        processed_cover.as_deref(),
        lyrics_file.as_deref(),
        &title,
        &artist,
        url,
        &config.audio_format,
    );

    std::fs::rename(&audio_file, &final_dest)?;
    println!("  [SUCCESS] Saved: {:?}", final_dest);

    let _ = std::fs::remove_dir_all(&session_dir);
    Ok(final_dest)
}

async fn run_native_host(root_dir: &Path) -> Result<()> {
    let tasks_dir = root_dir.join("native_host").join("tasks");
    let _ = std::fs::create_dir_all(&tasks_dir);

    while let Ok(Some(msg)) = native_host::read_native_message() {
        match msg.action.as_str() {
            "ping" => {
                let resp = serde_json::json!({ "status": "success", "message": "pong" });
                let _ = native_host::write_native_message(&resp);
            }
            "get_tasks" | "get_tasks_from_popup" => {
                let tasks = native_host::load_all_tasks(&tasks_dir);
                let resp = serde_json::json!({ "status": "success", "tasks": tasks });
                let _ = native_host::write_native_message(&resp);
            }
            "open_output_dir" => {
                let config_path = root_dir.join("config.json");
                let cfg = Config::load_or_create(&config_path);
                let out_dir = cfg.get_output_dir(root_dir);
                let _ = std::fs::create_dir_all(&out_dir);

                #[cfg(windows)]
                let _ = std::process::Command::new("explorer.exe").arg(&out_dir).spawn();

                let resp = serde_json::json!({ "status": "success" });
                let _ = native_host::write_native_message(&resp);
            }
            "start_download" => {
                if let Some(target_url) = msg.url {
                    let task_id = format!("{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis());
                    let mut task_data = native_host::TaskData {
                        id: task_id.clone(),
                        url: target_url.clone(),
                        title: msg.title.unwrap_or_else(|| target_url.clone()),
                        status: "downloading".to_string(),
                        progress: 0.0,
                        percent: 0.0,
                        speed: String::new(),
                        eta: String::new(),
                        error: String::new(),
                        created_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs_f64(),
                        options: msg.options.clone(),
                    };

                    native_host::update_task_file(&tasks_dir, &task_data);

                    let resp = serde_json::json!({ "status": "success", "task_id": task_id });
                    let _ = native_host::write_native_message(&resp);

                    // Spawn background download task
                    let root_buf = root_dir.to_path_buf();
                    let tasks_buf = tasks_dir.clone();
                    tokio::spawn(async move {
                        let config_path = root_buf.join("config.json");
                        let mut cfg = Config::load_or_create(&config_path);

                        if let Some(opts) = msg.options {
                            if let Some(f) = opts.get("audio_format").and_then(|v| v.as_str()) { cfg.audio_format = f.to_string(); }
                            if let Some(m) = opts.get("cover_mode_non_square").and_then(|v| v.as_str()) { cfg.cover_mode_non_square = m.to_string(); }
                            if let Some(b) = opts.get("auto_crop_shorts").and_then(|v| v.as_bool()) { cfg.auto_crop_shorts = b; }
                            if let Some(b) = opts.get("embed_lyrics").and_then(|v| v.as_bool()) { cfg.embed_lyrics = b; }
                            if let Some(b) = opts.get("normalize_audio").and_then(|v| v.as_bool()) { cfg.normalize_audio = b; }
                        }

                        if let Ok(tools) = ToolPaths::locate(&root_buf, &cfg.bin_folder) {
                            let cookie_args = tools::get_cookie_args(&root_buf, &cfg.private_folder);
                            match process_single_download(&root_buf, &cfg, &tools, &cookie_args, &target_url).await {
                                Ok(_) => {
                                    task_data.status = "completed".to_string();
                                    task_data.progress = 100.0;
                                    task_data.percent = 100.0;
                                    task_data.eta = "00:00".to_string();
                                    native_host::update_task_file(&tasks_buf, &task_data);
                                }
                                Err(e) => {
                                    task_data.status = "error".to_string();
                                    task_data.error = e.to_string();
                                    native_host::update_task_file(&tasks_buf, &task_data);
                                }
                            }
                        } else {
                            task_data.status = "error".to_string();
                            task_data.error = "Core tools (yt-dlp/ffmpeg) not found in bin".to_string();
                            native_host::update_task_file(&tasks_buf, &task_data);
                        }
                    });
                }
            }
            _ => {
                let resp = serde_json::json!({ "status": "error", "message": "Unknown action" });
                let _ = native_host::write_native_message(&resp);
            }
        }
    }

    Ok(())
}
