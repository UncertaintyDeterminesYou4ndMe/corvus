pub mod model_names;
pub mod provider;

use anyhow::Result;
use serde::Serialize;

use crate::config::ClaudeConfig;
use provider::ProviderType;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Severity {
    Ok,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiagResult {
    pub severity: Severity,
    pub category: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct HealthReport {
    pub provider_type: ProviderType,
    pub base_url: String,
    pub results: Vec<DiagResult>,
    pub summary: ReportSummary,
}

#[derive(Debug, Serialize)]
pub struct ReportSummary {
    pub ok_count: usize,
    pub info_count: usize,
    pub warn_count: usize,
    pub error_count: usize,
}

impl HealthReport {
    fn new(provider_type: ProviderType, base_url: String, results: Vec<DiagResult>) -> Self {
        let summary = ReportSummary {
            ok_count: results.iter().filter(|r| r.severity == Severity::Ok).count(),
            info_count: results.iter().filter(|r| r.severity == Severity::Info).count(),
            warn_count: results.iter().filter(|r| r.severity == Severity::Warn).count(),
            error_count: results.iter().filter(|r| r.severity == Severity::Error).count(),
        };
        Self { provider_type, base_url, results, summary }
    }
}

fn diag(severity: Severity, category: &str, message: impl Into<String>) -> DiagResult {
    DiagResult {
        severity,
        category: category.to_string(),
        message: message.into(),
        fix: None,
    }
}

fn diag_with_fix(severity: Severity, category: &str, message: impl Into<String>, fix: impl Into<String>) -> DiagResult {
    DiagResult {
        severity,
        category: category.to_string(),
        message: message.into(),
        fix: Some(fix.into()),
    }
}

/// Run all diagnostic checks and return a health report.
pub fn run_all_checks(config: &ClaudeConfig, skip_network: bool, verbose: u8) -> Result<HealthReport> {
    let mut results = Vec::new();
    let provider_type = provider::detect(&config.env);
    let base_url = config.env.base_url.as_deref().unwrap_or("https://api.anthropic.com").to_string();

    // === api_config ===
    check_api_config(config, &provider_type, &mut results);

    // === model_config ===
    model_names::check_models(config, &provider_type, &mut results);

    // === beta_flags ===
    check_beta_flags(config, &provider_type, &mut results);

    // === files ===
    check_config_files(&mut results);

    // === network ===
    if !skip_network {
        check_network(&base_url, verbose, &mut results);
    }

    Ok(HealthReport::new(provider_type, base_url, results))
}

fn check_api_config(config: &ClaudeConfig, provider: &ProviderType, results: &mut Vec<DiagResult>) {
    // API key
    if config.env.api_key.is_some() {
        let masked = crate::display::mask_key(config.env.api_key.as_deref().unwrap());
        results.push(diag(Severity::Ok, "api_config", format!("API key configured ({})", masked)));
    } else if config.env.auth_token.is_some() {
        results.push(diag(Severity::Ok, "api_config", "Auth token configured (ANTHROPIC_AUTH_TOKEN)"));
    } else {
        results.push(diag_with_fix(
            Severity::Error, "api_config",
            "No API key configured",
            "export ANTHROPIC_API_KEY=\"your-key-here\"",
        ));
    }

    // Base URL
    if let Some(ref url) = config.env.base_url {
        results.push(diag(Severity::Ok, "api_config", format!("Base URL: {}", url)));
    } else {
        results.push(diag(Severity::Info, "api_config", "Using default Anthropic API (no ANTHROPIC_BASE_URL set)"));
    }

    // Provider detection
    results.push(diag(Severity::Info, "api_config", format!("Provider detected: {}", provider)));
}

fn check_beta_flags(config: &ClaudeConfig, provider: &ProviderType, results: &mut Vec<DiagResult>) {
    match provider {
        ProviderType::AwsBedrock => {
            if config.env.disable_experimental_betas {
                results.push(diag(Severity::Ok, "beta_flags", "CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1 (required for Bedrock)"));
            } else {
                results.push(diag_with_fix(
                    Severity::Error, "beta_flags",
                    "Bedrock does not support experimental beta flags — will cause 400 errors",
                    "export CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1",
                ));
            }
        }
        ProviderType::AnthropicDirect => {
            results.push(diag(Severity::Ok, "beta_flags", "Direct Anthropic API — all beta flags supported"));
        }
        _ => {
            // Relay services
            if config.env.disable_experimental_betas {
                results.push(diag(Severity::Ok, "beta_flags", "CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1"));
            } else {
                results.push(diag_with_fix(
                    Severity::Warn, "beta_flags",
                    "Relay services may not support experimental beta flags",
                    "export CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS=1",
                ));
            }
            if config.env.betas.is_some() {
                results.push(diag_with_fix(
                    Severity::Warn, "beta_flags",
                    format!("ANTHROPIC_BETAS is set — relay service may not recognize these"),
                    "unset ANTHROPIC_BETAS",
                ));
            }
        }
    }
}

fn check_config_files(results: &mut Vec<DiagResult>) {
    use crate::config::settings;

    if settings::settings_file_path("settings.json").is_some() {
        results.push(diag(Severity::Ok, "files", format!("~/.claude/settings.json exists")));
    } else {
        results.push(diag(Severity::Info, "files", "~/.claude/settings.json not found (using defaults)"));
    }

    if settings::settings_file_path("settings.local.json").is_some() {
        results.push(diag(Severity::Ok, "files", "~/.claude/settings.local.json exists"));
    }

    if settings::state_file_path().is_some() {
        results.push(diag(Severity::Ok, "files", "~/.claude.json exists"));
    } else {
        results.push(diag(Severity::Warn, "files", "~/.claude.json not found — Claude Code may not have been run yet"));
    }
}

fn check_network(base_url: &str, _verbose: u8, results: &mut Vec<DiagResult>) {
    let start = std::time::Instant::now();
    match ureq::get(base_url).call() {
        Ok(_) | Err(ureq::Error::Status(_, _)) => {
            // Any HTTP response (even 4xx/5xx) means the server is reachable
            let elapsed = start.elapsed();
            results.push(diag(
                Severity::Ok, "network",
                format!("{} reachable ({:.0}ms)", base_url, elapsed.as_millis()),
            ));
        }
        Err(e) => {
            results.push(diag_with_fix(
                Severity::Error, "network",
                format!("{} unreachable: {}", base_url, e),
                "Check your network connection or ANTHROPIC_BASE_URL setting",
            ));
        }
    }
}
