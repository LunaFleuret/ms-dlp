use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_audio_format")]
    pub audio_format: String,

    #[serde(default = "default_audio_quality")]
    pub audio_quality: String,

    #[serde(default = "default_cover_mode")]
    pub cover_mode_non_square: String,

    #[serde(default = "default_true")]
    pub auto_crop_shorts: bool,

    #[serde(default = "default_true")]
    pub embed_lyrics: bool,

    #[serde(default = "default_fragments")]
    pub concurrent_fragments: u32,

    #[serde(default = "default_output_folder")]
    pub output_folder: String,

    #[serde(default = "default_true")]
    pub normalize_audio: bool,

    #[serde(default = "default_target_lufs")]
    pub target_loudness_lufs: f64,

    #[serde(default = "default_bin_folder")]
    pub bin_folder: String,

    #[serde(default = "default_private_folder")]
    pub private_folder: String,

    #[serde(default = "default_temp_folder")]
    pub temp_folder: String,
}

fn default_audio_format() -> String { "m4a".to_string() }
fn default_audio_quality() -> String { "0".to_string() }
fn default_cover_mode() -> String { "stretch".to_string() }
fn default_true() -> bool { true }
fn default_fragments() -> u32 { 5 }
fn default_output_folder() -> String { "output".to_string() }
fn default_target_lufs() -> f64 { -14.0 }
fn default_bin_folder() -> String { "bin".to_string() }
fn default_private_folder() -> String { "private".to_string() }
fn default_temp_folder() -> String { "temp_download".to_string() }

impl Default for Config {
    fn default() -> Self {
        Self {
            audio_format: default_audio_format(),
            audio_quality: default_audio_quality(),
            cover_mode_non_square: default_cover_mode(),
            auto_crop_shorts: true,
            embed_lyrics: true,
            concurrent_fragments: default_fragments(),
            output_folder: default_output_folder(),
            normalize_audio: true,
            target_loudness_lufs: default_target_lufs(),
            bin_folder: default_bin_folder(),
            private_folder: default_private_folder(),
            temp_folder: default_temp_folder(),
        }
    }
}

impl Config {
    pub fn load_or_create<P: AsRef<Path>>(config_path: P) -> Self {
        let path = config_path.as_ref();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                if let Ok(cfg) = serde_json::from_str::<Config>(&content) {
                    return cfg;
                }
            }
        }

        let default_cfg = Config::default();
        if let Ok(json_str) = serde_json::to_string_pretty(&default_cfg) {
            let _ = std::fs::write(path, json_str);
        }
        default_cfg
    }

    #[allow(dead_code)]
    pub fn save<P: AsRef<Path>>(&self, config_path: P) -> Result<()> {
        let json_str = serde_json::to_string_pretty(self)?;
        std::fs::write(config_path, json_str)?;
        Ok(())
    }

    pub fn get_output_dir(&self, root_dir: &Path) -> PathBuf {
        root_dir.join(&self.output_folder)
    }

    pub fn get_temp_dir(&self, root_dir: &Path) -> PathBuf {
        root_dir.join(&self.temp_folder)
    }

    #[allow(dead_code)]
    pub fn get_bin_dir(&self, root_dir: &Path) -> PathBuf {
        root_dir.join(&self.bin_folder)
    }
}
