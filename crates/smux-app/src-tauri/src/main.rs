#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;

use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::UnixStream;
use tokio::sync::Mutex;

use smux_core::ipc::{
    ClientMessage, DaemonMessage, SessionInfo, default_socket_path, recv_message, send_message,
};

// ---------------------------------------------------------------------------
// PTY management
// ---------------------------------------------------------------------------

struct PtySession {
    #[allow(dead_code)]
    master: Box<dyn MasterPty + Send>,
    writer: Arc<std::sync::Mutex<Box<dyn Write + Send>>>,
}

struct PtyManager {
    sessions: std::sync::Mutex<HashMap<String, PtySession>>,
    pending_readers: std::sync::Mutex<HashMap<String, Box<dyn Read + Send>>>,
}

impl PtyManager {
    fn new() -> Self {
        Self {
            sessions: std::sync::Mutex::new(HashMap::new()),
            pending_readers: std::sync::Mutex::new(HashMap::new()),
        }
    }
}

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
// PTY commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn create_pty(
    pty_mgr: tauri::State<PtyManager>,
    rows: Option<u16>,
    cols: Option<u16>,
) -> Result<String, String> {
    let rows = rows.unwrap_or(24);
    let cols = cols.unwrap_or(80);

    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("openpty failed: {e}"))?;

    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| format!("clone reader: {e}"))?;
    let writer = pair
        .master
        .take_writer()
        .map_err(|e| format!("take writer: {e}"))?;

    // Detect user's shell
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.env("TERM", "xterm-256color");
    if let Ok(home) = std::env::var("HOME") {
        cmd.cwd(&home);
    }

    let _child = pair
        .slave
        .spawn_command(cmd)
        .map_err(|e| format!("spawn failed: {e}"))?;
    drop(pair.slave); // must drop slave after spawn

    let tab_id = uuid::Uuid::new_v4().to_string();

    pty_mgr.sessions.lock().unwrap().insert(
        tab_id.clone(),
        PtySession {
            master: pair.master,
            writer: Arc::new(std::sync::Mutex::new(writer)),
        },
    );
    pty_mgr
        .pending_readers
        .lock()
        .unwrap()
        .insert(tab_id.clone(), reader);

    Ok(tab_id)
}

#[tauri::command]
fn start_pty(
    app: AppHandle,
    pty_mgr: tauri::State<PtyManager>,
    tab_id: String,
) -> Result<(), String> {
    let reader = pty_mgr
        .pending_readers
        .lock()
        .unwrap()
        .remove(&tab_id)
        .ok_or("no pending reader for this tab")?;

    let event_name = format!("pty-output-{tab_id}");
    let exit_event = format!("pty-exit-{tab_id}");

    std::thread::spawn(move || {
        let mut reader = reader;
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => {
                    let _ = app.emit(&exit_event, ());
                    break;
                }
                Ok(n) => {
                    let text = String::from_utf8_lossy(&buf[..n]).to_string();
                    let _ = app.emit(&event_name, text);
                }
                Err(_) => {
                    let _ = app.emit(&exit_event, ());
                    break;
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
fn write_pty(
    pty_mgr: tauri::State<PtyManager>,
    tab_id: String,
    data: String,
) -> Result<(), String> {
    let sessions = pty_mgr.sessions.lock().unwrap();
    let session = sessions.get(&tab_id).ok_or("session not found")?;
    session
        .writer
        .lock()
        .unwrap()
        .write_all(data.as_bytes())
        .map_err(|e| format!("write failed: {e}"))
}

#[tauri::command]
fn resize_pty(
    pty_mgr: tauri::State<PtyManager>,
    tab_id: String,
    rows: u16,
    cols: u16,
) -> Result<(), String> {
    let sessions = pty_mgr.sessions.lock().unwrap();
    let session = sessions.get(&tab_id).ok_or("session not found")?;
    session
        .master
        .resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| format!("resize failed: {e}"))
}

#[tauri::command]
fn close_pty(pty_mgr: tauri::State<PtyManager>, tab_id: String) -> Result<(), String> {
    pty_mgr.sessions.lock().unwrap().remove(&tab_id);
    pty_mgr.pending_readers.lock().unwrap().remove(&tab_id);
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
        .manage(PtyManager::new())
        .invoke_handler(tauri::generate_handler![
            ping,
            start_session,
            attach_session,
            list_sessions,
            get_active_session,
            get_git_info,
            open_browser_window,
            close_browser_window,
            create_pty,
            start_pty,
            write_pty,
            resize_pty,
            close_pty,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
