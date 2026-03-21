//! Tests for the IPC protocol: serialization round-trips and socket I/O.

use smux_core::ipc::{ClientMessage, DaemonMessage, SessionInfo, recv_message, send_message};

// ---------------------------------------------------------------------------
// Serialization round-trip: ClientMessage
// ---------------------------------------------------------------------------

#[test]
fn client_message_start_session_round_trip() {
    let msg = ClientMessage::StartSession {
        planner: "claude".into(),
        verifier: "codex".into(),
        task: "fix the bug".into(),
        max_rounds: 5,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ClientMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn client_message_attach_round_trip() {
    let msg = ClientMessage::AttachSession {
        session_id: "abc123".into(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ClientMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn client_message_detach_round_trip() {
    let msg = ClientMessage::DetachSession;
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ClientMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn client_message_list_round_trip() {
    let msg = ClientMessage::ListSessions;
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ClientMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn client_message_rewind_round_trip() {
    let msg = ClientMessage::RewindSession {
        session_id: "sess-1".into(),
        round: 3,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ClientMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn client_message_shutdown_round_trip() {
    let msg = ClientMessage::Shutdown;
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: ClientMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, msg);
}

// ---------------------------------------------------------------------------
// Serialization round-trip: DaemonMessage
// ---------------------------------------------------------------------------

#[test]
fn daemon_message_session_created_round_trip() {
    let msg = DaemonMessage::SessionCreated {
        session_id: "abc123".into(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: DaemonMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn daemon_message_session_list_round_trip() {
    let msg = DaemonMessage::SessionList {
        sessions: vec![SessionInfo {
            id: "s1".into(),
            task: "fix bug".into(),
            planner: "claude".into(),
            verifier: "codex".into(),
            current_round: 2,
            status: "running".into(),
        }],
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: DaemonMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn daemon_message_agent_output_round_trip() {
    let msg = DaemonMessage::AgentOutput {
        role: "planner".into(),
        content: "here is my plan".into(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: DaemonMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn daemon_message_round_complete_round_trip() {
    let msg = DaemonMessage::RoundComplete {
        round: 3,
        verdict_summary: "APPROVED".into(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: DaemonMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn daemon_message_session_complete_round_trip() {
    let msg = DaemonMessage::SessionComplete {
        summary: "done".into(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: DaemonMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn daemon_message_error_round_trip() {
    let msg = DaemonMessage::Error {
        message: "something went wrong".into(),
    };
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: DaemonMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn daemon_message_ok_round_trip() {
    let msg = DaemonMessage::Ok;
    let json = serde_json::to_string(&msg).unwrap();
    let decoded: DaemonMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, msg);
}

// ---------------------------------------------------------------------------
// Wire protocol: send_message + recv_message over Unix socket pair
// ---------------------------------------------------------------------------

#[tokio::test]
async fn send_recv_client_message_over_socket() {
    let dir = tempfile::TempDir::new().unwrap();
    let sock_path = dir.path().join("test.sock");

    let listener = tokio::net::UnixListener::bind(&sock_path).unwrap();

    let sock_path_clone = sock_path.clone();
    let client_handle = tokio::spawn(async move {
        let mut client = tokio::net::UnixStream::connect(&sock_path_clone)
            .await
            .unwrap();
        let msg = ClientMessage::StartSession {
            planner: "claude".into(),
            verifier: "codex".into(),
            task: "test task".into(),
            max_rounds: 3,
        };
        send_message(&mut client, &msg).await.unwrap();
    });

    let (mut server, _) = listener.accept().await.unwrap();
    let received: ClientMessage = recv_message(&mut server).await.unwrap();

    client_handle.await.unwrap();

    assert_eq!(
        received,
        ClientMessage::StartSession {
            planner: "claude".into(),
            verifier: "codex".into(),
            task: "test task".into(),
            max_rounds: 3,
        }
    );
}

#[tokio::test]
async fn send_recv_daemon_message_over_socket() {
    let dir = tempfile::TempDir::new().unwrap();
    let sock_path = dir.path().join("test2.sock");

    let listener = tokio::net::UnixListener::bind(&sock_path).unwrap();

    let server_handle = tokio::spawn(async move {
        let (mut server, _) = listener.accept().await.unwrap();
        let msg = DaemonMessage::SessionCreated {
            session_id: "sess-42".into(),
        };
        send_message(&mut server, &msg).await.unwrap();
    });

    let mut client = tokio::net::UnixStream::connect(&sock_path).await.unwrap();
    let received: DaemonMessage = recv_message(&mut client).await.unwrap();

    server_handle.await.unwrap();

    assert_eq!(
        received,
        DaemonMessage::SessionCreated {
            session_id: "sess-42".into(),
        }
    );
}

#[tokio::test]
async fn multiple_messages_over_socket() {
    let dir = tempfile::TempDir::new().unwrap();
    let sock_path = dir.path().join("test3.sock");

    let listener = tokio::net::UnixListener::bind(&sock_path).unwrap();

    let sock_path_clone = sock_path.clone();
    let client_handle = tokio::spawn(async move {
        let mut client = tokio::net::UnixStream::connect(&sock_path_clone)
            .await
            .unwrap();
        send_message(&mut client, &ClientMessage::ListSessions)
            .await
            .unwrap();
        send_message(&mut client, &ClientMessage::Shutdown)
            .await
            .unwrap();
    });

    let (mut server, _) = listener.accept().await.unwrap();
    let msg1: ClientMessage = recv_message(&mut server).await.unwrap();
    let msg2: ClientMessage = recv_message(&mut server).await.unwrap();

    client_handle.await.unwrap();

    assert_eq!(msg1, ClientMessage::ListSessions);
    assert_eq!(msg2, ClientMessage::Shutdown);
}

#[tokio::test]
async fn connection_closed_returns_error() {
    let dir = tempfile::TempDir::new().unwrap();
    let sock_path = dir.path().join("test4.sock");

    let listener = tokio::net::UnixListener::bind(&sock_path).unwrap();

    let sock_path_clone = sock_path.clone();
    let client_handle = tokio::spawn(async move {
        let _client = tokio::net::UnixStream::connect(&sock_path_clone)
            .await
            .unwrap();
        // Drop immediately — connection closes.
    });

    let (mut server, _) = listener.accept().await.unwrap();
    client_handle.await.unwrap();

    // Give the drop a moment to propagate.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let result = recv_message::<ClientMessage>(&mut server).await;
    assert!(result.is_err(), "should fail on closed connection");
}
