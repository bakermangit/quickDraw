use tokio::sync::mpsc::Sender;
use anyhow::Result;
use crate::types::InputEvent;

pub mod raw_input;
pub mod hook;

pub trait InputSource: Send + 'static {
    /// Start capturing input. Sends events through the provided channel.
    /// This should spawn its own thread/task and return immediately.
    fn start(&mut self, tx: Sender<InputEvent>) -> Result<()>;

    /// Stop capturing input and clean up resources.
    fn stop(&mut self) -> Result<()>;

    /// Whether this input source can block/intercept events from reaching other apps.
    /// Raw Input and polling are read-only (false). Hooks can intercept (true).
    #[allow(dead_code)]
    fn can_block(&self) -> bool;

    /// Human-readable name for logging/config (e.g., "raw_input", "hook")
    #[allow(dead_code)]
    fn name(&self) -> &str;
}
