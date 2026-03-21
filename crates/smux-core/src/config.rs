//! Configuration management for smux.
//!
//! Loads TOML config from `~/.smux/config.toml`, falling back to sensible
//! defaults when the file is absent or partially populated.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::SmuxError;

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

/// Root configuration struct deserialized from `~/.smux/config.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SmuxConfig {
    pub defaults: DefaultsConfig,
    pub agents: AgentsConfig,
    pub safety: SafetyConfig,
    pub rewind: RewindConfig,
    pub health: HealthConfig,
    pub sessions: SessionsConfig,
}

// ---------------------------------------------------------------------------
// Sections
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DefaultsConfig {
    pub max_rounds: u32,
    pub browser: bool,
    pub layout: String,
}

impl Default for DefaultsConfig {
    fn default() -> Self {
        Self {
            max_rounds: 10,
            browser: false,
            layout: "center".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentsConfig {
    pub planner: AgentConfig,
    pub verifier: AgentConfig,
    pub pty_patterns: HashMap<String, String>,
    pub idle: IdleConfig,
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            planner: AgentConfig {
                default: "claude".to_string(),
                adapter: "headless".to_string(),
                system_prompt: String::new(),
            },
            verifier: AgentConfig {
                default: "codex".to_string(),
                adapter: "headless".to_string(),
                system_prompt: String::new(),
            },
            pty_patterns: HashMap::new(),
            idle: IdleConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    pub default: String,
    pub adapter: String,
    pub system_prompt: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            default: "claude".to_string(),
            adapter: "headless".to_string(),
            system_prompt: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct IdleConfig {
    pub timeout_secs: u64,
    pub max_response_secs: u64,
}

impl Default for IdleConfig {
    fn default() -> Self {
        Self {
            timeout_secs: 2,
            max_response_secs: 300,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SafetyConfig {
    pub claude_permission_mode: String,
    pub claude_allowed_tools: Vec<String>,
    pub codex_approval_policy: String,
    pub codex_sandbox_mode: String,
    pub max_files_deleted_per_round: usize,
    pub max_lines_changed_per_round: usize,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            claude_permission_mode: "auto".to_string(),
            claude_allowed_tools: Vec::new(),
            codex_approval_policy: "on-request".to_string(),
            codex_sandbox_mode: "workspace-write".to_string(),
            max_files_deleted_per_round: 5,
            max_lines_changed_per_round: 2000,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RewindConfig {
    pub clean_ignored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct HealthConfig {
    pub stuck_timeout: u64,
    pub auto_restart: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            stuck_timeout: 30,
            auto_restart: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionsConfig {
    pub cleanup_after_days: u64,
}

impl Default for SessionsConfig {
    fn default() -> Self {
        Self {
            cleanup_after_days: 7,
        }
    }
}

// ---------------------------------------------------------------------------
// Loading / saving
// ---------------------------------------------------------------------------

/// Default path for the smux config file: `~/.smux/config.toml`.
pub fn default_config_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".smux")
        .join("config.toml")
}

impl SmuxConfig {
    /// Load config from `~/.smux/config.toml`, or return defaults if not found.
    pub fn load() -> Result<Self, SmuxError> {
        let path = default_config_path();
        Self::load_from(&path)
    }

    /// Load from a specific path.
    ///
    /// Returns defaults (not an error) when the file does not exist.
    pub fn load_from(path: &Path) -> Result<Self, SmuxError> {
        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(e) => {
                return Err(SmuxError::Storage(format!(
                    "failed to read config at {}: {e}",
                    path.display()
                )));
            }
        };

        toml::from_str(&content).map_err(|e| {
            SmuxError::Storage(format!("failed to parse config at {}: {e}", path.display()))
        })
    }

    /// Save the default config (with comments) to a path.
    ///
    /// Used by `smux init` to create a well-documented starter config.
    pub fn save_default(path: &Path) -> Result<(), SmuxError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                SmuxError::Storage(format!(
                    "failed to create config directory {}: {e}",
                    parent.display()
                ))
            })?;
        }

        std::fs::write(path, DEFAULT_CONFIG_TOML).map_err(|e| {
            SmuxError::Storage(format!("failed to write config to {}: {e}", path.display()))
        })
    }
}

/// Well-commented default config written by `smux init`.
pub const DEFAULT_CONFIG_TOML: &str = r#"# smux configuration
# Location: ~/.smux/config.toml

[defaults]
max_rounds = 10      # maximum planner-verifier rounds per session
browser = false      # open browser UI on session start
layout = "center"    # tmux pane layout: center, left, right

[agents.planner]
default = "claude"         # default planner provider
adapter = "headless"       # adapter type: headless or pty
system_prompt = ""         # optional system prompt override

[agents.verifier]
default = "codex"          # default verifier provider
adapter = "headless"       # adapter type: headless or pty
system_prompt = ""         # optional system prompt override

[agents.idle]
timeout_secs = 2           # seconds of idle before considering agent done
max_response_secs = 300    # hard timeout for a single agent response

# [agents.pty_patterns]
# ready = "\\$"            # regex pattern to detect agent is ready

[safety]
claude_permission_mode = "auto"      # auto, manual, or deny
claude_allowed_tools = []            # list of allowed tool names
codex_approval_policy = "on-request" # on-request or auto-approve
codex_sandbox_mode = "workspace-write"  # workspace-write, full-write, or read-only
max_files_deleted_per_round = 5      # safety limit per round
max_lines_changed_per_round = 2000   # safety limit per round

[rewind]
clean_ignored = false    # also clean gitignored files on rewind

[health]
stuck_timeout = 30       # seconds before declaring an agent stuck
auto_restart = true      # automatically restart stuck agents

[sessions]
cleanup_after_days = 7   # auto-delete completed sessions after N days
"#;
