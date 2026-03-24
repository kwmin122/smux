//! Transcript redaction for secrets and sensitive data.
//!
//! Applies regex-based rules to replace secrets with [REDACTED].

use regex::Regex;
use serde::{Deserialize, Serialize};

/// A redaction rule with a name and regex pattern.
/// After deserialization, call `ensure_compiled()` or use `redact_transcript()`
/// which compiles lazily.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionRule {
    pub name: String,
    pub pattern: String,
}

impl RedactionRule {
    pub fn new(name: &str, pattern: &str) -> Self {
        Self {
            name: name.to_string(),
            pattern: pattern.to_string(),
        }
    }

    /// Default redaction rules covering common secret patterns.
    pub fn default_rules() -> Vec<Self> {
        vec![
            Self::new("openai_key", r"sk-[A-Za-z0-9]{20,}"),
            Self::new("anthropic_key", r"sk-ant-api\d{2}-[A-Za-z0-9\-_]{20,}"),
            Self::new("github_pat", r"gh[pos]_[A-Za-z0-9]{20,}"),
            Self::new("aws_key", r"AKIA[A-Z0-9]{16}"),
            Self::new("bearer", r"Bearer\s+[A-Za-z0-9\-._~+/]+=*"),
            Self::new("basic_auth", r"Basic\s+[A-Za-z0-9+/]{16,}={0,2}"),
            Self::new(
                "connection_string",
                r"((?:postgres|mysql|mongodb|redis)://[^:]+:)[^\s@]+(@)",
            ),
            Self::new("slack_token", r"xox[bps]-[A-Za-z0-9\-]{20,}"),
            Self::new("npm_token", r"npm_[A-Za-z0-9]{20,}"),
            Self::new(
                "generic_secret",
                r"(?i)(?:secret|password|token|api_key|private_key)\s*[=:]\s*\S{8,}",
            ),
        ]
    }
}

/// Apply redaction rules to a transcript, replacing matches with [REDACTED].
/// Compiles each regex on every call. For hot paths, pre-compile with `compile_rules()`.
pub fn redact_transcript(text: &str, rules: &[RedactionRule]) -> String {
    let mut result = text.to_string();
    for rule in rules {
        if let Ok(re) = Regex::new(&rule.pattern) {
            result = re.replace_all(&result, "[REDACTED]").to_string();
        }
    }
    result
}

/// Pre-compile rules for repeated use. Returns (name, compiled regex) pairs.
pub fn compile_rules(rules: &[RedactionRule]) -> Vec<(String, Regex)> {
    rules
        .iter()
        .filter_map(|r| Regex::new(&r.pattern).ok().map(|re| (r.name.clone(), re)))
        .collect()
}

/// Apply pre-compiled rules (faster for hot paths).
pub fn redact_with_compiled(text: &str, compiled: &[(String, Regex)]) -> String {
    let mut result = text.to_string();
    for (_, re) in compiled {
        result = re.replace_all(&result, "[REDACTED]").to_string();
    }
    result
}
