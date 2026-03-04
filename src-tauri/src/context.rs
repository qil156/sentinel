use anyhow::Result;

use crate::capture::{capture_foreground_window, ExclusionRect};
use crate::types::ScreenContext;

pub fn build_screen_context(exclusions: &[ExclusionRect]) -> Result<ScreenContext> {
    let capture = capture_foreground_window(exclusions)?;

    Ok(ScreenContext {
        window_title: capture.window_title,
        image_base64: capture.image_base64,
    })
}
