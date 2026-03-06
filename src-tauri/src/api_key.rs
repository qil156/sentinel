use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::Manager;

use crate::types::{ProviderModelOption, UserLlmSettings};

const SETTINGS_FILE: &str = "llm_settings.json";
const LEGACY_OPENAI_KEY_FILE: &str = "openai_api_key.txt";
const DEFAULT_PROVIDER: &str = "openai";
const DEFAULT_MODEL: &str = "gpt-5.3-codex";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedSettings {
    selected_provider: String,
    selected_model: String,
    api_keys: HashMap<String, String>,
}

impl Default for PersistedSettings {
    fn default() -> Self {
        Self {
            selected_provider: DEFAULT_PROVIDER.to_string(),
            selected_model: DEFAULT_MODEL.to_string(),
            api_keys: HashMap::new(),
        }
    }
}

pub fn get_user_settings(app: &tauri::AppHandle) -> Result<UserLlmSettings> {
    let settings = load_settings(app)?;
    let has_selected_provider_key = settings
        .api_keys
        .get(&settings.selected_provider)
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false);

    Ok(UserLlmSettings {
        selected_provider: settings.selected_provider,
        selected_model: settings.selected_model,
        has_selected_provider_key,
    })
}

pub fn list_model_options() -> Vec<ProviderModelOption> {
    vec![
        ProviderModelOption {
            provider_id: "openai".to_string(),
            provider_label: "OpenAI".to_string(),
            model_id: "gpt-5.3-codex".to_string(),
            model_label: "gpt-5.3-codex".to_string(),
            is_available: true,
        },
        ProviderModelOption {
            provider_id: "openai".to_string(),
            provider_label: "OpenAI".to_string(),
            model_id: "gpt-4.1".to_string(),
            model_label: "gpt-4.1".to_string(),
            is_available: true,
        },
        ProviderModelOption {
            provider_id: "anthropic".to_string(),
            provider_label: "Claude".to_string(),
            model_id: "claude-sonnet-4-5".to_string(),
            model_label: "claude-sonnet-4-5".to_string(),
            is_available: false,
        },
        ProviderModelOption {
            provider_id: "google".to_string(),
            provider_label: "Gemini".to_string(),
            model_id: "gemini-2.0-flash".to_string(),
            model_label: "gemini-2.0-flash".to_string(),
            is_available: false,
        },
        ProviderModelOption {
            provider_id: "deepseek".to_string(),
            provider_label: "DeepSeek".to_string(),
            model_id: "deepseek-chat".to_string(),
            model_label: "deepseek-chat".to_string(),
            is_available: false,
        },
        ProviderModelOption {
            provider_id: "xai".to_string(),
            provider_label: "Grok".to_string(),
            model_id: "grok-2-latest".to_string(),
            model_label: "grok-2-latest".to_string(),
            is_available: false,
        },
    ]
}

pub fn has_user_api_key(app: &tauri::AppHandle) -> Result<bool> {
    Ok(get_user_settings(app)?.has_selected_provider_key)
}

pub fn save_user_api_key(app: &tauri::AppHandle, provider: &str, api_key: &str) -> Result<()> {
    let trimmed = api_key.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("API key cannot be empty."));
    }
    if !trimmed.starts_with("sk-") {
        return Err(anyhow!("API key should start with 'sk-'."));
    }

    let mut settings = load_settings(app)?;
    settings
        .api_keys
        .insert(provider.trim().to_string(), trimmed.to_string());
    save_settings(app, &settings)?;

    if provider == "openai" {
        std::env::set_var("OPENAI_API_KEY", trimmed);
    }
    Ok(())
}

pub fn set_active_model(app: &tauri::AppHandle, provider: &str, model: &str) -> Result<UserLlmSettings> {
    let provider = provider.trim();
    let model = model.trim();

    let valid = list_model_options()
        .iter()
        .any(|opt| opt.provider_id == provider && opt.model_id == model);
    if !valid {
        return Err(anyhow!("Unsupported provider/model selection."));
    }

    let mut settings = load_settings(app)?;
    settings.selected_provider = provider.to_string();
    settings.selected_model = model.to_string();
    save_settings(app, &settings)?;
    get_user_settings(app)
}

pub fn resolve_active_config(app: &tauri::AppHandle) -> Result<(String, String, String)> {
    let settings = load_settings(app)?;
    let key = settings
        .api_keys
        .get(&settings.selected_provider)
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow!("No API key configured for selected provider."))?;

    if settings.selected_provider == "openai" {
        std::env::set_var("OPENAI_API_KEY", &key);
    }

    Ok((settings.selected_provider, settings.selected_model, key))
}

fn load_settings(app: &tauri::AppHandle) -> Result<PersistedSettings> {
    let settings_path = settings_file_path(app)?;
    if settings_path.exists() {
        let raw = fs::read_to_string(&settings_path).context("Could not read settings file.")?;
        let settings = serde_json::from_str::<PersistedSettings>(&raw).context("Settings file is invalid JSON.")?;
        return Ok(settings);
    }

    let mut settings = PersistedSettings::default();
    let legacy_path = legacy_key_file_path(app)?;
    if legacy_path.exists() {
        if let Ok(legacy_key) = fs::read_to_string(legacy_path) {
            let legacy_key = legacy_key.trim();
            if !legacy_key.is_empty() {
                settings
                    .api_keys
                    .insert(DEFAULT_PROVIDER.to_string(), legacy_key.to_string());
            }
        }
    }

    save_settings(app, &settings)?;
    Ok(settings)
}

fn save_settings(app: &tauri::AppHandle, settings: &PersistedSettings) -> Result<()> {
    let path = settings_file_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Could not prepare app data directory.")?;
    }
    let raw = serde_json::to_string_pretty(settings).context("Could not serialize settings.")?;
    fs::write(path, raw).context("Could not save settings.")?;
    Ok(())
}

fn settings_file_path(app: &tauri::AppHandle) -> Result<PathBuf> {
    if let Ok(app_data_dir) = app.path().app_data_dir() {
        return Ok(app_data_dir.join(SETTINGS_FILE));
    }

    let app_config_dir = app
        .path()
        .app_config_dir()
        .context("Could not resolve app data/config directory.")?;
    Ok(app_config_dir.join(SETTINGS_FILE))
}

fn legacy_key_file_path(app: &tauri::AppHandle) -> Result<PathBuf> {
    if let Ok(app_data_dir) = app.path().app_data_dir() {
        return Ok(app_data_dir.join(LEGACY_OPENAI_KEY_FILE));
    }

    let app_config_dir = app
        .path()
        .app_config_dir()
        .context("Could not resolve app data/config directory.")?;
    Ok(app_config_dir.join(LEGACY_OPENAI_KEY_FILE))
}
