use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

use super::settings;

/// Aggregated stats from stats-cache.json
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StatsCache {
    #[serde(default)]
    pub version: u32,
    #[serde(default)]
    pub last_computed_date: Option<String>,
    #[serde(default)]
    pub daily_activity: Vec<DailyActivity>,
    #[serde(default)]
    pub daily_model_tokens: Vec<DailyModelTokens>,
    #[serde(default)]
    pub model_usage: HashMap<String, ModelUsage>,
    #[serde(default)]
    pub total_sessions: u64,
    #[serde(default)]
    pub total_messages: u64,
    #[serde(default)]
    pub longest_session: Option<LongestSession>,
    #[serde(default)]
    pub first_session_date: Option<String>,
    #[serde(default)]
    pub hour_counts: HashMap<String, u64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyActivity {
    pub date: String,
    #[serde(default)]
    pub message_count: u64,
    #[serde(default)]
    pub session_count: u64,
    #[serde(default)]
    pub tool_call_count: u64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyModelTokens {
    pub date: String,
    #[serde(default)]
    pub tokens_by_model: HashMap<String, u64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelUsage {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
    #[serde(default)]
    pub web_search_requests: u64,
    #[serde(default)]
    pub cost_usd: f64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LongestSession {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub duration: u64,
    #[serde(default)]
    pub message_count: u64,
    #[serde(default)]
    pub timestamp: Option<String>,
}

/// Usage info from ~/.claude.json (complementary fields)
#[derive(Debug, Default, Serialize)]
pub struct UsageInfo {
    pub num_startups: u64,
    pub skill_usage: HashMap<String, ToolUsageEntry>,
    pub tool_usage: HashMap<String, ToolUsageEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolUsageEntry {
    #[serde(default)]
    pub usage_count: u64,
    #[serde(default)]
    pub last_used_at: u64,
}

/// Load stats-cache.json
pub fn load_stats() -> Result<StatsCache> {
    let Some(dir) = settings::claude_dir() else {
        return Ok(StatsCache::default());
    };
    let path = dir.join("stats-cache.json");
    if !path.exists() {
        return Ok(StatsCache::default());
    }
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let stats: StatsCache = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    Ok(stats)
}

/// Load complementary usage info from ~/.claude.json
pub fn load_usage_info() -> Result<UsageInfo> {
    let Some(path) = settings::state_file_path() else {
        return Ok(UsageInfo::default());
    };
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse {}", path.display()))?;

    let num_startups = value.get("numStartups")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let skill_usage = value.get("skillUsage")
        .and_then(|v| serde_json::from_value::<HashMap<String, ToolUsageEntry>>(v.clone()).ok())
        .unwrap_or_default();

    let tool_usage = value.get("toolUsage")
        .and_then(|v| serde_json::from_value::<HashMap<String, ToolUsageEntry>>(v.clone()).ok())
        .unwrap_or_default();

    Ok(UsageInfo {
        num_startups,
        skill_usage,
        tool_usage,
    })
}
