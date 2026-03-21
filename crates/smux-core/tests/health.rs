//! Tests for `smux_core::health::HealthMonitor`.

use std::time::Duration;

use smux_core::health::{AgentHealth, HealthConfig, HealthMonitor};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn config_fast() -> HealthConfig {
    HealthConfig {
        stuck_timeout: Duration::from_millis(100),
        warning_threshold: Duration::from_millis(50),
        auto_restart: true,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Regular events keep the state Healthy.
#[test]
fn healthy_when_events_arrive() {
    let mut mon = HealthMonitor::new(config_fast());

    // Simulate several events arriving in quick succession.
    for _ in 0..5 {
        mon.record_event();
        let state = mon.check();
        assert_eq!(
            *state,
            AgentHealth::Healthy,
            "state should remain Healthy when events arrive frequently"
        );
    }
}

/// No events for longer than `warning_threshold` transitions to Slow.
#[test]
fn slow_after_warning_threshold() {
    let mut mon = HealthMonitor::new(config_fast());
    mon.record_event();

    // Wait past the warning threshold but not the stuck timeout.
    std::thread::sleep(Duration::from_millis(60));

    let state = mon.check();
    match state {
        AgentHealth::Slow { since } => {
            // `since` should be approximately when the last event was recorded.
            assert!(
                since.elapsed() >= Duration::from_millis(50),
                "`since` should be at least 50ms ago"
            );
        }
        other => panic!("expected Slow, got {other:?}"),
    }
}

/// No events for longer than `stuck_timeout` transitions to Stuck.
#[test]
fn stuck_after_timeout() {
    let mut mon = HealthMonitor::new(config_fast());
    mon.record_event();

    // Wait past the stuck timeout.
    std::thread::sleep(Duration::from_millis(110));

    let state = mon.check();
    match state {
        AgentHealth::Stuck { since } => {
            assert!(
                since.elapsed() >= Duration::from_millis(100),
                "`since` should be at least 100ms ago"
            );
        }
        other => panic!("expected Stuck, got {other:?}"),
    }
}

/// `reset()` returns the state to Healthy even after Stuck.
#[test]
fn reset_clears_stuck() {
    let mut mon = HealthMonitor::new(config_fast());
    mon.record_event();

    std::thread::sleep(Duration::from_millis(110));
    let _ = mon.check();
    assert!(
        matches!(mon.state(), AgentHealth::Stuck { .. }),
        "precondition: should be Stuck"
    );

    mon.reset();
    assert_eq!(
        *mon.state(),
        AgentHealth::Healthy,
        "reset should return state to Healthy"
    );

    let state = mon.check();
    assert_eq!(
        *state,
        AgentHealth::Healthy,
        "check immediately after reset should be Healthy"
    );
}

/// `mark_dead` transitions to Dead with the given exit code.
#[test]
fn dead_on_exit() {
    let mut mon = HealthMonitor::new(config_fast());
    mon.record_event();

    mon.mark_dead(Some(137));

    assert_eq!(
        *mon.state(),
        AgentHealth::Dead {
            exit_code: Some(137)
        }
    );

    // Dead is terminal — check() should not change it.
    let state = mon.check();
    assert_eq!(
        *state,
        AgentHealth::Dead {
            exit_code: Some(137)
        },
        "Dead state should be sticky"
    );
}

/// Dead with `None` exit code.
#[test]
fn dead_with_no_exit_code() {
    let mut mon = HealthMonitor::new(config_fast());
    mon.mark_dead(None);

    assert_eq!(*mon.state(), AgentHealth::Dead { exit_code: None });
}

/// After Slow, an event brings it back to Healthy.
#[test]
fn slow_recovers_on_event() {
    let mut mon = HealthMonitor::new(config_fast());
    mon.record_event();

    std::thread::sleep(Duration::from_millis(60));
    let _ = mon.check();
    assert!(
        matches!(mon.state(), AgentHealth::Slow { .. }),
        "precondition: should be Slow"
    );

    mon.record_event();
    assert_eq!(
        *mon.state(),
        AgentHealth::Healthy,
        "recording an event should restore Healthy"
    );
}

/// Dead is not cleared by `record_event`.
#[test]
fn dead_not_cleared_by_event() {
    let mut mon = HealthMonitor::new(config_fast());
    mon.mark_dead(Some(1));
    mon.record_event();

    assert_eq!(
        *mon.state(),
        AgentHealth::Dead { exit_code: Some(1) },
        "Dead should not be cleared by record_event"
    );
}

/// Default config has expected values.
#[test]
fn default_config() {
    let cfg = HealthConfig::default();
    assert_eq!(cfg.stuck_timeout, Duration::from_secs(30));
    assert_eq!(cfg.warning_threshold, Duration::from_secs(15));
    assert!(cfg.auto_restart);
}
