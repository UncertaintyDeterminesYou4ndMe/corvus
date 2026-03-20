use std::io::Read;
use anyhow::{Context, Result, bail};
use crate::config::ClaudeConfig;
use crate::display::{self, OutputFormat};

pub fn run(
    url_override: Option<&str>,
    key_override: Option<&str>,
    verbose: u8,
    format: &OutputFormat,
) -> Result<()> {
    let config = ClaudeConfig::load()?;

    let base_url = url_override
        .or(config.env.base_url.as_deref())
        .unwrap_or("https://api.anthropic.com");
    let api_key = key_override
        .or(config.env.api_key.as_deref())
        .context("No API key configured. Set ANTHROPIC_API_KEY or use --key")?;

    let url = format!("{}/v1/models", base_url.trim_end_matches('/'));

    if verbose > 0 {
        eprintln!("Querying: {}", url);
    }

    let resp = ureq::get(&url)
        .set("Authorization", &format!("Bearer {}", api_key))
        .call()
        .context("Failed to query models endpoint")?;

    let mut body_str = String::new();
    resp.into_reader().read_to_string(&mut body_str)
        .context("Failed to read response body")?;

    let body: serde_json::Value = serde_json::from_str(&body_str)
        .context("Failed to parse models response as JSON")?;

    // Try OpenAI-compatible format first, then fall back to raw array
    let provider_models: Vec<String> = if let Some(data) = body.get("data").and_then(|d| d.as_array()) {
        data.iter()
            .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
            .collect()
    } else if let Some(arr) = body.as_array() {
        arr.iter()
            .filter_map(|m| m.get("id").and_then(|id| id.as_str()).map(String::from))
            .collect()
    } else {
        bail!("Unexpected models response format");
    };

    display::print_models_report(&config, &provider_models, base_url, format)?;
    Ok(())
}
