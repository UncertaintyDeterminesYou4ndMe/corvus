use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ModelFamily {
    Opus,
    Sonnet,
    Haiku,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: &'static str,
    pub family: ModelFamily,
    pub aliases: &'static [&'static str],
    pub supports_thinking: bool,
    pub supports_1m_context: bool,
    pub input_cost_per_mtok: f64,
    pub output_cost_per_mtok: f64,
    pub cache_read_cost_per_mtok: f64,
    pub cache_write_cost_per_mtok: f64,
}

pub const KNOWN_MODELS: &[ModelInfo] = &[
    // Opus 4
    ModelInfo {
        id: "claude-opus-4-20250514",
        family: ModelFamily::Opus,
        aliases: &["claude-opus-4", "opus-4", "opus", "opus[1m]"],
        supports_thinking: true,
        supports_1m_context: true,
        input_cost_per_mtok: 15.0,
        output_cost_per_mtok: 75.0,
        cache_read_cost_per_mtok: 1.50,
        cache_write_cost_per_mtok: 18.75,
    },
    // Sonnet 4.5
    ModelInfo {
        id: "claude-sonnet-4-5-20250929",
        family: ModelFamily::Sonnet,
        aliases: &["claude-sonnet-4-5", "sonnet-4-5", "sonnet", "sonnet[1m]"],
        supports_thinking: true,
        supports_1m_context: true,
        input_cost_per_mtok: 3.0,
        output_cost_per_mtok: 15.0,
        cache_read_cost_per_mtok: 0.30,
        cache_write_cost_per_mtok: 3.75,
    },
    // Sonnet 4.6
    ModelInfo {
        id: "claude-sonnet-4-6-20260220",
        family: ModelFamily::Sonnet,
        aliases: &["claude-sonnet-4-6", "sonnet-4-6"],
        supports_thinking: true,
        supports_1m_context: true,
        input_cost_per_mtok: 3.0,
        output_cost_per_mtok: 15.0,
        cache_read_cost_per_mtok: 0.30,
        cache_write_cost_per_mtok: 3.75,
    },
    // Opus 4.6
    ModelInfo {
        id: "claude-opus-4-6-20260320",
        family: ModelFamily::Opus,
        aliases: &["claude-opus-4-6", "opus-4-6"],
        supports_thinking: true,
        supports_1m_context: true,
        input_cost_per_mtok: 15.0,
        output_cost_per_mtok: 75.0,
        cache_read_cost_per_mtok: 1.50,
        cache_write_cost_per_mtok: 18.75,
    },
    // Haiku 4.5
    ModelInfo {
        id: "claude-haiku-4-5-20251001",
        family: ModelFamily::Haiku,
        aliases: &["claude-haiku-4-5", "haiku-4-5", "haiku"],
        supports_thinking: false,
        supports_1m_context: false,
        input_cost_per_mtok: 0.80,
        output_cost_per_mtok: 4.0,
        cache_read_cost_per_mtok: 0.08,
        cache_write_cost_per_mtok: 1.0,
    },
    // Claude 3.5 Sonnet (legacy, widely used by relay services)
    ModelInfo {
        id: "claude-3-5-sonnet-20241022",
        family: ModelFamily::Sonnet,
        aliases: &["claude-3-5-sonnet", "claude-3.5-sonnet"],
        supports_thinking: false,
        supports_1m_context: false,
        input_cost_per_mtok: 3.0,
        output_cost_per_mtok: 15.0,
        cache_read_cost_per_mtok: 0.30,
        cache_write_cost_per_mtok: 3.75,
    },
    // Claude 3 Opus (legacy)
    ModelInfo {
        id: "claude-3-opus-20240229",
        family: ModelFamily::Opus,
        aliases: &["claude-3-opus"],
        supports_thinking: false,
        supports_1m_context: false,
        input_cost_per_mtok: 15.0,
        output_cost_per_mtok: 75.0,
        cache_read_cost_per_mtok: 1.50,
        cache_write_cost_per_mtok: 18.75,
    },
    // Claude 3 Haiku (legacy)
    ModelInfo {
        id: "claude-3-haiku-20240307",
        family: ModelFamily::Haiku,
        aliases: &["claude-3-haiku"],
        supports_thinking: false,
        supports_1m_context: false,
        input_cost_per_mtok: 0.25,
        output_cost_per_mtok: 1.25,
        cache_read_cost_per_mtok: 0.03,
        cache_write_cost_per_mtok: 0.30,
    },
];

/// Look up a model by exact ID or alias.
pub fn lookup(model_id: &str) -> Option<&'static ModelInfo> {
    KNOWN_MODELS.iter().find(|m| {
        m.id == model_id || m.aliases.iter().any(|a| *a == model_id)
    })
}

/// Check if a model ID has a suffix that relay services typically don't support.
/// Returns the problematic suffix if found.
pub fn has_problematic_suffix(model_id: &str) -> Option<&'static str> {
    const PROBLEMATIC_SUFFIXES: &[&str] = &[
        "-thinking",
        "-extended-thinking",
    ];
    for suffix in PROBLEMATIC_SUFFIXES {
        if model_id.ends_with(suffix) {
            return Some(suffix);
        }
    }
    None
}

/// Check if a model ID uses a short alias that may not be recognized by relay services.
pub fn is_short_alias(model_id: &str) -> bool {
    // Short aliases don't contain date stamps (YYYYMMDD)
    let has_date = model_id.chars().filter(|c| c.is_ascii_digit()).count() >= 8;
    !has_date && lookup(model_id).is_some()
}

/// Get the canonical (full) model ID for an alias.
pub fn canonicalize(model_id: &str) -> Option<&'static str> {
    lookup(model_id).map(|m| m.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_exact_id() {
        let m = lookup("claude-opus-4-20250514").unwrap();
        assert_eq!(m.family, ModelFamily::Opus);
    }

    #[test]
    fn test_lookup_alias() {
        let m = lookup("sonnet").unwrap();
        assert_eq!(m.family, ModelFamily::Sonnet);
    }

    #[test]
    fn test_lookup_unknown() {
        assert!(lookup("gpt-4o").is_none());
    }

    #[test]
    fn test_problematic_suffix() {
        assert_eq!(
            has_problematic_suffix("claude-opus-4-20250514-thinking"),
            Some("-thinking")
        );
        assert!(has_problematic_suffix("claude-opus-4-20250514").is_none());
    }

    #[test]
    fn test_short_alias() {
        assert!(is_short_alias("sonnet"));
        assert!(is_short_alias("claude-sonnet-4-6"));
        assert!(!is_short_alias("claude-sonnet-4-5-20250929"));
    }

    #[test]
    fn test_canonicalize() {
        assert_eq!(
            canonicalize("opus"),
            Some("claude-opus-4-20250514")
        );
    }
}
