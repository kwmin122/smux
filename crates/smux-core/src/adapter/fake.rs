//! A deterministic test double that replays canned responses.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio_stream::wrappers::ReceiverStream;

use super::{AdapterError, AgentAdapter, AgentEventStream};
use crate::types::{AdapterCapabilities, AgentEvent, SessionConfig, SessionSnapshot, TurnHandle};

/// Deterministic adapter for testing.
///
/// Constructed with a list of canned responses.  Each call to
/// [`send_turn`](AgentAdapter::send_turn) pops the next response and makes it
/// available via [`stream_events`](AgentAdapter::stream_events) as a `Chunk`
/// followed by `TurnComplete`.
pub struct FakeAdapter {
    responses: Vec<String>,
    /// Index of the *next* response to return.
    index: usize,
    started: bool,
    /// Holds the tokio sender for the most recent turn's event stream.
    /// The receiver side is wrapped in `current_rx`.
    current_rx: Arc<Mutex<Option<tokio::sync::mpsc::Receiver<AgentEvent>>>>,
}

impl FakeAdapter {
    /// Create a new `FakeAdapter` with the given canned responses.
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            responses,
            index: 0,
            started: false,
            current_rx: Arc::new(Mutex::new(None)),
        }
    }
}

#[async_trait]
impl AgentAdapter for FakeAdapter {
    fn capabilities(&self) -> AdapterCapabilities {
        AdapterCapabilities {
            persistent_session: false,
            streaming: true,
            native_snapshot: false,
        }
    }

    async fn start_session(&mut self, _config: SessionConfig) -> Result<(), AdapterError> {
        if self.started {
            return Err(AdapterError::AlreadyStarted);
        }
        self.started = true;
        Ok(())
    }

    async fn send_turn(&mut self, _prompt: &str) -> Result<TurnHandle, AdapterError> {
        if !self.started {
            return Err(AdapterError::NotStarted);
        }

        let response = self
            .responses
            .get(self.index)
            .cloned()
            .ok_or_else(|| AdapterError::Other("no more canned responses".into()))?;

        let turn_index = self.index as u64;
        self.index += 1;

        // Create a channel and eagerly send events into it.
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        let chunk = response.clone();
        tokio::spawn(async move {
            let _ = tx.send(AgentEvent::Chunk(chunk.clone())).await;
            let _ = tx.send(AgentEvent::TurnComplete(chunk)).await;
            // tx drops here, closing the stream
        });

        // Store the receiver so `stream_events` can consume it.
        *self.current_rx.lock().unwrap() = Some(rx);

        Ok(TurnHandle { turn_index })
    }

    fn stream_events(&self) -> Result<AgentEventStream<'_>, AdapterError> {
        let rx = self
            .current_rx
            .lock()
            .unwrap()
            .take()
            .ok_or(AdapterError::NoTurns)?;
        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    async fn snapshot_state(&self) -> Result<SessionSnapshot, AdapterError> {
        if !self.started {
            return Err(AdapterError::NotStarted);
        }
        let index_bytes = (self.index as u64).to_le_bytes().to_vec();
        Ok(SessionSnapshot {
            adapter_type: "fake".into(),
            state: index_bytes,
        })
    }

    async fn restore_state(&mut self, snapshot: SessionSnapshot) -> Result<(), AdapterError> {
        if snapshot.adapter_type != "fake" {
            return Err(AdapterError::RestoreFailed(format!(
                "expected adapter_type 'fake', got '{}'",
                snapshot.adapter_type
            )));
        }
        let bytes: [u8; 8] = snapshot
            .state
            .try_into()
            .map_err(|_| AdapterError::RestoreFailed("invalid state length".into()))?;
        self.index = u64::from_le_bytes(bytes) as usize;
        self.started = true;
        Ok(())
    }

    async fn terminate(&mut self) -> Result<(), AdapterError> {
        self.started = false;
        *self.current_rx.lock().unwrap() = None;
        Ok(())
    }
}
