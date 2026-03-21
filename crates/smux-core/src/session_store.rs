//! Session and round persistence for smux.
//!
//! Storage layout:
//! ```text
//! ~/.smux/sessions/<session-id>/
//!   session.json
//!   rounds/
//!     round-001.json
//!     round-002.json
//! ```

use std::path::PathBuf;

use crate::SmuxError;
use crate::types::{RoundSnapshot, SessionMeta};

/// Persistent storage for a single smux session.
pub struct SessionStore {
    /// Base directory: `~/.smux/sessions/<session-id>/`
    base_dir: PathBuf,
}

impl SessionStore {
    /// Create a new store for the given session ID.
    ///
    /// Uses `~/.smux/sessions/<session-id>/` as the base directory.
    pub fn new(session_id: &str) -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            base_dir: home.join(".smux/sessions").join(session_id),
        }
    }

    /// Create a store rooted at a custom base directory (useful for testing).
    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Directory that holds round JSON files.
    fn rounds_dir(&self) -> PathBuf {
        self.base_dir.join("rounds")
    }

    /// Path to a specific round file.
    fn round_path(&self, round: u32) -> PathBuf {
        self.rounds_dir().join(format!("round-{round:03}.json"))
    }

    /// Path to the session metadata file.
    fn session_meta_path(&self) -> PathBuf {
        self.base_dir.join("session.json")
    }

    /// Save a round snapshot to disk.
    pub fn save_round(&self, snapshot: &RoundSnapshot) -> Result<(), SmuxError> {
        tracing::debug!(round = snapshot.round, path = %self.round_path(snapshot.round).display(), "saving round snapshot");
        std::fs::create_dir_all(self.rounds_dir()).map_err(|e| {
            SmuxError::Storage(format!(
                "failed to create rounds dir {}: {e}",
                self.rounds_dir().display()
            ))
        })?;

        let json = serde_json::to_string_pretty(snapshot)
            .map_err(|e| SmuxError::Storage(format!("failed to serialize round snapshot: {e}")))?;

        std::fs::write(self.round_path(snapshot.round), json)
            .map_err(|e| SmuxError::Storage(format!("failed to write round file: {e}")))?;

        Ok(())
    }

    /// Load a specific round snapshot from disk.
    pub fn load_round(&self, round: u32) -> Result<RoundSnapshot, SmuxError> {
        tracing::debug!(round, "loading round snapshot");
        let path = self.round_path(round);
        let data = std::fs::read_to_string(&path).map_err(|e| {
            SmuxError::Storage(format!("failed to read round file {}: {e}", path.display()))
        })?;

        serde_json::from_str(&data)
            .map_err(|e| SmuxError::Storage(format!("failed to parse round file: {e}")))
    }

    /// List all saved round numbers, sorted ascending.
    pub fn list_rounds(&self) -> Result<Vec<u32>, SmuxError> {
        let dir = self.rounds_dir();
        if !dir.exists() {
            return Ok(Vec::new());
        }

        let mut rounds = Vec::new();
        let entries = std::fs::read_dir(&dir)
            .map_err(|e| SmuxError::Storage(format!("failed to read rounds dir: {e}")))?;

        for entry in entries {
            let entry =
                entry.map_err(|e| SmuxError::Storage(format!("failed to read dir entry: {e}")))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            // Parse "round-001.json" -> 1
            if let Some(num_str) = name
                .strip_prefix("round-")
                .and_then(|s| s.strip_suffix(".json"))
                && let Ok(n) = num_str.parse::<u32>()
            {
                rounds.push(n);
            }
        }

        rounds.sort();
        Ok(rounds)
    }

    /// Save session metadata to disk.
    pub fn save_session_meta(&self, meta: &SessionMeta) -> Result<(), SmuxError> {
        tracing::debug!(path = %self.session_meta_path().display(), "saving session metadata");
        std::fs::create_dir_all(&self.base_dir).map_err(|e| {
            SmuxError::Storage(format!(
                "failed to create session dir {}: {e}",
                self.base_dir.display()
            ))
        })?;

        let json = serde_json::to_string_pretty(meta)
            .map_err(|e| SmuxError::Storage(format!("failed to serialize session meta: {e}")))?;

        std::fs::write(self.session_meta_path(), json)
            .map_err(|e| SmuxError::Storage(format!("failed to write session meta: {e}")))?;

        Ok(())
    }

    /// Load session metadata from disk.
    pub fn load_session_meta(&self) -> Result<SessionMeta, SmuxError> {
        tracing::debug!(path = %self.session_meta_path().display(), "loading session metadata");
        let path = self.session_meta_path();
        let data = std::fs::read_to_string(&path).map_err(|e| {
            SmuxError::Storage(format!(
                "failed to read session meta {}: {e}",
                path.display()
            ))
        })?;

        serde_json::from_str(&data)
            .map_err(|e| SmuxError::Storage(format!("failed to parse session meta: {e}")))
    }
}
