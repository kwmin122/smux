//! Smoke tests for the smux daemon.
//!
//! Each test starts a minimal daemon on a random socket path (in a temp dir),
//! sends IPC messages, and verifies responses.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::net::UnixStream;
use tokio::sync::Mutex;

use smux_core::ipc::{ClientMessage, DaemonMessage, recv_message, send_message};

// ---------------------------------------------------------------------------
// Test daemon — minimal IPC server for testing
// ---------------------------------------------------------------------------

struct SessionEntry {
    id: String,
    task: String,
    planner: String,
    verifier: String,
    current_round: u32,
    status: String,
}

/// Start a minimal test daemon on the given socket path.
fn start_test_daemon(socket_path: PathBuf) {
    tokio::spawn(async move {
        if let Err(e) = run_test_daemon(&socket_path).await {
            eprintln!("test daemon error: {e}");
        }
    });
}

async fn run_test_daemon(socket_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    if socket_path.exists() {
        tokio::fs::remove_file(socket_path).await?;
    }

    let listener = tokio::net::UnixListener::bind(socket_path)?;
    let sessions: Arc<Mutex<HashMap<String, SessionEntry>>> = Arc::new(Mutex::new(HashMap::new()));

    loop {
        let (stream, _) = listener.accept().await?;
        let sessions = sessions.clone();

        let should_shutdown =
            tokio::spawn(async move { handle_test_client(stream, sessions).await })
                .await
                .unwrap_or(false);

        if should_shutdown {
            break;
        }
    }

    let _ = tokio::fs::remove_file(socket_path).await;
    Ok(())
}

/// Handle a single client connection. Returns true if daemon should shut down.
async fn handle_test_client(
    mut stream: UnixStream,
    sessions: Arc<Mutex<HashMap<String, SessionEntry>>>,
) -> bool {
    let msg: ClientMessage = match recv_message(&mut stream).await {
        Ok(m) => m,
        Err(_) => return false,
    };

    match msg {
        ClientMessage::ListSessions => {
            let s = sessions.lock().await;
            let list: Vec<smux_core::ipc::SessionInfo> = s
                .values()
                .map(|e| smux_core::ipc::SessionInfo {
                    id: e.id.clone(),
                    task: e.task.clone(),
                    planner: e.planner.clone(),
                    verifier: e.verifier.clone(),
                    current_round: e.current_round,
                    status: e.status.clone(),
                })
                .collect();
            let _ = send_message(&mut stream, &DaemonMessage::SessionList { sessions: list }).await;
            false
        }

        ClientMessage::StartSession {
            planner,
            verifier,
            task,
            max_rounds: _,
        } => {
            let session_id = format!("test-{}", sessions.lock().await.len());
            sessions.lock().await.insert(
                session_id.clone(),
                SessionEntry {
                    id: session_id.clone(),
                    task: task.clone(),
                    planner: planner.clone(),
                    verifier: verifier.clone(),
                    current_round: 1,
                    status: "completed".into(),
                },
            );

            let _ = send_message(
                &mut stream,
                &DaemonMessage::SessionCreated {
                    session_id: session_id.clone(),
                },
            )
            .await;

            // Simulate a quick session.
            let _ = send_message(
                &mut stream,
                &DaemonMessage::AgentOutput {
                    role: "planner".into(),
                    content: "implementing the task".into(),
                },
            )
            .await;
            let _ = send_message(
                &mut stream,
                &DaemonMessage::RoundComplete {
                    round: 1,
                    verdict_summary: "APPROVED".into(),
                },
            )
            .await;
            let _ = send_message(
                &mut stream,
                &DaemonMessage::SessionComplete {
                    summary: "APPROVED at round 1".into(),
                },
            )
            .await;

            // Connection done for this client.
            false
        }

        ClientMessage::AttachSession { session_id } => {
            let exists = sessions.lock().await.contains_key(&session_id);
            if exists {
                let _ = send_message(&mut stream, &DaemonMessage::Ok).await;
                let _ = send_message(
                    &mut stream,
                    &DaemonMessage::AgentOutput {
                        role: "verifier".into(),
                        content: "verifying...".into(),
                    },
                )
                .await;
                let _ = send_message(
                    &mut stream,
                    &DaemonMessage::SessionComplete {
                        summary: "attached session done".into(),
                    },
                )
                .await;
            } else {
                let _ = send_message(
                    &mut stream,
                    &DaemonMessage::Error {
                        message: format!("session not found: {session_id}"),
                    },
                )
                .await;
            }
            false
        }

        ClientMessage::Shutdown => {
            let _ = send_message(&mut stream, &DaemonMessage::Ok).await;
            true
        }

        _ => {
            let _ = send_message(&mut stream, &DaemonMessage::Ok).await;
            false
        }
    }
}

/// Connect to the test daemon, retrying a few times.
async fn connect_retry(socket_path: &PathBuf) -> UnixStream {
    for _ in 0..30 {
        if let Ok(stream) = UnixStream::connect(socket_path).await {
            return stream;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    panic!(
        "could not connect to test daemon at {}",
        socket_path.display()
    );
}

// ---------------------------------------------------------------------------
// Test 1: ListSessions on empty daemon returns empty list
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_sessions_empty() {
    let dir = tempfile::TempDir::new().unwrap();
    let sock = dir.path().join("smux.sock");
    start_test_daemon(sock.clone());

    let mut stream = connect_retry(&sock).await;

    send_message(&mut stream, &ClientMessage::ListSessions)
        .await
        .unwrap();

    let response: DaemonMessage = recv_message(&mut stream).await.unwrap();

    match response {
        DaemonMessage::SessionList { sessions } => {
            assert!(sessions.is_empty(), "expected empty session list");
        }
        other => panic!("expected SessionList, got {other:?}"),
    }

    // Shut down the daemon.
    let mut stream2 = connect_retry(&sock).await;
    send_message(&mut stream2, &ClientMessage::Shutdown)
        .await
        .unwrap();
}

// ---------------------------------------------------------------------------
// Test 2: StartSession returns SessionCreated + events
// ---------------------------------------------------------------------------

#[tokio::test]
async fn start_session_returns_created() {
    let dir = tempfile::TempDir::new().unwrap();
    let sock = dir.path().join("smux.sock");
    start_test_daemon(sock.clone());

    let mut stream = connect_retry(&sock).await;

    send_message(
        &mut stream,
        &ClientMessage::StartSession {
            planner: "claude".into(),
            verifier: "codex".into(),
            task: "test task".into(),
            max_rounds: 3,
        },
    )
    .await
    .unwrap();

    let response: DaemonMessage = recv_message(&mut stream).await.unwrap();
    match &response {
        DaemonMessage::SessionCreated { session_id } => {
            assert!(!session_id.is_empty(), "session_id should not be empty");
        }
        other => panic!("expected SessionCreated, got {other:?}"),
    }

    // Drain remaining events.
    let mut got_complete = false;
    for _ in 0..10 {
        match recv_message::<DaemonMessage>(&mut stream).await {
            Ok(DaemonMessage::SessionComplete { .. }) => {
                got_complete = true;
                break;
            }
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    assert!(got_complete, "should receive SessionComplete");

    // Shut down.
    let mut stream2 = connect_retry(&sock).await;
    let _ = send_message(&mut stream2, &ClientMessage::Shutdown).await;
}

// ---------------------------------------------------------------------------
// Test 3: StartSession → AttachSession receives events
// ---------------------------------------------------------------------------

#[tokio::test]
async fn start_then_attach_receives_events() {
    let dir = tempfile::TempDir::new().unwrap();
    let sock = dir.path().join("smux.sock");
    start_test_daemon(sock.clone());

    // Start a session.
    let mut stream1 = connect_retry(&sock).await;
    send_message(
        &mut stream1,
        &ClientMessage::StartSession {
            planner: "claude".into(),
            verifier: "codex".into(),
            task: "implement feature".into(),
            max_rounds: 5,
        },
    )
    .await
    .unwrap();

    // Read session_id.
    let session_id = match recv_message::<DaemonMessage>(&mut stream1).await.unwrap() {
        DaemonMessage::SessionCreated { session_id } => session_id,
        other => panic!("expected SessionCreated, got {other:?}"),
    };

    // Drain all messages from start stream.
    loop {
        match recv_message::<DaemonMessage>(&mut stream1).await {
            Ok(DaemonMessage::SessionComplete { .. }) => break,
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    drop(stream1);

    // Attach to the session on a new connection.
    let mut stream2 = connect_retry(&sock).await;
    send_message(
        &mut stream2,
        &ClientMessage::AttachSession {
            session_id: session_id.clone(),
        },
    )
    .await
    .unwrap();

    let attach_response: DaemonMessage = recv_message(&mut stream2).await.unwrap();
    assert_eq!(attach_response, DaemonMessage::Ok);

    // Should receive events.
    let event: DaemonMessage = recv_message(&mut stream2).await.unwrap();
    match event {
        DaemonMessage::AgentOutput { role, content } => {
            assert_eq!(role, "verifier");
            assert!(!content.is_empty());
        }
        other => panic!("expected AgentOutput, got {other:?}"),
    }

    // Drain to SessionComplete.
    loop {
        match recv_message::<DaemonMessage>(&mut stream2).await {
            Ok(DaemonMessage::SessionComplete { .. }) => break,
            Ok(_) => continue,
            Err(_) => break,
        }
    }

    // Cleanup.
    let mut stream3 = connect_retry(&sock).await;
    let _ = send_message(&mut stream3, &ClientMessage::Shutdown).await;
}

// ---------------------------------------------------------------------------
// Test 4: ListSessions after StartSession shows the session
// ---------------------------------------------------------------------------

#[tokio::test]
async fn list_sessions_after_start() {
    let dir = tempfile::TempDir::new().unwrap();
    let sock = dir.path().join("smux.sock");
    start_test_daemon(sock.clone());

    // Start a session.
    let mut stream1 = connect_retry(&sock).await;
    send_message(
        &mut stream1,
        &ClientMessage::StartSession {
            planner: "claude".into(),
            verifier: "codex".into(),
            task: "my task".into(),
            max_rounds: 3,
        },
    )
    .await
    .unwrap();

    // Drain all messages.
    loop {
        match recv_message::<DaemonMessage>(&mut stream1).await {
            Ok(DaemonMessage::SessionComplete { .. }) => break,
            Ok(_) => continue,
            Err(_) => break,
        }
    }
    drop(stream1);

    // List sessions on a new connection.
    let mut stream2 = connect_retry(&sock).await;
    send_message(&mut stream2, &ClientMessage::ListSessions)
        .await
        .unwrap();

    let response: DaemonMessage = recv_message(&mut stream2).await.unwrap();
    match response {
        DaemonMessage::SessionList { sessions } => {
            assert_eq!(sessions.len(), 1, "should have 1 session");
            assert_eq!(sessions[0].task, "my task");
            assert_eq!(sessions[0].planner, "claude");
            assert_eq!(sessions[0].verifier, "codex");
        }
        other => panic!("expected SessionList, got {other:?}"),
    }

    // Cleanup.
    let mut stream3 = connect_retry(&sock).await;
    send_message(&mut stream3, &ClientMessage::Shutdown)
        .await
        .unwrap();
}
