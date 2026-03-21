//! Health monitoring for agent adapters.
//!
//! Tracks the liveness of an agent based on event timestamps and classifies
//! its state as [`AgentHealth::Healthy`], [`Slow`](AgentHealth::Slow),
//! [`Stuck`](AgentHealth::Stuck), or [`Dead`](AgentHealth::Dead).

use std::time::{Duration, Instant};

/// Agent health state.
#[derive(Debug, Clone, PartialEq)]
pub enum AgentHealth {
    /// Agent is responsive — events arriving within the warning threshold.
    Healthy,
    /// No output for longer than `warning_threshold` but less than
    /// `stuck_timeout`.
    Slow { since: Instant },
    /// No output for longer than `stuck_timeout`.
    Stuck { since: Instant },
    /// The underlying process exited.
    Dead { exit_code: Option<i32> },
}

/// Configuration for the [`HealthMonitor`].
#[derive(Debug, Clone)]
pub struct HealthConfig {
    /// Duration after which an agent is considered stuck (default 30 s).
    pub stuck_timeout: Duration,
    /// Duration after which a warning is issued (default 15 s).
    pub warning_threshold: Duration,
    /// Whether to automatically restart a stuck agent (default `true`).
    pub auto_restart: bool,
}

impl Default for HealthConfig {
    fn default() -> Self {
        Self {
            stuck_timeout: Duration::from_secs(30),
            warning_threshold: Duration::from_secs(15),
            auto_restart: true,
        }
    }
}

/// Monitors agent health based on event timestamps.
pub struct HealthMonitor {
    config: HealthConfig,
    last_event: Instant,
    state: AgentHealth,
}

impl HealthMonitor {
    /// Create a new monitor with the given configuration.
    pub fn new(config: HealthConfig) -> Self {
        Self {
            config,
            last_event: Instant::now(),
            state: AgentHealth::Healthy,
        }
    }

    /// Record that an event was received from the agent.
    pub fn record_event(&mut self) {
        self.last_event = Instant::now();
        if !matches!(self.state, AgentHealth::Dead { .. }) {
            self.state = AgentHealth::Healthy;
        }
    }

    /// Check the current health. Call this periodically.
    ///
    /// Updates and returns the internal state based on elapsed time since the
    /// last event.
    pub fn check(&mut self) -> &AgentHealth {
        // Dead is terminal — don't transition out of it.
        if matches!(self.state, AgentHealth::Dead { .. }) {
            return &self.state;
        }

        let elapsed = self.last_event.elapsed();

        if elapsed >= self.config.stuck_timeout {
            let since = match self.state {
                AgentHealth::Stuck { since } => since,
                _ => self.last_event,
            };
            self.state = AgentHealth::Stuck { since };
        } else if elapsed >= self.config.warning_threshold {
            let since = match self.state {
                AgentHealth::Slow { since } => since,
                _ => self.last_event,
            };
            self.state = AgentHealth::Slow { since };
        } else {
            self.state = AgentHealth::Healthy;
        }

        &self.state
    }

    /// Reset the monitor to `Healthy` (e.g. after a restart).
    pub fn reset(&mut self) {
        self.last_event = Instant::now();
        self.state = AgentHealth::Healthy;
    }

    /// Mark the agent as dead with the given exit code.
    pub fn mark_dead(&mut self, exit_code: Option<i32>) {
        self.state = AgentHealth::Dead { exit_code };
    }

    /// Return a reference to the current health state (without re-checking).
    pub fn state(&self) -> &AgentHealth {
        &self.state
    }

    /// Return the health configuration.
    pub fn config(&self) -> &HealthConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_values() {
        let cfg = HealthConfig::default();
        assert_eq!(cfg.stuck_timeout, Duration::from_secs(30));
        assert_eq!(cfg.warning_threshold, Duration::from_secs(15));
        assert!(cfg.auto_restart);
    }
}
