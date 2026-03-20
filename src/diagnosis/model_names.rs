use crate::config::ClaudeConfig;
use crate::diagnosis::{DiagResult, Severity, diag, diag_with_fix};
use crate::diagnosis::provider::ProviderType;
use crate::known_models;

/// Run model-related diagnostic checks.
pub fn check_models(config: &ClaudeConfig, provider: &ProviderType, results: &mut Vec<DiagResult>) {
    let configured = config.env.configured_models();

    if configured.is_empty() {
        results.push(diag(
            Severity::Info, "model_config",
            "No model env vars set — using Claude Code defaults",
        ));
        return;
    }

    for (env_var, model_id) in &configured {
        // Check if model is known
        if known_models::lookup(model_id).is_none() {
            // Not in our registry — might be valid but we can't verify
            results.push(diag(
                Severity::Warn, "model_config",
                format!("{}=\"{}\" — not in known model registry (may still be valid)", env_var, model_id),
            ));
            continue;
        }

        // Check for problematic suffixes with relay services
        if let Some(suffix) = known_models::has_problematic_suffix(model_id) {
            if provider.is_relay() || *provider == ProviderType::AwsBedrock {
                let base = model_id.strip_suffix(suffix).unwrap_or(model_id);
                results.push(diag_with_fix(
                    Severity::Warn, "model_config",
                    format!("{}=\"{}\" — \"{}\" suffix may not be supported by {}", env_var, model_id, suffix, provider),
                    format!("export {}=\"{}\"", env_var, base),
                ));
                continue;
            }
        }

        // Check short aliases with relay services
        if known_models::is_short_alias(model_id) && (provider.is_relay() || *provider == ProviderType::AwsBedrock) {
            if let Some(canonical) = known_models::canonicalize(model_id) {
                results.push(diag_with_fix(
                    Severity::Warn, "model_config",
                    format!("{}=\"{}\" — short alias may not be in provider's model list", env_var, model_id),
                    format!("Try the full model ID: export {}=\"{}\"", env_var, canonical),
                ));
                continue;
            }
        }

        // Check family mismatch (e.g., Opus env var has Sonnet model)
        if let Some(info) = known_models::lookup(model_id) {
            let expected_family = match *env_var {
                "ANTHROPIC_DEFAULT_OPUS_MODEL" => Some(known_models::ModelFamily::Opus),
                "ANTHROPIC_DEFAULT_SONNET_MODEL" => Some(known_models::ModelFamily::Sonnet),
                "ANTHROPIC_DEFAULT_HAIKU_MODEL" => Some(known_models::ModelFamily::Haiku),
                _ => None,
            };
            if let Some(expected) = expected_family {
                if info.family != expected {
                    results.push(diag(
                        Severity::Warn, "model_config",
                        format!(
                            "{}=\"{}\" — model is {:?} but env var is for {:?}",
                            env_var, model_id, info.family, expected
                        ),
                    ));
                    continue;
                }
            }
        }

        // All good
        results.push(diag(
            Severity::Ok, "model_config",
            format!("{}=\"{}\"", env_var, model_id),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::env_vars::EnvConfig;

    fn make_config(env: EnvConfig) -> ClaudeConfig {
        ClaudeConfig { env, settings: None, settings_local: None }
    }

    #[test]
    fn test_thinking_suffix_warning() {
        let config = make_config(EnvConfig {
            base_url: Some("https://api.apimart.ai".into()),
            default_opus_model: Some("claude-opus-4-20250514-thinking".into()),
            ..Default::default()
        });
        let provider = ProviderType::Apimart;
        let mut results = Vec::new();
        check_models(&config, &provider, &mut results);
        assert!(results.iter().any(|r| r.severity == Severity::Warn && r.message.contains("-thinking")));
    }

    #[test]
    fn test_short_alias_warning() {
        let config = make_config(EnvConfig {
            base_url: Some("https://api.apimart.ai".into()),
            default_sonnet_model: Some("claude-sonnet-4-6".into()),
            ..Default::default()
        });
        let provider = ProviderType::Apimart;
        let mut results = Vec::new();
        check_models(&config, &provider, &mut results);
        assert!(results.iter().any(|r| r.severity == Severity::Warn && r.message.contains("short alias")));
    }

    #[test]
    fn test_valid_model_ok() {
        let config = make_config(EnvConfig {
            default_sonnet_model: Some("claude-sonnet-4-5-20250929".into()),
            ..Default::default()
        });
        let provider = ProviderType::AnthropicDirect;
        let mut results = Vec::new();
        check_models(&config, &provider, &mut results);
        assert!(results.iter().any(|r| r.severity == Severity::Ok));
    }

    #[test]
    fn test_family_mismatch() {
        let config = make_config(EnvConfig {
            default_opus_model: Some("claude-sonnet-4-5-20250929".into()),
            ..Default::default()
        });
        let provider = ProviderType::AnthropicDirect;
        let mut results = Vec::new();
        check_models(&config, &provider, &mut results);
        assert!(results.iter().any(|r| r.severity == Severity::Warn && r.message.contains("Sonnet") && r.message.contains("Opus")));
    }
}
