#[derive(Debug, Clone)]
pub struct ForegroundCapture {
    pub window_title: String,
    pub image_base64: String,
}

#[derive(Debug, Clone, Copy)]
pub struct ExclusionRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::capture_foreground_window;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::capture_foreground_window;

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
use anyhow::{anyhow, Result};

#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub fn capture_foreground_window(_exclusions: &[ExclusionRect]) -> Result<ForegroundCapture> {
    Err(anyhow!("Screen capture is currently only implemented for Windows."))
}
