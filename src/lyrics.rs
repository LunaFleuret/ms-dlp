use regex::Regex;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct LrclibResponse {
    #[serde(rename = "plainLyrics")]
    plain_lyrics: Option<String>,
    #[serde(rename = "syncedLyrics")]
    synced_lyrics: Option<String>,
}

pub fn parse_vtt_lyrics(vtt_path: &Path) -> Option<String> {
    if !vtt_path.is_file() {
        return None;
    }

    let content = std::fs::read_to_string(vtt_path).ok()?;
    let tag_re = Regex::new(r"<[^>]+>").ok()?;
    let invisible_re = Regex::new(r"[\u200B-\u200D\uFEFF]").ok()?;

    let mut result_lines = Vec::new();
    let mut prev_line = String::new();
    let mut in_style = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();

        if line.starts_with("STYLE") {
            in_style = true;
            continue;
        }
        if in_style {
            if line.contains('}') {
                in_style = false;
            }
            continue;
        }
        if line.starts_with('}') || line.starts_with(']') || line.starts_with(')') {
            continue;
        }

        if line.starts_with("WEBVTT")
            || line.starts_with("NOTE")
            || line.starts_with("REGION")
            || line.starts_with("Kind:")
            || line.starts_with("Language:")
            || line.starts_with("##")
            || line.starts_with("::cue")
            || line.chars().all(|c| c.is_ascii_digit())
            || (line.len() >= 5 && line.chars().take(2).all(|c| c.is_ascii_digit()) && line.contains(':'))
            || line.is_empty()
        {
            continue;
        }

        let clean = tag_re.replace_all(line, "");
        let clean = invisible_re.replace_all(&clean, "").trim().to_string();

        if !clean.is_empty() && clean != prev_line {
            prev_line = clean.clone();
            result_lines.push(clean);
        }
    }

    if !result_lines.is_empty() {
        Some(result_lines.join("\n"))
    } else {
        None
    }
}

pub async fn fetch_lyrics_from_lrclib(title: &str, artist: &str) -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) Chrome/131.0.0.0")
        .build()
        .ok()?;

    let clean_re = Regex::new(r"(?i)(【.*?】|\[.*?\]|\(.*?\)|cover|acoustic|official|mv|様)").ok()?;
    let clean_title = clean_re.replace_all(title, "").trim().to_string();
    let clean_artist = clean_re.replace_all(artist, "").trim().to_string();

    let get_url = format!(
        "https://lrclib.net/api/get?track_name={}&artist_name={}",
        urlencoding_encode(&clean_title),
        urlencoding_encode(&clean_artist)
    );

    if let Ok(resp) = client.get(&get_url).send().await {
        if resp.status().is_success() {
            if let Ok(data) = resp.json::<LrclibResponse>().await {
                if let Some(plain) = data.plain_lyrics {
                    if !plain.trim().is_empty() {
                        println!("    [Lyrics Match] Hit via '{}' by '{}'", clean_title, clean_artist);
                        return Some(plain.trim().to_string());
                    }
                }
                if let Some(synced) = data.synced_lyrics {
                    let ts_re = Regex::new(r"\[\d{2}:\d{2}\.\d{2,3}\]").ok()?;
                    let plain = ts_re.replace_all(&synced, "").trim().to_string();
                    if !plain.is_empty() {
                        println!("    [Lyrics Match] Hit via '{}' by '{}' (synced)", clean_title, clean_artist);
                        return Some(plain);
                    }
                }
            }
        }
    }

    None
}

fn urlencoding_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
