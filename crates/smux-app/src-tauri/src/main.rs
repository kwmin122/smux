#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::UnixStream;
use tokio::sync::Mutex;

use smux_core::ipc::{
    ClientMessage, DaemonMessage, SessionInfo, default_socket_path, recv_message, send_message,
};

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

struct AppState {
    /// Active session ID (if any).
    active_session: Arc<Mutex<Option<String>>>,
}

// ---------------------------------------------------------------------------
// Events emitted to the frontend
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize)]
#[serde(tag = "kind")]
enum DaemonEvent {
    #[serde(rename = "agent_output")]
    AgentOutput { role: String, content: String },
    #[serde(rename = "round_complete")]
    RoundComplete { round: u32, verdict_summary: String },
    #[serde(rename = "session_complete")]
    SessionComplete { summary: String },
    #[serde(rename = "cross_verify_result")]
    CrossVerifyResult {
        round: u32,
        individual: Vec<serde_json::Value>,
        final_verdict: String,
        strategy: String,
        agreement_ratio: f64,
    },
    #[serde(rename = "error")]
    Error { message: String },
}

// ---------------------------------------------------------------------------
// Tauri commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn ping() -> String {
    "pong".to_string()
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartSessionArgs {
    planner: String,
    verifier: String,
    task: String,
    max_rounds: u32,
    verifiers: Vec<String>,
    consensus: String,
}

#[tauri::command]
async fn start_session(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    args: StartSessionArgs,
) -> Result<String, String> {
    let StartSessionArgs {
        planner,
        verifier,
        task,
        max_rounds,
        verifiers,
        consensus,
    } = args;
    let socket_path = default_socket_path();
    let mut stream = UnixStream::connect(&socket_path)
        .await
        .map_err(|e| format!("Failed to connect to daemon: {e}"))?;

    let msg = ClientMessage::StartSession {
        planner,
        verifier,
        task,
        max_rounds,
        verifiers,
        consensus,
    };
    send_message(&mut stream, &msg)
        .await
        .map_err(|e| format!("Send error: {e}"))?;

    let resp: DaemonMessage = recv_message(&mut stream)
        .await
        .map_err(|e| format!("Recv error: {e}"))?;

    match resp {
        DaemonMessage::SessionCreated { session_id } => {
            *state.active_session.lock().await = Some(session_id.clone());

            // Spawn background task to stream daemon events to frontend.
            let app_handle = app.clone();
            let sid = session_id.clone();
            tokio::spawn(async move {
                stream_daemon_events(app_handle, stream, &sid).await;
            });

            Ok(session_id)
        }
        DaemonMessage::Error { message } => Err(message),
        other => Err(format!("Unexpected response: {other:?}")),
    }
}

#[tauri::command]
async fn attach_session(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
    session_id: String,
) -> Result<(), String> {
    let socket_path = default_socket_path();
    let mut stream = UnixStream::connect(&socket_path)
        .await
        .map_err(|e| format!("Failed to connect to daemon: {e}"))?;

    let msg = ClientMessage::AttachSession {
        session_id: session_id.clone(),
    };
    send_message(&mut stream, &msg)
        .await
        .map_err(|e| format!("Send error: {e}"))?;

    let resp: DaemonMessage = recv_message(&mut stream)
        .await
        .map_err(|e| format!("Recv error: {e}"))?;

    match resp {
        DaemonMessage::Ok => {
            *state.active_session.lock().await = Some(session_id.clone());

            let app_handle = app.clone();
            let sid = session_id.clone();
            tokio::spawn(async move {
                stream_daemon_events(app_handle, stream, &sid).await;
            });

            Ok(())
        }
        DaemonMessage::Error { message } => Err(message),
        other => Err(format!("Unexpected response: {other:?}")),
    }
}

#[tauri::command]
async fn list_sessions() -> Result<Vec<SessionInfo>, String> {
    let socket_path = default_socket_path();
    let mut stream = UnixStream::connect(&socket_path)
        .await
        .map_err(|e| format!("Failed to connect to daemon: {e}"))?;

    send_message(&mut stream, &ClientMessage::ListSessions)
        .await
        .map_err(|e| format!("Send error: {e}"))?;

    let resp: DaemonMessage = recv_message(&mut stream)
        .await
        .map_err(|e| format!("Recv error: {e}"))?;

    match resp {
        DaemonMessage::SessionList { sessions } => Ok(sessions),
        DaemonMessage::Error { message } => Err(message),
        other => Err(format!("Unexpected response: {other:?}")),
    }
}

#[tauri::command]
async fn get_active_session(state: tauri::State<'_, AppState>) -> Result<Option<String>, String> {
    Ok(state.active_session.lock().await.clone())
}

#[derive(Clone, Serialize)]
struct GitInfo {
    branch: String,
    files_changed: u32,
}

#[tauri::command]
async fn get_git_info() -> Result<GitInfo, String> {
    let branch = tokio::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .await
        .map_err(|e| format!("git error: {e}"))
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .map(|s| s.trim().to_string())
                .map_err(|e| format!("utf8 error: {e}"))
        })
        .unwrap_or_else(|_| "unknown".into());

    // Use `git status --short` to capture the full worktree state:
    // modified, staged, untracked, and deleted files.
    let files_changed = tokio::process::Command::new("git")
        .args(["status", "--short"])
        .output()
        .await
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .count() as u32
        })
        .unwrap_or(0);

    Ok(GitInfo {
        branch,
        files_changed,
    })
}

#[tauri::command]
async fn open_browser_window(app: AppHandle, url: String) -> Result<(), String> {
    // Validate localhost-only.
    let parsed = url::Url::parse(&url).map_err(|e| format!("invalid URL: {e}"))?;
    let host = parsed.host_str().unwrap_or("");
    if host != "localhost" && host != "127.0.0.1" && host != "[::1]" {
        return Err("only localhost URLs are allowed".into());
    }

    // Close existing browser window if any.
    if let Some(w) = app.get_webview_window("smux-browser") {
        let _ = w.close();
    }

    // Open a new native WebView window.
    tauri::WebviewWindowBuilder::new(&app, "smux-browser", tauri::WebviewUrl::External(parsed))
        .title(format!("smux browser — {url}"))
        .inner_size(1024.0, 768.0)
        .build()
        .map_err(|e| format!("failed to open browser window: {e}"))?;

    Ok(())
}

#[tauri::command]
async fn close_browser_window(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("smux-browser") {
        w.close().map_err(|e| format!("failed to close: {e}"))?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Event streaming
// ---------------------------------------------------------------------------

async fn stream_daemon_events(app: AppHandle, mut stream: UnixStream, _session_id: &str) {
    loop {
        match recv_message::<DaemonMessage>(&mut stream).await {
            Ok(msg) => {
                let event = match &msg {
                    DaemonMessage::AgentOutput { role, content } => DaemonEvent::AgentOutput {
                        role: role.clone(),
                        content: content.clone(),
                    },
                    DaemonMessage::RoundComplete {
                        round,
                        verdict_summary,
                    } => DaemonEvent::RoundComplete {
                        round: *round,
                        verdict_summary: verdict_summary.clone(),
                    },
                    DaemonMessage::SessionComplete { summary } => {
                        let evt = DaemonEvent::SessionComplete {
                            summary: summary.clone(),
                        };
                        let _ = app.emit("daemon-event", evt);
                        return; // Session done, stop streaming.
                    }
                    DaemonMessage::CrossVerifyResult {
                        round,
                        individual,
                        final_verdict,
                        strategy,
                        agreement_ratio,
                    } => DaemonEvent::CrossVerifyResult {
                        round: *round,
                        individual: individual
                            .iter()
                            .map(|v| {
                                serde_json::json!({
                                    "verifier": v.verifier,
                                    "verdict": v.verdict,
                                    "confidence": v.confidence,
                                    "reason": v.reason,
                                })
                            })
                            .collect(),
                        final_verdict: final_verdict.clone(),
                        strategy: strategy.clone(),
                        agreement_ratio: *agreement_ratio,
                    },
                    DaemonMessage::Error { message } => DaemonEvent::Error {
                        message: message.clone(),
                    },
                    _ => continue,
                };

                let _ = app.emit("daemon-event", event);
            }
            Err(smux_core::ipc::IpcError::ConnectionClosed) => return,
            Err(_) => return,
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    tauri::Builder::default()
        .manage(AppState {
            active_session: Arc::new(Mutex::new(None)),
        })
        .invoke_handler(tauri::generate_handler![
            ping,
            start_session,
            attach_session,
            list_sessions,
            get_active_session,
            get_git_info,
            open_browser_window,
            close_browser_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
