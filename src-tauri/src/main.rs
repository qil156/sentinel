#![cfg_attr(all(not(debug_assertions), target_os = "windows"), windows_subsystem = "windows")]

mod api_key;
mod context;
mod llm;
mod platform;
mod types;

use crate::api_key::{
    get_user_settings, has_user_api_key, list_model_options, resolve_active_config, save_user_api_key, set_active_model,
};
use crate::context::build_screen_context;
use crate::llm::ask_with_provider;
use crate::platform::ExclusionRect;
use crate::types::{AssistantResponse, ProviderModelOption, UserLlmSettings};
use tauri::{Manager, WindowEvent};
use std::time::Duration;

#[tauri::command]
async fn ask_about_screen(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    question: String,
) -> Result<AssistantResponse, String> {
    // Validate provider/model/key first so auth/config errors are not masked by capture errors.
    let (provider, model, api_key) = resolve_active_config(&app).map_err(|err| err.to_string())?;

    let exclusions: Vec<ExclusionRect> = sentinel_window_rect(&window).into_iter().collect();
    let screen_context = build_screen_context(&exclusions).map_err(|err| err.to_string())?;

    ask_with_provider(&provider, &model, &question, &screen_context, &api_key)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
fn has_api_key(app: tauri::AppHandle) -> Result<bool, String> {
    has_user_api_key(&app).map_err(|err| err.to_string())
}

#[tauri::command]
fn save_api_key(app: tauri::AppHandle, provider: String, api_key: String) -> Result<(), String> {
    save_user_api_key(&app, &provider, &api_key).map_err(|err| err.to_string())
}

#[tauri::command]
fn get_user_llm_settings(app: tauri::AppHandle) -> Result<UserLlmSettings, String> {
    get_user_settings(&app).map_err(|err| err.to_string())
}

#[tauri::command]
fn get_model_options() -> Vec<ProviderModelOption> {
    list_model_options()
}

#[tauri::command]
fn set_model_selection(app: tauri::AppHandle, provider: String, model: String) -> Result<UserLlmSettings, String> {
    set_active_model(&app, &provider, &model).map_err(|err| err.to_string())
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
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let tracked = window.clone();
                window.on_window_event(move |event| {
                    if matches!(event, WindowEvent::Focused(false)) {
                        let delayed = tracked.clone();
                        tauri::async_runtime::spawn(async move {
                            tokio::time::sleep(Duration::from_millis(220)).await;
                            let still_unfocused = delayed.is_focused().map(|focused| !focused).unwrap_or(false);
                            let already_minimized = delayed.is_minimized().unwrap_or(false);
                            if still_unfocused && !already_minimized {
                                let _ = delayed.minimize();
                            }
                        });
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ask_about_screen,
            has_api_key,
            save_api_key,
            get_user_llm_settings,
            get_model_options,
            set_model_selection
        ])
        .run(tauri::generate_context!())
        .expect("error while running Sentinel");
}
