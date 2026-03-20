pub mod env_vars;
pub mod settings;
pub mod stats_cache;

use anyhow::Result;

pub use env_vars::EnvConfig;
pub use settings::Settings;

/// Aggregate of all Claude Code configuration sources.
#[derive(Debug)]
pub struct ClaudeConfig {
    pub env: EnvConfig,
    pub settings: Option<Settings>,
    pub settings_local: Option<Settings>,
}

impl ClaudeConfig {
    /// Load all config sources: env vars + settings files.
    pub fn load() -> Result<Self> {
        Ok(Self {
            env: EnvConfig::load(),
            settings: settings::load_main_settings()?,
            settings_local: settings::load_local_settings()?,
        })
    }

    /// Get the effective model (env overrides settings).
    pub fn effective_model(&self) -> Option<&str> {
        self.env.model.as_deref()
            .or_else(|| self.settings.as_ref().and_then(|s| s.model.as_deref()))
    }
}
