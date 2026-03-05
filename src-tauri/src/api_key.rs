use anyhow::{anyhow, Context, Result};
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

const API_KEY_FILE: &str = "openai_api_key.txt";

pub fn has_user_api_key(app: &tauri::AppHandle) -> Result<bool> {
    let path = key_file_path(app)?;
    if !path.exists() {
        return Ok(false);
    }

    let key = fs::read_to_string(path).context("Could not read stored API key.")?;
    Ok(!key.trim().is_empty())
}

pub fn save_user_api_key(app: &tauri::AppHandle, api_key: &str) -> Result<()> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("API key cannot be empty."));
    }
    if !trimmed.starts_with("sk-") {
        return Err(anyhow!("API key should start with 'sk-'."));
    }

    let path = key_file_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Could not prepare app data directory.")?;
    }

    fs::write(&path, trimmed).context("Could not save API key.")?;
    std::env::set_var("OPENAI_API_KEY", trimmed);
    Ok(())
}

pub fn resolve_api_key(app: &tauri::AppHandle) -> Result<String> {
    if let Ok(env_key) = std::env::var("OPENAI_API_KEY") {
        let env_key = env_key.trim();
        if !env_key.is_empty() {
            return Ok(env_key.to_string());
        }
    }

    let path = key_file_path(app)?;
    let file_key = fs::read_to_string(path).context("No API key is configured.")?;
    let file_key = file_key.trim().to_string();

    if file_key.is_empty() {
        return Err(anyhow!("No API key is configured."));
    }

    std::env::set_var("OPENAI_API_KEY", &file_key);
    Ok(file_key)
}

fn key_file_path(app: &tauri::AppHandle) -> Result<PathBuf> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .context("Could not resolve app data directory.")?;
    Ok(app_data_dir.join(API_KEY_FILE))
}
