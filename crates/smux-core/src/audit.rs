//! Audit logging for session events.
//!
//! Records stage transitions, approvals, verifier findings, and blocked commands
//! for compliance and debugging.

use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

/// An auditable event in a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEvent {
    StageTransition {
        from: String,
        to: String,
    },
    Approval {
        stage: String,
        approved_by: String,
    },
    VerifierFinding {
        verifier: String,
        verdict: String,
        reason: String,
    },
    CommandBlocked {
        command: String,
        reason: String,
    },
    SessionStarted {
        task: String,
        agents: Vec<String>,
    },
    SessionCompleted {
        summary: String,
    },
    Retry {
        stage: String,
        reason: String,
    },
}

/// A timestamped audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRecord {
    pub timestamp: u64,
    pub event: AuditEvent,
}

/// Collects audit records for a session.
pub struct AuditSink {
    records: Vec<AuditRecord>,
}

impl AuditSink {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    pub fn record(&mut self, event: AuditEvent) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.records.push(AuditRecord { timestamp, event });
    }

    pub fn records(&self) -> &[AuditRecord] {
        &self.records
    }
}

impl Default for AuditSink {
    fn default() -> Self {
        Self::new()
    }
}
