//! smux-core — session model, adapter traits, and orchestration logic for smux.

pub mod adapter;
pub mod context;
pub mod git_worktree;
pub mod orchestrator;
pub mod session_store;
pub mod stop;
pub mod types;

/// Top-level error type for smux-core operations that are not adapter-specific.
#[derive(Debug, thiserror::Error)]
pub enum SmuxError {
    /// A git operation failed.
    #[error("git error: {0}")]
    Git(String),
    /// A storage (filesystem / serialization) operation failed.
    #[error("storage error: {0}")]
    Storage(String),
}
