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

    // Check if running in worker mode (detached background download)
    if args.len() >= 3 && args[1] == "--worker" {
        return run_worker(root_dir, &args[2]).await;
    }

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

    let cover_notice = if let Ok(val) = std::env::var("MSDLP_COVER_MODE") {
        let v = val.trim().to_lowercase();
        if v == "stretch" || v == "crop" || v == "blur" || v == "original" {
            format!("{} (Env Override)", config.cover_mode_non_square)
        } else {
            config.cover_mode_non_square.clone()
        }
    } else {
        config.cover_mode_non_square.clone()
    };

    println!(
        "Config : Format = {}, Normalize = {} ({:.1} LUFS), NonSquareCover = {}, Lyrics = {}, Threads = {}",
        config.audio_format,
        config.normalize_audio,
        config.target_loudness_lufs,
        cover_notice,
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
    process_single_download_with_progress(root_dir, config, tools, cookie_args, url, None, "", |_pct, _speed, _eta| {}).await
}

async fn process_single_download_with_progress<F>(
    root_dir: &Path,
    config: &Config,
    tools: &ToolPaths,
    cookie_args: &[String],
    url: &str,
    tasks_dir: Option<&Path>,
    task_id: &str,
    mut on_progress: F,
) -> Result<PathBuf>
where
    F: FnMut(f32, &str, &str),
{
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
        &mut on_progress,
    ).await {
        Ok(f) => f,
        Err(e) => {
            eprintln!("  [ERROR] Download failed: {}", e);
            let _ = std::fs::remove_dir_all(&session_dir);
            return Err(e);
        }
    };

    if let Some(td) = tasks_dir {
        if let Some(mut t) = native_host::load_task(td, task_id) {
            t.status = "converting".to_string();
            t.percent = 100.0;
            t.progress = 100.0;
            t.speed = String::new();
            t.eta = String::new();
            native_host::update_task_file(td, &t);
        }
    }

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

    if let Some(td) = tasks_dir {
        if let Some(mut t) = native_host::load_task(td, task_id) {
            t.title = if artist != "Unknown Artist" && !artist.is_empty() {
                format!("{} - {}", artist, title)
            } else {
                title.clone()
            };
            native_host::update_task_file(td, &t);
        }
    }

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

        let is_youtube = url.contains("youtube.com") || url.contains("youtu.be");
        if let Ok(p) = cover::format_cover_art(&tools.ffmpeg, &thumb_path, &cover_out, mode, is_youtube) {
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

    let track_num: Option<u32> = std::env::var("MSDLP_TRACK_NUM").ok().and_then(|v| v.trim().parse().ok());
    let track_total: Option<u32> = std::env::var("MSDLP_TRACK_TOTAL").ok().and_then(|v| v.trim().parse().ok());

    let file_stem = if let Some(num) = track_num {
        let width = track_total.map(|t| t.to_string().len().max(2)).unwrap_or(2);
        format!("{:0width$}_{}", num, clean_title, width = width)
    } else {
        clean_title.clone()
    };

    let mut final_dest = output_dir.join(format!("{}.{}", file_stem, ext));

    let mut counter = 1;
    while final_dest.exists() {
        final_dest = output_dir.join(format!("{} ({}).{}", file_stem, counter, ext));
        counter += 1;
    }

    let track_info = track_num.map(|n| (n, track_total));
    if let Some((n, tot_opt)) = track_info {
        if let Some(tot) = tot_opt {
            println!("    Track  : {}/{}", n, tot);
        } else {
            println!("    Track  : {}", n);
        }
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
        track_info,
    );

    std::fs::rename(&audio_file, &final_dest)?;
    println!("  [SUCCESS] Saved: {:?}", final_dest);

    let _ = std::fs::remove_dir_all(&session_dir);
    Ok(final_dest)
}

async fn run_worker(root_dir: &Path, task_id: &str) -> Result<()> {
    let tasks_dir = root_dir.join("native_host").join("tasks");
    let mut task_data = match native_host::load_task(&tasks_dir, task_id) {
        Some(t) => t,
        None => return Ok(()),
    };

    let config_path = root_dir.join("config.json");
    let mut cfg = Config::load_or_create(&config_path);

    if let Some(ref opts) = task_data.options {
        if let Some(f) = opts.get("audio_format").and_then(|v| v.as_str()) { cfg.audio_format = f.to_string(); }
        if let Some(m) = opts.get("cover_mode_non_square").and_then(|v| v.as_str()) { cfg.cover_mode_non_square = m.to_string(); }
        if let Some(b) = opts.get("auto_crop_shorts").and_then(|v| v.as_bool()) { cfg.auto_crop_shorts = b; }
        if let Some(b) = opts.get("embed_lyrics").and_then(|v| v.as_bool()) { cfg.embed_lyrics = b; }
        if let Some(b) = opts.get("normalize_audio").and_then(|v| v.as_bool()) { cfg.normalize_audio = b; }
        if let Some(c) = opts.get("concurrent_fragments").and_then(|v| v.as_u64()) { cfg.concurrent_fragments = c as u32; }
    }

    let tools = match ToolPaths::locate(root_dir, &cfg.bin_folder) {
        Ok(t) => t,
        Err(e) => {
            task_data.status = "error".to_string();
            task_data.error = format!("Core tools not found in bin: {}", e);
            native_host::update_task_file(&tasks_dir, &task_data);
            return Ok(());
        }
    };

    let cookie_args = tools::get_cookie_args(root_dir, &cfg.private_folder);

    let tasks_dir_clone = tasks_dir.clone();
    let mut last_update = std::time::Instant::now();
    let mut current_task = task_data.clone();

    let on_progress = move |pct: f32, speed: &str, eta: &str| {
        if last_update.elapsed().as_millis() >= 350 || pct >= 99.0 {
            current_task.percent = pct;
            current_task.progress = pct;
            current_task.speed = speed.to_string();
            current_task.eta = eta.to_string();
            native_host::update_task_file(&tasks_dir_clone, &current_task);
            last_update = std::time::Instant::now();
        }
    };

    match process_single_download_with_progress(
        root_dir,
        &cfg,
        &tools,
        &cookie_args,
        &task_data.url,
        Some(&tasks_dir),
        task_id,
        on_progress,
    ).await {
        Ok(_) => {
            if let Some(mut completed_task) = native_host::load_task(&tasks_dir, task_id) {
                completed_task.status = "completed".to_string();
                completed_task.progress = 100.0;
                completed_task.percent = 100.0;
                completed_task.speed = String::new();
                completed_task.eta = "00:00".to_string();
                native_host::update_task_file(&tasks_dir, &completed_task);
            }
        }
        Err(e) => {
            if let Some(mut err_task) = native_host::load_task(&tasks_dir, task_id) {
                err_task.status = "error".to_string();
                err_task.error = e.to_string();
                native_host::update_task_file(&tasks_dir, &err_task);
            }
        }
    }

    Ok(())
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
            "clear_finished" | "clear_finished_tasks" => {
                let deleted = native_host::clear_finished_tasks(&tasks_dir);
                let resp = serde_json::json!({ "status": "success", "deleted": deleted });
                let _ = native_host::write_native_message(&resp);
            }
            "cancel_task" => {
                let tid = msg.task_id.as_deref().or(msg.id.as_deref()).unwrap_or("");
                let ok = native_host::cancel_task(&tasks_dir, tid);
                let resp = serde_json::json!({ "status": if ok { "success" } else { "error" } });
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
                    let task_data = native_host::TaskData {
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

                    // Spawn background download worker as an independent detached process
                    let current_exe = std::env::current_exe().unwrap_or_else(|_| root_dir.join("ms-dlp.exe"));
                    let mut cmd = std::process::Command::new(&current_exe);
                    cmd.arg("--worker").arg(&task_id);

                    #[cfg(windows)]
                    {
                        use std::os::windows::process::CommandExt;
                        // 0x08000000 = CREATE_NO_WINDOW
                        // 0x00000008 = DETACHED_PROCESS
                        cmd.creation_flags(0x08000008);
                    }

                    cmd.stdin(std::process::Stdio::null());
                    cmd.stdout(std::process::Stdio::null());
                    cmd.stderr(std::process::Stdio::null());

                    let _ = cmd.spawn();

                    let resp = serde_json::json!({ "status": "success", "task_id": task_id });
                    let _ = native_host::write_native_message(&resp);
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
