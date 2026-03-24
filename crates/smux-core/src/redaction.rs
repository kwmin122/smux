//! Transcript redaction for secrets and sensitive data.
//!
//! Applies regex-based rules to replace secrets with [REDACTED].

use regex::Regex;
use serde::{Deserialize, Serialize};

/// A redaction rule with a name and regex pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionRule {
    pub name: String,
    #[serde(skip)]
    compiled: Option<Regex>,
    pub pattern: String,
}

impl RedactionRule {
    pub fn new(name: &str, pattern: &str) -> Self {
        Self {
            name: name.to_string(),
            compiled: Regex::new(pattern).ok(),
            pattern: pattern.to_string(),
        }
    }

    fn regex(&self) -> Option<&Regex> {
        self.compiled.as_ref()
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
pub fn redact_transcript(text: &str, rules: &[RedactionRule]) -> String {
    let mut result = text.to_string();
    for rule in rules {
        if let Some(re) = rule.regex() {
            result = re.replace_all(&result, "[REDACTED]").to_string();
        }
    }
    result
}
