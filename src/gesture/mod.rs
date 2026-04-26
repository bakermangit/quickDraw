pub mod dollar_one;
pub mod rubine;

use crate::types::{GestureCapture, GestureMatch, GestureTemplate};

pub trait GestureRecognizer: Send + 'static {
    /// Attempt to recognize a gesture from captured mouse data.
    /// Returns the best match above the confidence threshold, or None.
    fn recognize(
        &self,
        capture: &GestureCapture,
        templates: &[GestureTemplate],
    ) -> Option<GestureMatch>;

    /// Process a raw capture into a template for storage.
    /// Called during gesture recording to generate the processed form.
    fn create_template(&self, name: String, capture: &GestureCapture) -> GestureTemplate;

    /// Human-readable name (e.g., "dollar_one", "rubine")
    #[allow(dead_code)]
    fn name(&self) -> &str;
}

#[allow(dead_code)]
pub trait GestureFilter: Send + 'static {
    /// Post-recognition filter. Returns true if the gesture should be accepted.
    fn accept(&self, capture: &GestureCapture, template: &GestureTemplate) -> bool;

    /// Human-readable name
    fn name(&self) -> &str;
}
