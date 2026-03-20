use serde::Serialize;
use std::env;

/// All Claude Code related environment variables.
#[derive(Debug, Default, Serialize)]
pub struct EnvConfig {
    // API Configuration
    pub api_key: Option<String>,
    pub base_url: Option<String>,
    pub auth_token: Option<String>,

    // Bedrock
    pub bedrock_base_url: Option<String>,
    pub aws_bearer_token_bedrock: Option<String>,

    // Model Selection
    pub model: Option<String>,
    pub small_fast_model: Option<String>,
    pub default_opus_model: Option<String>,
    pub default_sonnet_model: Option<String>,
    pub default_haiku_model: Option<String>,
    pub custom_model_option: Option<String>,
    pub custom_model_option_name: Option<String>,
    pub betas: Option<String>,

    // Feature Toggles
    pub disable_experimental_betas: bool,
    pub disable_git_instructions: bool,
    pub disable_1m_context: bool,
    pub simple_mode: bool,
    pub disable_nonessential_traffic: bool,
    pub enable_tasks: bool,
}

impl EnvConfig {
    pub fn load() -> Self {
        Self {
            api_key: env::var("ANTHROPIC_API_KEY").ok(),
            base_url: env::var("ANTHROPIC_BASE_URL").ok(),
            auth_token: env::var("ANTHROPIC_AUTH_TOKEN").ok(),

            bedrock_base_url: env::var("ANTHROPIC_BEDROCK_BASE_URL").ok(),
            aws_bearer_token_bedrock: env::var("AWS_BEARER_TOKEN_BEDROCK").ok(),

            model: env::var("ANTHROPIC_MODEL").ok(),
            small_fast_model: env::var("ANTHROPIC_SMALL_FAST_MODEL").ok(),
            default_opus_model: env::var("ANTHROPIC_DEFAULT_OPUS_MODEL").ok(),
            default_sonnet_model: env::var("ANTHROPIC_DEFAULT_SONNET_MODEL").ok(),
            default_haiku_model: env::var("ANTHROPIC_DEFAULT_HAIKU_MODEL").ok(),
            custom_model_option: env::var("ANTHROPIC_CUSTOM_MODEL_OPTION").ok(),
            custom_model_option_name: env::var("ANTHROPIC_CUSTOM_MODEL_OPTION_NAME").ok(),
            betas: env::var("ANTHROPIC_BETAS").ok(),

            disable_experimental_betas: env::var("CLAUDE_CODE_DISABLE_EXPERIMENTAL_BETAS")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            disable_git_instructions: env::var("CLAUDE_CODE_DISABLE_GIT_INSTRUCTIONS")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            disable_1m_context: env::var("CLAUDE_CODE_DISABLE_1M_CONTEXT")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            simple_mode: env::var("CLAUDE_CODE_SIMPLE")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            disable_nonessential_traffic: env::var("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            enable_tasks: env::var("CLAUDE_CODE_ENABLE_TASKS")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true), // default true
        }
    }

    /// Get all configured model env vars as (env_var_name, model_id) pairs.
    pub fn configured_models(&self) -> Vec<(&'static str, &str)> {
        let mut result = Vec::new();
        if let Some(ref m) = self.model {
            result.push(("ANTHROPIC_MODEL", m.as_str()));
        }
        if let Some(ref m) = self.default_opus_model {
            result.push(("ANTHROPIC_DEFAULT_OPUS_MODEL", m.as_str()));
        }
        if let Some(ref m) = self.default_sonnet_model {
            result.push(("ANTHROPIC_DEFAULT_SONNET_MODEL", m.as_str()));
        }
        if let Some(ref m) = self.default_haiku_model {
            result.push(("ANTHROPIC_DEFAULT_HAIKU_MODEL", m.as_str()));
        }
        if let Some(ref m) = self.small_fast_model {
            result.push(("ANTHROPIC_SMALL_FAST_MODEL", m.as_str()));
        }
        if let Some(ref m) = self.custom_model_option {
            result.push(("ANTHROPIC_CUSTOM_MODEL_OPTION", m.as_str()));
        }
        result
    }
}
