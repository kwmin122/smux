//! Agent adapter trait and implementations.

pub mod fake;

use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use crate::types::{AdapterCapabilities, AgentEvent, SessionConfig, SessionSnapshot, TurnHandle};

/// A boxed, pinned, Send stream of [`AgentEvent`]s.
pub type AgentEventStream<'a> = Pin<Box<dyn Stream<Item = AgentEvent> + Send + 'a>>;

/// Session-oriented interface to an AI coding agent.
///
/// Each adapter wraps a specific agent CLI (Claude, Codex, etc.).
/// The lifecycle is:
///
/// 1. `start_session` — initialise with config
/// 2. `send_turn` / `stream_events` — interact in a loop
/// 3. `snapshot_state` / `restore_state` — checkpoint and rewind
/// 4. `terminate` — clean up
///
/// The trait is object-safe so it can be stored as `Box<dyn AgentAdapter>`.
#[async_trait]
pub trait AgentAdapter: Send + Sync {
    /// Declare what this adapter supports.
    fn capabilities(&self) -> AdapterCapabilities;

    /// Initialise (or re-initialise) the session.
    async fn start_session(&mut self, config: SessionConfig) -> Result<(), AdapterError>;

    /// Submit a user prompt and get back a handle to the in-progress turn.
    async fn send_turn(&mut self, prompt: &str) -> Result<TurnHandle, AdapterError>;

    /// Return a stream of events for the most recently submitted turn.
    fn stream_events(&self) -> AgentEventStream<'_>;

    /// Capture the adapter's internal state as an opaque snapshot.
    async fn snapshot_state(&self) -> Result<SessionSnapshot, AdapterError>;

    /// Restore the adapter to a previously captured snapshot.
    async fn restore_state(&mut self, snapshot: SessionSnapshot) -> Result<(), AdapterError>;

    /// Terminate the session and release resources.
    async fn terminate(&mut self) -> Result<(), AdapterError>;
}

/// Errors produced by adapter operations.
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    #[error("session not started")]
    NotStarted,
    #[error("session already started")]
    AlreadyStarted,
    #[error("no turns sent yet")]
    NoTurns,
    #[error("snapshot restore failed: {0}")]
    RestoreFailed(String),
    #[error("adapter I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}
