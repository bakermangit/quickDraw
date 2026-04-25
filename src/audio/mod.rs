use std::path::PathBuf;
use windows::Win32::Media::Audio::{PlaySoundW, SND_FILENAME, SND_ASYNC};
use windows::Win32::Media::Multimedia::mciSendStringW;
use windows::core::HSTRING;
use crate::config::{AudioConfig, get_config_dir};

pub struct AudioPlayer {
    config: AudioConfig,
}

impl AudioPlayer {
    pub fn new(config: AudioConfig) -> Self {
        Self { config }
    }

    pub fn play_success(&self, override_path: Option<&str>) {
        if !self.config.enabled {
            return;
        }

        let path_str = override_path.unwrap_or(&self.config.success);
        self.play_file(path_str);
    }

    pub fn play_error(&self) {
        if !self.config.enabled {
            return;
        }

        self.play_file(&self.config.error);
    }

    fn play_file(&self, path_str: &str) {
        let path = PathBuf::from(path_str);
        let absolute_path = if path.is_absolute() {
            path
        } else if let Ok(config_dir) = get_config_dir() {
            config_dir.join(path)
        } else {
            path
        };

        if !absolute_path.exists() {
            tracing::debug!("Audio file not found: {}", absolute_path.display());
            return;
        }

        let extension = absolute_path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        if extension == "wav" {
            let wide = HSTRING::from(absolute_path.as_os_str());
            unsafe {
                let _ = PlaySoundW(&wide, None, SND_FILENAME | SND_ASYNC);
            }
        } else {
            // Use MCI for MP3 and other formats
            self.play_mci(&absolute_path);
        }
    }

    fn play_mci(&self, path: &std::path::Path) {
        let path_str = path.to_string_lossy();
        // MCI commands need to handle spaces in paths. Quoting the path is the standard way.
        // We use aliases to manage the sound. 
        // For simple fire-and-forget, we close the alias before opening it again.
        let open_cmd = HSTRING::from(format!("open \"{}\" alias qdsound", path_str));
        let play_cmd = HSTRING::from("play qdsound from 0");
        let close_cmd = HSTRING::from("close qdsound");

        unsafe {
            // Close any previous instance first
            let _ = mciSendStringW(&close_cmd, None, None);
            if mciSendStringW(&open_cmd, None, None) == 0 {
                let _ = mciSendStringW(&play_cmd, None, None);
            } else {
                tracing::error!("MCI failed to open audio file: {}", path_str);
            }
        }
    }
}
