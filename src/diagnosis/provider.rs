use serde::Serialize;
use std::fmt;

use crate::config::EnvConfig;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ProviderType {
    AnthropicDirect,
    AwsBedrock,
    Apimart,
    NewApiOneApi,
    GoogleVertex,
    CustomRelay(String),
}

impl fmt::Display for ProviderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AnthropicDirect => write!(f, "Anthropic Direct API"),
            Self::AwsBedrock => write!(f, "AWS Bedrock"),
            Self::Apimart => write!(f, "apimart relay service"),
            Self::NewApiOneApi => write!(f, "NewAPI/OneAPI relay service"),
            Self::GoogleVertex => write!(f, "Google Vertex AI"),
            Self::CustomRelay(domain) => write!(f, "custom relay ({})", domain),
        }
    }
}

impl ProviderType {
    pub fn is_relay(&self) -> bool {
        matches!(self, Self::Apimart | Self::NewApiOneApi | Self::CustomRelay(_))
    }
}

/// Detect provider type from environment configuration.
pub fn detect(env: &EnvConfig) -> ProviderType {
    // Bedrock has its own env var
    if env.bedrock_base_url.is_some() || env.aws_bearer_token_bedrock.is_some() {
        return ProviderType::AwsBedrock;
    }

    let Some(ref url) = env.base_url else {
        return ProviderType::AnthropicDirect;
    };

    let url_lower = url.to_lowercase();

    if url_lower.contains("api.anthropic.com") {
        ProviderType::AnthropicDirect
    } else if url_lower.contains("bedrock") || url_lower.contains("amazonaws.com") {
        ProviderType::AwsBedrock
    } else if url_lower.contains("apimart") {
        ProviderType::Apimart
    } else if url_lower.contains("one-api") || url_lower.contains("oneapi") || url_lower.contains("new-api") || url_lower.contains("newapi") || url_lower.contains("api2d") {
        ProviderType::NewApiOneApi
    } else if url_lower.contains("vertex") || url_lower.contains("googleapis.com") {
        ProviderType::GoogleVertex
    } else {
        // Extract domain for display
        let domain = url.trim_start_matches("https://")
            .trim_start_matches("http://")
            .split('/')
            .next()
            .unwrap_or("unknown")
            .to_string();
        ProviderType::CustomRelay(domain)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_with_url(url: &str) -> EnvConfig {
        EnvConfig {
            base_url: Some(url.to_string()),
            ..Default::default()
        }
    }

    #[test]
    fn test_detect_anthropic_direct() {
        assert_eq!(detect(&EnvConfig::default()), ProviderType::AnthropicDirect);
        assert_eq!(detect(&env_with_url("https://api.anthropic.com")), ProviderType::AnthropicDirect);
    }

    #[test]
    fn test_detect_bedrock() {
        let env = EnvConfig {
            bedrock_base_url: Some("https://bedrock.us-east-1.amazonaws.com".into()),
            ..Default::default()
        };
        assert_eq!(detect(&env), ProviderType::AwsBedrock);
    }

    #[test]
    fn test_detect_apimart() {
        assert_eq!(detect(&env_with_url("https://api.apimart.ai")), ProviderType::Apimart);
    }

    #[test]
    fn test_detect_oneapi() {
        assert_eq!(detect(&env_with_url("https://my-one-api.com/v1")), ProviderType::NewApiOneApi);
    }

    #[test]
    fn test_detect_custom_relay() {
        assert_eq!(
            detect(&env_with_url("https://my-custom-proxy.com/v1")),
            ProviderType::CustomRelay("my-custom-proxy.com".into())
        );
    }
}
