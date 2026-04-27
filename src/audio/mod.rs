use std::path::PathBuf;
#[cfg(windows)]
use windows::Win32::Media::Multimedia::mciSendStringW;
#[cfg(windows)]
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

        // Use MCI for all playback to support volume control consistently
        self.play_mci(&absolute_path);
    }

    fn play_mci(&self, _path: &std::path::Path) {
        #[cfg(windows)]
        {
            let path_str = _path.to_string_lossy();
            // MCI volume is 0-1000. Note: some drivers might use 0-100 but mpegvideo is 0-1000.
            let volume = (self.config.volume * 1000.0).clamp(0.0, 1000.0) as u32;

            // Use 'mpegvideo' type (DirectShow) as it provides better volume control.
            // We try to open it with 'type mpegvideo'. If that fails, we let MCI decide.
            let open_cmd_forced = HSTRING::from(format!("open \"{}\" type mpegvideo alias qdsound", path_str));
            let open_cmd_auto = HSTRING::from(format!("open \"{}\" alias qdsound", path_str));
            let volume_cmd = HSTRING::from(format!("setaudio qdsound volume to {}", volume));
            let volume_cmd_alt = HSTRING::from(format!("set qdsound audio volume to {}", volume));
            let play_cmd = HSTRING::from("play qdsound from 0");
            let close_cmd = HSTRING::from("close qdsound");

            unsafe {
                // Close any previous instance first to free the alias
                let _ = mciSendStringW(&close_cmd, None, None);

                let mut res = mciSendStringW(&open_cmd_forced, None, None);
                if res != 0 {
                    // Fallback to auto-type if forced mpegvideo fails
                    res = mciSendStringW(&open_cmd_auto, None, None);
                }

                if res == 0 {
                    // Try multiple MCI volume command variants to maximize compatibility across drivers
                    let _ = mciSendStringW(&volume_cmd, None, None);
                    let _ = mciSendStringW(&volume_cmd_alt, None, None);

                    let play_res = mciSendStringW(&play_cmd, None, None);
                    if play_res != 0 {
                        tracing::error!("MCI failed to play audio file (error {}): {}", play_res, path_str);
                    }
                } else {
                    tracing::error!("MCI failed to open audio file (error {}): {}", res, path_str);
                }
            }
        }
    }
}
