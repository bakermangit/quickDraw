use anyhow::Result;
use crate::config::ActionConfig;

pub mod keyboard;

pub trait OutputAction: Send + 'static {
    /// Execute the action.
    fn execute(&self) -> Result<()>;

    /// Human-readable name for logging
    #[allow(dead_code)]
    fn name(&self) -> &str;
}

pub fn create_action(config: &ActionConfig) -> Result<Box<dyn OutputAction>> {
    match config {
        ActionConfig::KeyPress { key, modifiers } => {
            let vk = keyboard::parse_virtual_key(&key.0)?;
            let mut mods = Vec::new();
            for m in modifiers {
                mods.push(keyboard::parse_virtual_key(&m.0)?);
            }
            Ok(Box::new(keyboard::KeyPressAction { key: vk, modifiers: mods }))
        }
    }
}
