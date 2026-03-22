use colored::Colorize;

use crate::diagnosis::provider::ProviderType;
use crate::known_models;

/// Extracted info from a Claude Code API request.
#[derive(Debug)]
pub struct RequestAnalysis {
    pub method: String,
    pub path: String,
    pub model: Option<String>,
    pub message_count: Option<usize>,
    pub tool_count: Option<usize>,
    pub anthropic_version: Option<String>,
    pub beta_flags: Vec<String>,
    pub is_streaming: bool,
    pub warnings: Vec<String>,
}

/// Extracted info from an API response.
#[derive(Debug)]
pub struct ResponseAnalysis {
    pub status: u16,
    pub duration_ms: u128,
    pub output_tokens: Option<u64>,
    pub error_message: Option<String>,
}

/// Parse the request body (JSON) to extract model, messages, tools.
pub fn analyze_request_body(body: &[u8]) -> (Option<String>, Option<usize>, Option<usize>, bool) {
    let Ok(val) = serde_json::from_slice::<serde_json::Value>(body) else {
        return (None, None, None, false);
    };

    let model = val.get("model").and_then(|m| m.as_str()).map(String::from);
    let msg_count = val.get("messages").and_then(|m| m.as_array()).map(|a| a.len());
    let tool_count = val.get("tools").and_then(|t| t.as_array()).map(|a| a.len());
    let is_streaming = val.get("stream").and_then(|s| s.as_bool()).unwrap_or(false);

    (model, msg_count, tool_count, is_streaming)
}

/// Extract beta flags from the anthropic-beta header.
pub fn parse_beta_flags(header_value: &str) -> Vec<String> {
    header_value
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Generate warnings based on the request analysis and provider type.
pub fn check_request(analysis: &RequestAnalysis, provider: &ProviderType) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check model name
    if let Some(ref model) = analysis.model {
        if let Some(suffix) = known_models::has_problematic_suffix(model) {
            if provider.is_relay() || *provider == ProviderType::AwsBedrock {
                warnings.push(format!(
                    "Model \"{}\" has \"{}\" suffix — may not be supported by {}",
                    model, suffix, provider
                ));
            }
        }
        if known_models::is_short_alias(model) && provider.is_relay() {
            warnings.push(format!(
                "Model \"{}\" is a short alias — provider may not recognize it",
                model
            ));
        }
    }

    // Check beta flags
    if !analysis.beta_flags.is_empty() {
        match provider {
            ProviderType::AwsBedrock => {
                warnings.push(format!(
                    "Beta flags sent to Bedrock (will cause 400): {}",
                    analysis.beta_flags.join(", ")
                ));
            }
            p if p.is_relay() => {
                warnings.push(format!(
                    "Beta flags: {} (may not be supported)",
                    analysis.beta_flags.join(", ")
                ));
            }
            _ => {}
        }
    }

    // Check anthropic-version header with Bedrock
    if let Some(ref ver) = analysis.anthropic_version {
        if *provider == ProviderType::AwsBedrock {
            warnings.push(format!(
                "anthropic-version header \"{}\" sent to Bedrock — may cause 400",
                ver
            ));
        }
    }

    warnings
}

/// Format a request log line for terminal display.
pub fn format_request_log(analysis: &RequestAnalysis, timestamp: &str) -> String {
    let mut out = String::new();

    // Timestamp + method + path
    out.push_str(&format!(
        "[{}] {} {}",
        timestamp.dimmed(),
        analysis.method.cyan().bold(),
        analysis.path,
    ));
    out.push('\n');

    // Model + messages + tools
    let mut details = Vec::new();
    if let Some(ref model) = analysis.model {
        details.push(format!("Model: {}", model.cyan()));
    }
    if let Some(count) = analysis.message_count {
        details.push(format!("Messages: {}", count));
    }
    if let Some(count) = analysis.tool_count {
        details.push(format!("Tools: {}", count));
    }
    if analysis.is_streaming {
        details.push("streaming".green().to_string());
    }
    if !details.is_empty() {
        out.push_str(&format!("  {}\n", details.join("  ")));
    }

    // Headers
    if let Some(ref ver) = analysis.anthropic_version {
        out.push_str(&format!("  Headers: anthropic-version={}\n", ver));
    }

    // Warnings
    for warn in &analysis.warnings {
        out.push_str(&format!("  {} {}\n", "⚠".yellow(), warn.yellow()));
    }

    out
}

/// Format a verbose body dump (pretty JSON, or truncated raw).
pub fn format_body_dump(label: &str, body: &[u8]) -> String {
    if body.is_empty() {
        return String::new();
    }
    let mut out = format!("  {} {}\n", "┌─".dimmed(), label.dimmed());
    if let Ok(val) = serde_json::from_slice::<serde_json::Value>(body) {
        // Mask api_key field if present
        let pretty = serde_json::to_string_pretty(&val).unwrap_or_default();
        for line in pretty.lines() {
            out.push_str(&format!("  {} {}\n", "│".dimmed(), line.dimmed()));
        }
    } else {
        // Non-JSON (e.g. streaming chunks): show first 512 bytes
        let preview = std::str::from_utf8(&body[..body.len().min(512)])
            .unwrap_or("<binary>");
        out.push_str(&format!("  {} {}\n", "│".dimmed(), preview.dimmed()));
        if body.len() > 512 {
            out.push_str(&format!("  {} … ({} bytes total)\n", "│".dimmed(), body.len()));
        }
    }
    out.push_str(&format!("  {}\n", "└─".dimmed()));
    out
}

/// Format a response log line.
pub fn format_response_log(resp: &ResponseAnalysis) -> String {
    let status_str = if resp.status < 300 {
        format!("{}", resp.status).green().to_string()
    } else if resp.status < 500 {
        format!("{}", resp.status).yellow().to_string()
    } else {
        format!("{}", resp.status).red().to_string()
    };

    let mut out = format!("  → {} ({:.1}s", status_str, resp.duration_ms as f64 / 1000.0);

    if let Some(tokens) = resp.output_tokens {
        out.push_str(&format!(", {} output tokens", tokens));
    }
    out.push(')');

    if let Some(ref err) = resp.error_message {
        out.push_str(&format!("\n  {} {}", "Error:".red().bold(), err));
    }

    out.push('\n');
    out
}
