//! Tests for transcript redaction.

use smux_core::redaction::{RedactionRule, redact_transcript};

#[test]
fn redact_openai_key() {
    let text = "export OPENAI_API_KEY=sk-abc123def456ghi789jkl012mno345";
    let result = redact_transcript(text, &RedactionRule::default_rules());
    assert!(result.contains("[REDACTED]"));
    assert!(!result.contains("sk-abc123"));
}

#[test]
fn redact_github_token() {
    let text = "git clone https://ghp_abcdefghijklmnopqrstuvwx@github.com/user/repo";
    let result = redact_transcript(text, &RedactionRule::default_rules());
    assert!(result.contains("[REDACTED]"));
}

#[test]
fn redact_bearer_token() {
    let text = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.sig";
    let result = redact_transcript(text, &RedactionRule::default_rules());
    assert!(result.contains("[REDACTED]"));
}

#[test]
fn preserve_normal_text() {
    let text = "cargo build --release && echo done";
    let result = redact_transcript(text, &RedactionRule::default_rules());
    assert_eq!(result, text);
}

#[test]
fn redact_anthropic_key() {
    let text = "ANTHROPIC_API_KEY=sk-ant-api03-abcdefghijklmnopqrstuv";
    let result = redact_transcript(text, &RedactionRule::default_rules());
    assert!(result.contains("[REDACTED]"));
}

#[test]
fn redact_connection_string() {
    let text = "postgres://admin:supersecret@db.example.com:5432/mydb";
    let result = redact_transcript(text, &RedactionRule::default_rules());
    assert!(result.contains("[REDACTED]"));
    assert!(!result.contains("supersecret"));
}

#[test]
fn custom_redaction_rule() {
    let rules = vec![RedactionRule::new("custom", r"SECRET_\w+=\S+")];
    let text = "SECRET_TOKEN=abc123 and normal text";
    let result = redact_transcript(text, &rules);
    assert!(result.contains("[REDACTED]"));
    assert!(result.contains("normal text"));
}
