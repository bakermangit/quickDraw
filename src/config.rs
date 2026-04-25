use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::types::{GestureCapture, VirtualKey};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub trigger: TriggerConfig,
    pub audio: AudioConfig,
    pub logging: LoggingConfig,
    pub server: ServerConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                input_method: "raw_input".to_string(),
                recognizer: "dollar_one".to_string(),
                confidence_threshold: 0.80,
                gesture_profile: "default".to_string(),
                cursor_reset: true,
            },
            trigger: TriggerConfig::Combo {
                key1: "Mouse1".to_string(),
                key2: "Mouse2".to_string(),
            },
            audio: AudioConfig {
                enabled: true,
                success: "sounds/success.wav".to_string(),
                error: "sounds/error.wav".to_string(),
            },
            logging: LoggingConfig {
                level: "warn".to_string(),
            },
            server: ServerConfig {
                port: 9876,
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub input_method: String,
    pub recognizer: String,
    pub confidence_threshold: f64,
    pub gesture_profile: String,
    pub cursor_reset: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerConfig {
    #[serde(alias = "button_combo", rename = "combo")]
    Combo {
        #[serde(alias = "first")]
        key1: String,
        #[serde(alias = "second")]
        key2: String,
    },
    #[serde(alias = "single_button", alias = "keyboard_modifier", rename = "single")]
    Single {
        #[serde(alias = "key", alias = "button")]
        key1: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub enabled: bool,
    pub success: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureProfile {
    pub gestures: Vec<GestureConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GestureConfig {
    pub name: String,
    pub action: ActionConfig,
    pub sound: Option<String>,
    pub pattern: GesturePatternConfig,
    pub raw: GestureCapture,
    pub confidence_threshold: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActionConfig {
    KeyPress {
        key: VirtualKey,
        #[serde(default)]
        modifiers: Vec<VirtualKey>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GesturePatternConfig {
    pub algorithm: String,
    pub template_points: Vec<[f64; 2]>,
}

pub fn get_config_dir() -> Result<PathBuf> {
    // Check for portable mode first: config.toml next to the executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let portable_config = exe_dir.join("config.toml");
            if portable_config.exists() {
                let path = exe_dir.to_path_buf();
                tracing::debug!("Config: portable mode ({})", path.display());
                return Ok(path);
            }
        }
    }

    // Fall back to standard AppData directory
    let appdata = std::env::var("APPDATA").context("APPDATA environment variable not found")?;
    let mut path = PathBuf::from(appdata);
    path.push("QuickDraw");
    tracing::debug!("Config: AppData mode ({})", path.display());
    Ok(path)
}

pub fn load_config() -> Result<Config> {
    let config_dir = get_config_dir()?;
    let config_path = config_dir.join("config.toml");

    if !config_path.exists() {
        let default_config = Config::default();
        std::fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
        let toml_str =
            toml::to_string_pretty(&default_config).context("Failed to serialize default config")?;
        std::fs::write(&config_path, toml_str).context("Failed to write default config")?;
        return Ok(default_config);
    }

    let toml_str = std::fs::read_to_string(&config_path).context("Failed to read config file")?;
    let config: Config = toml::from_str(&toml_str).context("Failed to parse config file")?;
    Ok(config)
}

pub fn load_gesture_profile(name: &str) -> Result<GestureProfile> {
    let config_dir = get_config_dir()?;
    let profile_path = config_dir.join("gestures").join(format!("{}.toml", name));

    if !profile_path.exists() {
        return Ok(GestureProfile { gestures: Vec::new() });
    }

    let toml_str = std::fs::read_to_string(&profile_path)
        .with_context(|| format!("Failed to read gesture profile: {}", profile_path.display()))?;
    let profile: GestureProfile =
        toml::from_str(&toml_str).context("Failed to parse gesture profile")?;
    Ok(profile)
}

pub fn save_gesture_profile(name: &str, profile: &GestureProfile) -> Result<()> {
    let config_dir = get_config_dir()?;
    let gestures_dir = config_dir.join("gestures");
    if !gestures_dir.exists() {
        std::fs::create_dir_all(&gestures_dir).context("Failed to create gestures directory")?;
    }
    let profile_path = gestures_dir.join(format!("{}.toml", name));

    let toml_str = toml::to_string_pretty(profile).context("Failed to serialize gesture profile")?;
    std::fs::write(&profile_path, toml_str).context("Failed to write gesture profile")?;
    Ok(())
}

pub fn parse_action_str(action_str: &str) -> Result<ActionConfig> {
    if let Some(key_part) = action_str.strip_prefix("key:") {
        let parts: Vec<&str> = key_part.split('+').collect();
        if parts.is_empty() {
            return Err(anyhow::anyhow!("Invalid action string: {}", action_str));
        }

        let key = VirtualKey(parts[0].to_string());
        let mut modifiers = Vec::new();
        for &m in &parts[1..] {
            modifiers.push(VirtualKey(m.to_string()));
        }

        Ok(ActionConfig::KeyPress { key, modifiers })
    } else {
        Err(anyhow::anyhow!("Unsupported action type in: {}", action_str))
    }
}
