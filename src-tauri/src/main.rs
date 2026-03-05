#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod api_key;
mod context;
mod llm;
mod platform;
mod types;

use crate::api_key::{has_user_api_key, resolve_api_key, save_user_api_key};
use crate::context::build_screen_context;
use crate::llm::ask_openai;
use crate::platform::ExclusionRect;
use crate::types::AssistantResponse;

#[tauri::command]
async fn ask_about_screen(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    question: String,
) -> Result<AssistantResponse, String> {
    let exclusions: Vec<ExclusionRect> = sentinel_window_rect(&window).into_iter().collect();
    let screen_context = build_screen_context(&exclusions).map_err(|err| err.to_string())?;
    let api_key = resolve_api_key(&app).map_err(|err| err.to_string())?;
    ask_openai(&question, &screen_context, &api_key)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn has_api_key(app: tauri::AppHandle) -> Result<bool, String> {
    has_user_api_key(&app).map_err(|err| err.to_string())
}

#[tauri::command]
fn save_api_key(app: tauri::AppHandle, api_key: String) -> Result<(), String> {
    save_user_api_key(&app, &api_key).map_err(|err| err.to_string())
}

fn sentinel_window_rect(window: &tauri::WebviewWindow) -> Option<ExclusionRect> {
    let position = window.outer_position().ok()?;
    let size = window.outer_size().ok()?;

    Some(ExclusionRect {
        left: position.x,
        top: position.y,
        right: position.x + size.width as i32,
        bottom: position.y + size.height as i32,
    })
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ask_about_screen, has_api_key, save_api_key])
        .run(tauri::generate_context!())
        .expect("error while running Sentinel");
}
