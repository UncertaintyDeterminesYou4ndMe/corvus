use anyhow::Result;
use colored::Colorize;
use serde::Serialize;
use crate::config::ClaudeConfig;
use crate::config::stats_cache::{StatsCache, UsageInfo};
use crate::diagnosis::{HealthReport, Severity};
use crate::known_models;

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

/// Mask an API key for display: show first 4 + last 4 chars.
pub fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        return "****".to_string();
    }
    format!("{}****{}", &key[..4], &key[key.len()-4..])
}

fn severity_icon(s: Severity) -> colored::ColoredString {
    match s {
        Severity::Ok => "  ✓ ".green(),
        Severity::Info => "  ℹ ".blue(),
        Severity::Warn => "  ⚠ ".yellow(),
        Severity::Error => "  ✗ ".red(),
    }
}

// ─── Health Report ───

pub fn print_health_report(report: &HealthReport, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(report)?);
        }
        OutputFormat::Text => print_health_report_text(report),
    }
    Ok(())
}

fn print_health_report_text(report: &HealthReport) {
    println!();
    println!("{}", "Corvus - Claude Code Health Report".bold());
    println!("{}", "===================================".dimmed());
    println!();
    println!("Provider: {} ({})", report.provider_type.to_string().cyan(), report.base_url.dimmed());
    println!();

    // Group by category
    let categories = ["api_config", "model_config", "beta_flags", "files", "network"];
    for cat in &categories {
        let items: Vec<_> = report.results.iter().filter(|r| r.category == *cat).collect();
        if items.is_empty() {
            continue;
        }
        println!("  [{}]", cat.bold());
        for item in items {
            print!("  {}{}", severity_icon(item.severity), item.message);
            println!();
            if let Some(ref fix) = item.fix {
                println!("       {} {}", "Fix:".yellow().bold(), fix);
            }
        }
        println!();
    }

    // Summary
    let s = &report.summary;
    print!("Summary: ");
    print!("{}", format!("{} ok", s.ok_count).green());
    if s.info_count > 0 {
        print!(", {}", format!("{} info", s.info_count).blue());
    }
    if s.warn_count > 0 {
        print!(", {}", format!("{} warnings", s.warn_count).yellow());
    }
    if s.error_count > 0 {
        print!(", {}", format!("{} errors", s.error_count).red());
    }
    println!();
    println!();
}

// ─── Env Summary ───

pub fn print_env_summary(config: &ClaudeConfig, show_secrets: bool, _verbose: u8, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&config.env)?);
        }
        OutputFormat::Text => print_env_summary_text(config, show_secrets),
    }
    Ok(())
}

fn print_env_summary_text(config: &ClaudeConfig, show_secrets: bool) {
    println!();
    println!("{}", "Corvus - Claude Code Environment".bold());
    println!("{}", "=================================".dimmed());
    println!();

    // API Configuration
    println!("{}", "API Configuration:".bold());
    print_env_var("ANTHROPIC_API_KEY", config.env.api_key.as_deref(), show_secrets, true);
    print_env_var("ANTHROPIC_BASE_URL", config.env.base_url.as_deref(), show_secrets, false);
    print_env_var("ANTHROPIC_AUTH_TOKEN", config.env.auth_token.as_deref(), show_secrets, true);
    if config.env.bedrock_base_url.is_some() || config.env.aws_bearer_token_bedrock.is_some() {
        print_env_var("ANTHROPIC_BEDROCK_BASE_URL", config.env.bedrock_base_url.as_deref(), show_secrets, false);
        print_env_var("AWS_BEARER_TOKEN_BEDROCK", config.env.aws_bearer_token_bedrock.as_deref(), show_secrets, true);
    }
    println!();

    // Model Configuration
    println!("{}", "Model Configuration:".bold());
    print_env_var("ANTHROPIC_MODEL", config.env.model.as_deref(), show_secrets, false);
    print_env_var("ANTHROPIC_DEFAULT_OPUS_MODEL", config.env.default_opus_model.as_deref(), show_secrets, false);
    print_env_var("ANTHROPIC_DEFAULT_SONNET_MODEL", config.env.default_sonnet_model.as_deref(), show_secrets, false);
    print_env_var("ANTHROPIC_DEFAULT_HAIKU_MODEL", config.env.default_haiku_model.as_deref(), show_secrets, false);
    print_env_var("ANTHROPIC_SMALL_FAST_MODEL", config.env.small_fast_model.as_deref(), show_secrets, false);
    print_env_var("ANTHROPIC_BETAS", config.env.betas.as_deref(), show_secrets, false);
    println!();

    // Feature Toggles
    println!("{}", "Feature Toggles:".bold());
    print_bool_var("CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS", config.env.disable_experimental_betas);
    print_bool_var("CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS", config.env.disable_git_instructions);
    print_bool_var("CLAUDE_CODE_DISABLE_1M_CONTEXT", config.env.disable_1m_context);
    print_bool_var("CLAUDE_CODE_SIMPLE", config.env.simple_mode);
    print_bool_var("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", config.env.disable_nonessential_traffic);
    println!();

    // Config Files
    println!("{}", "Config Files:".bold());
    use crate::config::settings;
    let files = [
        ("~/.claude/settings.json", settings::settings_file_path("settings.json")),
        ("~/.claude/settings.local.json", settings::settings_file_path("settings.local.json")),
        ("~/.claude.json", settings::state_file_path()),
    ];
    for (name, path) in &files {
        if path.is_some() {
            println!("  {:<40} {}", name, "EXISTS".green());
        } else {
            println!("  {:<40} {}", name, "(not found)".dimmed());
        }
    }
    println!();

    // Effective Configuration
    println!("{}", "Effective Configuration:".bold());
    let provider = crate::diagnosis::provider::detect(&config.env);
    println!("  Provider: {}", provider.to_string().cyan());
    if let Some(model) = config.effective_model() {
        println!("  Model:    {} (from {})", model.cyan(),
            if config.env.model.is_some() { "env" } else { "settings.json" });
    }
    if let Some(ref s) = config.settings {
        if let Some(true) = s.always_thinking_enabled {
            println!("  Thinking: {}", "always enabled".green());
        }
    }
    println!();
}

fn print_env_var(name: &str, value: Option<&str>, show_secrets: bool, is_secret: bool) {
    match value {
        Some(v) => {
            let display = if is_secret && !show_secrets { mask_key(v) } else { v.to_string() };
            println!("  {:<40} = {}", name, display.cyan());
        }
        None => {
            println!("  {:<40} = {}", name, "(not set)".dimmed());
        }
    }
}

fn print_bool_var(name: &str, value: bool) {
    let display = if value {
        "1".green().to_string()
    } else {
        "(not set)".dimmed().to_string()
    };
    println!("  {:<40} = {}", name, display);
}

// ─── Models Report ───

pub fn print_models_report(config: &ClaudeConfig, provider_models: &[String], base_url: &str, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            #[derive(Serialize)]
            struct Report<'a> { base_url: &'a str, provider_models: &'a [String], configured: Vec<(&'static str, &'a str, bool)> }
            let configured: Vec<_> = config.env.configured_models().iter()
                .map(|(name, id)| (*name, *id, provider_models.iter().any(|m| m == id)))
                .collect();
            let r = Report { base_url, provider_models, configured };
            println!("{}", serde_json::to_string_pretty(&r)?);
        }
        OutputFormat::Text => print_models_report_text(config, provider_models, base_url),
    }
    Ok(())
}

fn print_models_report_text(config: &ClaudeConfig, provider_models: &[String], base_url: &str) {
    println!();
    println!("{}", "Corvus - Provider Models".bold());
    println!("{}", "========================".dimmed());
    println!();
    println!("Endpoint: {}", base_url.cyan());
    println!("Models found: {}", provider_models.len().to_string().green());
    println!();

    // Show all provider models
    println!("{}", "Available Models:".bold());
    for model in provider_models {
        let known = known_models::lookup(model);
        if known.is_some() {
            println!("  {} {}", "✓".green(), model);
        } else {
            println!("  {} {}", "·".dimmed(), model);
        }
    }
    println!();

    // Cross-reference with configuration
    let configured = config.env.configured_models();
    if !configured.is_empty() {
        println!("{}", "Configuration Cross-Reference:".bold());
        for (env_var, model_id) in &configured {
            let found = provider_models.iter().any(|m| m == model_id);
            if found {
                println!("  {} {}=\"{}\"", "✓".green(), env_var, model_id);
            } else {
                println!("  {} {}=\"{}\" — {}", "✗".red(), env_var, model_id, "NOT in provider's model list".red());
                // Suggest closest match
                if let Some(canonical) = known_models::canonicalize(model_id) {
                    if provider_models.iter().any(|m| m == canonical) {
                        println!("    {} Try: export {}=\"{}\"", "→".yellow(), env_var, canonical);
                    }
                }
            }
        }
        println!();
    }
}

// ─── Stats Report ───

pub fn print_stats_report(
    stats: &StatsCache,
    usage: &UsageInfo,
    daily: bool,
    by_model: bool,
    _verbose: u8,
    format: &OutputFormat,
) -> Result<()> {
    match format {
        OutputFormat::Json => {
            // UsageInfo doesn't serialize; output stats only
            println!("{}", serde_json::to_string_pretty(stats)?);
        }
        OutputFormat::Text => print_stats_report_text(stats, usage, daily, by_model),
    }
    Ok(())
}

fn print_stats_report_text(stats: &StatsCache, usage: &UsageInfo, daily: bool, _by_model: bool) {
    println!();
    println!("{}", "Corvus - Usage Statistics".bold());
    println!("{}", "=========================".dimmed());
    println!();

    // Overview
    println!("{}", "Overview:".bold());
    if let Some(ref date) = stats.first_session_date {
        println!("  First session: {}", date.split('T').next().unwrap_or(date).cyan());
    }
    println!("  Total sessions: {} | Messages: {} | Startups: {}",
        stats.total_sessions.to_string().green(),
        stats.total_messages.to_string().green(),
        usage.num_startups.to_string().green(),
    );
    if let Some(ref ls) = stats.longest_session {
        println!("  Longest session: {} messages ({})",
            ls.message_count.to_string().cyan(),
            ls.session_id.as_deref().unwrap_or("?").get(..8).unwrap_or("?").dimmed(),
        );
    }
    println!();

    // Token Usage by Model
    if !stats.model_usage.is_empty() {
        println!("{}", "Token Usage by Model:".bold());
        let mut total_cost = 0.0f64;
        for (model, mu) in &stats.model_usage {
            println!("  {}:", model.cyan());

            // Look up pricing
            let pricing = known_models::lookup(model);
            let (in_cost, out_cost, cr_cost, cw_cost) = if let Some(p) = pricing {
                (
                    mu.input_tokens as f64 * p.input_cost_per_mtok / 1_000_000.0,
                    mu.output_tokens as f64 * p.output_cost_per_mtok / 1_000_000.0,
                    mu.cache_read_input_tokens as f64 * p.cache_read_cost_per_mtok / 1_000_000.0,
                    mu.cache_creation_input_tokens as f64 * p.cache_write_cost_per_mtok / 1_000_000.0,
                )
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };

            println!("    Input:       {:>12} tokens  (${:.2})", format_num(mu.input_tokens), in_cost);
            println!("    Output:      {:>12} tokens  (${:.2})", format_num(mu.output_tokens), out_cost);
            println!("    Cache Read:  {:>12} tokens  (${:.2})", format_num(mu.cache_read_input_tokens), cr_cost);
            println!("    Cache Write: {:>12} tokens  (${:.2})", format_num(mu.cache_creation_input_tokens), cw_cost);

            let model_total = in_cost + out_cost + cr_cost + cw_cost;
            total_cost += model_total;
            println!("    {}", "─────────────────────────".dimmed());
            println!("    Estimated:                   {}", format!("${:.2}", model_total).green().bold());
            println!();
        }
        if stats.model_usage.len() > 1 {
            println!("  Total Estimated Cost: {}", format!("${:.2}", total_cost).green().bold());
            println!();
        }
    }

    // Tool Usage
    if !usage.tool_usage.is_empty() {
        println!("{}", "Tool Usage (Top 5):".bold());
        let mut tools: Vec<_> = usage.tool_usage.iter().collect();
        tools.sort_by(|a, b| b.1.usage_count.cmp(&a.1.usage_count));
        let top: Vec<_> = tools.iter().take(5)
            .map(|(name, info)| format!("{}: {}", name, info.usage_count))
            .collect();
        println!("  {}", top.join(" | "));
        println!();
    }

    // Skill Usage
    if !usage.skill_usage.is_empty() {
        println!("{}", "Skill Usage (Top 5):".bold());
        let mut skills: Vec<_> = usage.skill_usage.iter().collect();
        skills.sort_by(|a, b| b.1.usage_count.cmp(&a.1.usage_count));
        let top: Vec<_> = skills.iter().take(5)
            .map(|(name, info)| format!("{}: {}", name, info.usage_count))
            .collect();
        println!("  {}", top.join(" | "));
        println!();
    }

    // Active Hours
    if !stats.hour_counts.is_empty() {
        println!("{}", "Active Hours:".bold());
        let max_count = stats.hour_counts.values().max().copied().unwrap_or(1);
        // 24 chars for 24 hours
        print!("  ");
        for hour in 0..24 {
            let count = stats.hour_counts.get(&hour.to_string()).copied().unwrap_or(0);
            let filled = if max_count > 0 { (count as f64 / max_count as f64 * 2.0) as usize } else { 0 };
            if filled >= 2 {
                print!("{}", "█".green());
            } else if filled >= 1 {
                print!("{}", "▄".green());
            } else {
                print!("{}", "░".dimmed());
            }
        }
        println!();

        // Find peak hours
        let mut peak: Vec<_> = stats.hour_counts.iter().collect();
        peak.sort_by(|a, b| b.1.cmp(a.1));
        let top_hours: Vec<_> = peak.iter().take(3)
            .map(|(h, _)| format!("{}:00", h))
            .collect();
        if !top_hours.is_empty() {
            println!("  Peak: {}", top_hours.join(", ").cyan());
        }
        println!();
    }

    // Daily breakdown
    if daily && !stats.daily_activity.is_empty() {
        println!("{}", "Daily Activity (recent):".bold());
        let recent: Vec<_> = stats.daily_activity.iter().rev().take(14).collect();
        for day in recent.iter().rev() {
            println!("  {} | msgs: {:>4} | sessions: {:>2} | tools: {:>4}",
                day.date.dimmed(),
                day.message_count.to_string().cyan(),
                day.session_count,
                day.tool_call_count,
            );
        }
        println!();
    }
}

fn format_num(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_key() {
        assert_eq!(mask_key("sk-1234567890abcdef"), "sk-1****cdef");
        assert_eq!(mask_key("short"), "****");
    }

    #[test]
    fn test_format_num() {
        assert_eq!(format_num(500), "500");
        assert_eq!(format_num(1500), "1.5K");
        assert_eq!(format_num(1_500_000), "1.5M");
    }
}
