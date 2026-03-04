use anyhow::{anyhow, Context, Result};
use base64::Engine;
use image::{codecs::png::PngEncoder, ColorType, ImageEncoder, RgbaImage};
use screenshots::Screen;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindow, GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId,
    IsIconic, IsWindowVisible, GW_HWNDNEXT,
};

use super::{ExclusionRect, ForegroundCapture};

pub fn capture_foreground_window(exclusions: &[ExclusionRect]) -> Result<ForegroundCapture> {
    let hwnd = resolve_capture_window()?;

    let rect = get_window_rect(hwnd)?;
    let window_title = get_window_title(hwnd)?;
    let image_base64 = capture_window_png_base64(rect, exclusions)?;

    Ok(ForegroundCapture {
        window_title,
        image_base64,
    })
}

fn get_window_rect(hwnd: HWND) -> Result<RECT> {
    let mut rect = RECT::default();
    unsafe { GetWindowRect(hwnd, &mut rect) }.map_err(|_| anyhow!("Could not read target window bounds."))?;
    Ok(rect)
}

fn get_window_title(hwnd: HWND) -> Result<String> {
    let title_length = unsafe { GetWindowTextLengthW(hwnd) };
    let mut buffer = vec![0u16; title_length as usize + 1];

    let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    if copied == 0 {
        return Ok("Untitled Window".to_string());
    }

    String::from_utf16(&buffer[..copied as usize]).context("Window title was not valid UTF-16.")
}

fn capture_window_png_base64(window_rect: RECT, exclusions: &[ExclusionRect]) -> Result<String> {
    let screens = Screen::all().context("Could not enumerate displays.")?;
    let screen = pick_screen(&screens, window_rect)?;

    let screen_image = screen.capture().context("Full-screen capture failed.")?;
    let screen_width = screen_image.width();
    let screen_height = screen_image.height();
    let screen_buffer = screen_image.into_raw();

    let rgba = bgra_to_rgba(screen_buffer);
    let mut full_image = RgbaImage::from_raw(screen_width, screen_height, rgba)
        .ok_or_else(|| anyhow!("Captured screen buffer dimensions were invalid."))?;
    apply_exclusion_masks(
        &mut full_image,
        screen.display_info.x,
        screen.display_info.y,
        exclusions,
    );

    let crop = compute_crop(window_rect, screen.display_info.x, screen.display_info.y, screen_width, screen_height)?;
    let cropped = image::imageops::crop_imm(&full_image, crop.0, crop.1, crop.2, crop.3).to_image();

    let mut png_bytes = Vec::new();
    PngEncoder::new(&mut png_bytes)
        .write_image(cropped.as_raw(), cropped.width(), cropped.height(), ColorType::Rgba8.into())
        .context("PNG encoding failed.")?;

    Ok(base64::engine::general_purpose::STANDARD.encode(png_bytes))
}

fn pick_screen<'a>(screens: &'a [Screen], window_rect: RECT) -> Result<&'a Screen> {
    let center_x = (window_rect.left + window_rect.right) / 2;
    let center_y = (window_rect.top + window_rect.bottom) / 2;

    screens
        .iter()
        .find(|screen| {
            let info = &screen.display_info;
            center_x >= info.x
                && center_x < info.x + info.width as i32
                && center_y >= info.y
                && center_y < info.y + info.height as i32
        })
        .or_else(|| screens.first())
        .ok_or_else(|| anyhow!("No displays were available for capture."))
}

fn compute_crop(
    window_rect: RECT,
    screen_x: i32,
    screen_y: i32,
    screen_width: u32,
    screen_height: u32,
) -> Result<(u32, u32, u32, u32)> {
    let left = (window_rect.left - screen_x).max(0) as u32;
    let top = (window_rect.top - screen_y).max(0) as u32;
    let right = (window_rect.right - screen_x).max(0) as u32;
    let bottom = (window_rect.bottom - screen_y).max(0) as u32;

    let clamped_right = right.min(screen_width);
    let clamped_bottom = bottom.min(screen_height);

    if clamped_right <= left || clamped_bottom <= top {
        return Err(anyhow!("Foreground window was outside the captured display."));
    }

    Ok((left, top, clamped_right - left, clamped_bottom - top))
}

fn bgra_to_rgba(mut raw: Vec<u8>) -> Vec<u8> {
    for pixel in raw.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    raw
}

fn apply_exclusion_masks(image: &mut RgbaImage, screen_x: i32, screen_y: i32, exclusions: &[ExclusionRect]) {
    let width = image.width() as i32;
    let height = image.height() as i32;

    for exclusion in exclusions {
        let left = (exclusion.left - screen_x).clamp(0, width);
        let top = (exclusion.top - screen_y).clamp(0, height);
        let right = (exclusion.right - screen_x).clamp(0, width);
        let bottom = (exclusion.bottom - screen_y).clamp(0, height);

        if right <= left || bottom <= top {
            continue;
        }

        for y in top..bottom {
            for x in left..right {
                image.put_pixel(x as u32, y as u32, image::Rgba([0, 0, 0, 255]));
            }
        }
    }
}

fn resolve_capture_window() -> Result<HWND> {
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.0.is_null() {
        return Err(anyhow!("No foreground window detected."));
    }

    if !is_own_process_window(foreground) {
        return Ok(foreground);
    }

    let mut current = foreground;
    for _ in 0..200 {
        let next = match unsafe { GetWindow(current, GW_HWNDNEXT) } {
            Ok(hwnd) if !hwnd.0.is_null() => hwnd,
            _ => break,
        };

        if is_capture_candidate(next) && !is_own_process_window(next) {
            return Ok(next);
        }

        current = next;
    }

    Err(anyhow!(
        "Sentinel is focused and no other visible window was available to capture."
    ))
}

fn is_own_process_window(hwnd: HWND) -> bool {
    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    process_id == unsafe { GetCurrentProcessId() }
}

fn is_capture_candidate(hwnd: HWND) -> bool {
    if !unsafe { IsWindowVisible(hwnd) }.as_bool() || unsafe { IsIconic(hwnd) }.as_bool() {
        return false;
    }

    let Ok(rect) = get_window_rect(hwnd) else {
        return false;
    };

    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;
    width > 120 && height > 120
}
