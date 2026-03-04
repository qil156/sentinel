#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod capture;
mod context;
mod llm;
mod types;

use crate::capture::ExclusionRect;
use crate::context::build_screen_context;
use crate::llm::ask_openai;
use crate::types::AssistantResponse;

#[tauri::command]
async fn ask_about_screen(window: tauri::WebviewWindow, question: String) -> Result<AssistantResponse, String> {
    let exclusions: Vec<ExclusionRect> = sentinel_window_rect(&window).into_iter().collect();
    let screen_context = build_screen_context(&exclusions).map_err(|err| err.to_string())?;
    ask_openai(&question, &screen_context)
        .await
        .map_err(|err| err.to_string())
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
        .invoke_handler(tauri::generate_handler![ask_about_screen])
        .run(tauri::generate_context!())
        .expect("error while running Sentinel");
}
