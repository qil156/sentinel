use anyhow::{anyhow, Result};

use super::{ExclusionRect, ForegroundCapture};

// TODO(macos):
// [ ] Foreground window detection:
//     Use macOS APIs to resolve the frontmost app and focused window metadata.
// [ ] Window bounds:
//     Read absolute window coordinates for cropping and exclusion masks.
// [ ] Screen capture:
//     Capture full display image that contains the target window.
// [ ] Crop pipeline:
//     Crop full-screen capture to target window bounds.
// [ ] Exclusion masking:
//     Apply ExclusionRect regions before crop to hide Sentinel UI overlays.
// [ ] Window title extraction:
//     Populate ForegroundCapture.window_title from macOS window metadata.
// [ ] Permissions UX:
//     Handle Screen Recording permission denial with actionable error messaging.
// [ ] Multi-monitor handling:
//     Ensure correct display selection when window spans or crosses displays.
// [ ] Performance:
//     Validate capture latency is acceptable for chat round-trips.
// [ ] Tests:
//     Add unit/integration coverage for crop math and exclusion behavior.
pub fn capture_foreground_window(_exclusions: &[ExclusionRect]) -> Result<ForegroundCapture> {
    Err(anyhow!(
        "macOS capture is not implemented yet. Add the macOS platform implementation in src-tauri/src/platform/macos.rs."
    ))
}
