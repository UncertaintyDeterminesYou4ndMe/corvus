use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct Settings {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub always_thinking_enabled: Option<bool>,
    #[serde(default)]
    pub skip_dangerous_mode_permission_prompt: Option<bool>,
    #[serde(default)]
    pub output_style: Option<String>,
    #[serde(default)]
    pub permissions: Option<Permissions>,
    #[serde(default)]
    pub status_line: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Default)]
#[allow(dead_code)]
pub struct Permissions {
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
}

/// Get the path to ~/.claude/
pub fn claude_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude"))
}

/// Load settings from a JSON file, returning None if it doesn't exist.
pub fn load_settings(path: &std::path::Path) -> Result<Option<Settings>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let settings: Settings = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(Some(settings))
}

/// Load ~/.claude/settings.json
pub fn load_main_settings() -> Result<Option<Settings>> {
    let Some(dir) = claude_dir() else {
        return Ok(None);
    };
    load_settings(&dir.join("settings.json"))
}

/// Check if a settings file exists and return its path.
pub fn settings_file_path(filename: &str) -> Option<PathBuf> {
    let path = claude_dir()?.join(filename);
    if path.exists() { Some(path) } else { None }
}

/// Get the path to ~/.claude.json (the state file).
pub fn state_file_path() -> Option<PathBuf> {
    let path = dirs::home_dir()?.join(".claude.json");
    if path.exists() { Some(path) } else { None }
}
