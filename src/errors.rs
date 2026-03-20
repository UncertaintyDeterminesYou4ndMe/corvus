use thiserror::Error;

#[derive(Error, Debug)]
pub enum CorvusError {
    #[error("Config file not found: {path}")]
    ConfigNotFound { path: String },

    #[error("Failed to parse config: {path}: {reason}")]
    ConfigParse { path: String, reason: String },

    #[error("API key not configured")]
    NoApiKey,

    #[error("Base URL not configured")]
    NoBaseUrl,

    #[error("HTTP request failed: {0}")]
    Http(String),

    #[error("Provider returned unexpected response: {0}")]
    ProviderResponse(String),
}
