//! Tests for smux config loading, saving, and merging.

use smux_core::config::{DEFAULT_CONFIG_TOML, SmuxConfig};

#[test]
fn parse_valid_config_all_fields() {
    let toml_str = r#"
[defaults]
max_rounds = 20
browser = true
layout = "left"

[agents.planner]
default = "codex"
adapter = "pty"
system_prompt = "You are a coding planner."

[agents.verifier]
default = "claude"
adapter = "pty"
system_prompt = "You are a code reviewer."

[agents.idle]
timeout_secs = 5
max_response_secs = 600

[agents.pty_patterns]
ready = "\\$"
error = "Error:"

[safety]
claude_permission_mode = "manual"
claude_allowed_tools = ["Read", "Edit"]
codex_approval_policy = "auto-approve"
codex_sandbox_mode = "full-write"
max_files_deleted_per_round = 10
max_lines_changed_per_round = 5000

[rewind]
clean_ignored = true

[health]
stuck_timeout = 60
auto_restart = false

[sessions]
cleanup_after_days = 14
"#;

    let config: SmuxConfig = toml::from_str(toml_str).expect("should parse valid config");

    assert_eq!(config.defaults.max_rounds, 20);
    assert!(config.defaults.browser);
    assert_eq!(config.defaults.layout, "left");

    assert_eq!(config.agents.planner.default, "codex");
    assert_eq!(config.agents.planner.adapter, "pty");
    assert_eq!(
        config.agents.planner.system_prompt,
        "You are a coding planner."
    );

    assert_eq!(config.agents.verifier.default, "claude");
    assert_eq!(config.agents.verifier.adapter, "pty");

    assert_eq!(config.agents.idle.timeout_secs, 5);
    assert_eq!(config.agents.idle.max_response_secs, 600);

    assert_eq!(config.agents.pty_patterns.len(), 2);
    assert_eq!(config.agents.pty_patterns.get("ready").unwrap(), "\\$");

    assert_eq!(config.safety.claude_permission_mode, "manual");
    assert_eq!(config.safety.claude_allowed_tools, vec!["Read", "Edit"]);
    assert_eq!(config.safety.codex_approval_policy, "auto-approve");
    assert_eq!(config.safety.codex_sandbox_mode, "full-write");
    assert_eq!(config.safety.max_files_deleted_per_round, 10);
    assert_eq!(config.safety.max_lines_changed_per_round, 5000);

    assert!(config.rewind.clean_ignored);

    assert_eq!(config.health.stuck_timeout, 60);
    assert!(!config.health.auto_restart);

    assert_eq!(config.sessions.cleanup_after_days, 14);
}

#[test]
fn parse_empty_file_returns_defaults() {
    let config: SmuxConfig = toml::from_str("").expect("should parse empty string");

    assert_eq!(config.defaults.max_rounds, 10);
    assert!(!config.defaults.browser);
    assert_eq!(config.defaults.layout, "center");

    assert_eq!(config.agents.planner.default, "claude");
    assert_eq!(config.agents.verifier.default, "codex");
    assert_eq!(config.agents.planner.adapter, "headless");

    assert_eq!(config.agents.idle.timeout_secs, 2);
    assert_eq!(config.agents.idle.max_response_secs, 300);
    assert!(config.agents.pty_patterns.is_empty());

    assert_eq!(config.safety.claude_permission_mode, "auto");
    assert!(config.safety.claude_allowed_tools.is_empty());
    assert_eq!(config.safety.codex_approval_policy, "on-request");
    assert_eq!(config.safety.codex_sandbox_mode, "workspace-write");
    assert_eq!(config.safety.max_files_deleted_per_round, 5);
    assert_eq!(config.safety.max_lines_changed_per_round, 2000);

    assert!(!config.rewind.clean_ignored);

    assert_eq!(config.health.stuck_timeout, 30);
    assert!(config.health.auto_restart);

    assert_eq!(config.sessions.cleanup_after_days, 7);
}

#[test]
fn parse_partial_file_merges_with_defaults() {
    let toml_str = r#"
[defaults]
max_rounds = 42

[safety]
max_files_deleted_per_round = 1
"#;

    let config: SmuxConfig = toml::from_str(toml_str).expect("should parse partial config");

    // Provided fields are overridden.
    assert_eq!(config.defaults.max_rounds, 42);
    assert_eq!(config.safety.max_files_deleted_per_round, 1);

    // Rest keeps defaults.
    assert!(!config.defaults.browser);
    assert_eq!(config.defaults.layout, "center");
    assert_eq!(config.agents.planner.default, "claude");
    assert_eq!(config.safety.max_lines_changed_per_round, 2000);
    assert_eq!(config.health.stuck_timeout, 30);
    assert_eq!(config.sessions.cleanup_after_days, 7);
}

#[test]
fn save_default_creates_reparseable_toml() {
    let dir = tempfile::tempdir().expect("should create temp dir");
    let path = dir.path().join("config.toml");

    SmuxConfig::save_default(&path).expect("should save default config");

    let content = std::fs::read_to_string(&path).expect("should read saved file");
    assert!(!content.is_empty(), "saved file should not be empty");

    // Re-parse and verify it matches defaults.
    let config: SmuxConfig = toml::from_str(&content).expect("saved config should be valid TOML");
    let defaults = SmuxConfig::default();

    assert_eq!(config.defaults.max_rounds, defaults.defaults.max_rounds);
    assert_eq!(config.defaults.browser, defaults.defaults.browser);
    assert_eq!(config.defaults.layout, defaults.defaults.layout);
    assert_eq!(
        config.agents.planner.default,
        defaults.agents.planner.default
    );
    assert_eq!(
        config.agents.verifier.default,
        defaults.agents.verifier.default
    );
    assert_eq!(config.safety.max_files_deleted_per_round, 5);
    assert_eq!(config.health.stuck_timeout, 30);
}

#[test]
fn load_nonexistent_file_returns_defaults() {
    let dir = tempfile::tempdir().expect("should create temp dir");
    let path = dir.path().join("does-not-exist.toml");

    let config = SmuxConfig::load_from(&path).expect("should not error on missing file");

    assert_eq!(config.defaults.max_rounds, 10);
    assert_eq!(config.agents.planner.default, "claude");
    assert_eq!(config.agents.verifier.default, "codex");
}

#[test]
fn default_config_toml_constant_is_valid() {
    let config: SmuxConfig =
        toml::from_str(DEFAULT_CONFIG_TOML).expect("DEFAULT_CONFIG_TOML should be valid");

    assert_eq!(config.defaults.max_rounds, 10);
    assert_eq!(config.agents.planner.default, "claude");
    assert_eq!(config.agents.verifier.default, "codex");
}

#[test]
fn cli_flag_overrides_config_value() {
    // Simulate: config says max_rounds=10, planner=claude, verifier=codex.
    // CLI passes --max-rounds 3 --planner codex.
    // Result: max_rounds=3, planner=codex, verifier=codex (from config).

    let config = SmuxConfig::default();
    assert_eq!(config.defaults.max_rounds, 10);
    assert_eq!(config.agents.planner.default, "claude");
    assert_eq!(config.agents.verifier.default, "codex");

    // Helper that mirrors the CLI's resolution logic: flag overrides config.
    fn resolve(flag: Option<String>, config_val: &str) -> String {
        flag.unwrap_or_else(|| config_val.to_string())
    }
    fn resolve_u32(flag: Option<u32>, config_val: u32) -> u32 {
        flag.unwrap_or(config_val)
    }

    // Simulate CLI flags: --planner codex --max-rounds 3 (no --verifier).
    let resolved_planner = resolve(Some("codex".to_string()), &config.agents.planner.default);
    let resolved_verifier = resolve(None, &config.agents.verifier.default);
    let resolved_max_rounds = resolve_u32(Some(3), config.defaults.max_rounds);

    assert_eq!(resolved_planner, "codex");
    assert_eq!(resolved_verifier, "codex");
    assert_eq!(resolved_max_rounds, 3);
}

#[test]
fn save_default_creates_parent_directory() {
    let dir = tempfile::tempdir().expect("should create temp dir");
    let path = dir.path().join("nested").join("dir").join("config.toml");

    SmuxConfig::save_default(&path).expect("should create nested dirs and save");
    assert!(path.exists());

    let config = SmuxConfig::load_from(&path).expect("should load from newly created nested path");
    assert_eq!(config.defaults.max_rounds, 10);
}

#[test]
fn pty_patterns_round_trip() {
    let toml_str = r#"
[agents.pty_patterns]
ready = "\\$\\s*$"
error = "^Error:"
prompt = ">>>"
"#;

    let config: SmuxConfig = toml::from_str(toml_str).expect("should parse pty_patterns");
    assert_eq!(config.agents.pty_patterns.len(), 3);
    assert_eq!(config.agents.pty_patterns.get("ready").unwrap(), "\\$\\s*$");

    // Round-trip: serialize back and re-parse.
    let serialized = toml::to_string(&config).expect("should serialize");
    let reparsed: SmuxConfig = toml::from_str(&serialized).expect("should re-parse");
    assert_eq!(reparsed.agents.pty_patterns.len(), 3);
}
