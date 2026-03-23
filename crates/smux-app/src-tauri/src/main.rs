#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::Arc;

use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::UnixStream;
use tokio::sync::Mutex;

use smux_core::ipc::{
    ClientMessage, DaemonMessage, SessionInfo, default_socket_path, recv_message, send_message,
};

// ---------------------------------------------------------------------------
// App configuration (~/.smux/config.toml)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct AppConfig {
    general: GeneralConfig,
    appearance: AppearanceConfig,
    ai: AiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct GeneralConfig {
    shell: String,
    scrollback: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct AppearanceConfig {
    font_family: String,
    font_size: u32,
    theme: String,
    cursor_style: String,
    cursor_blink: bool,
    minimum_contrast_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct AiConfig {
    auto_execution_level: String,
    allow_commands: Vec<String>,
    deny_commands: Vec<String>,
    max_rounds: u32,
    default_planner: String,
    default_verifier: String,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            shell: std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".to_string()),
            scrollback: 10000,
        }
    }
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            font_family: "JetBrains Mono".to_string(),
            font_size: 14,
            theme: "deep-navy".to_string(),
            cursor_style: "block".to_string(),
            cursor_blink: true,
            minimum_contrast_ratio: 4.5,
        }
    }
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            auto_execution_level: "auto".to_string(),
            allow_commands: vec![
                "git".to_string(),
                "cargo".to_string(),
                "npm".to_string(),
                "pnpm".to_string(),
                "yarn".to_string(),
            ],
            deny_commands: vec![
                "rm -rf /".to_string(),
                "sudo rm".to_string(),
                "shutdown".to_string(),
            ],
            max_rounds: 5,
            default_planner: "claude".to_string(),
            default_verifier: "codex".to_string(),
        }
    }
}

fn config_path() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".smux")
        .join("config.toml")
}

fn load_config() -> AppConfig {
    let path = config_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => toml::from_str(&content).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

fn save_config(config: &AppConfig) -> Result<(), String> {
    let path = config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir failed: {e}"))?;
        // Set restrictive permissions on ~/.smux directory
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o700));
        }
    }
    let content = toml::to_string_pretty(config).map_err(|e| format!("serialize failed: {e}"))?;
    std::fs::write(&path, content).map_err(|e| format!("write failed: {e}"))?;
    // Set restrictive permissions on config file
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

#[tauri::command]
fn load_app_config() -> AppConfig {
    load_config()
}

#[tauri::command]
fn save_app_config(pty_mgr: tauri::State<PtyManager>, config: AppConfig) -> Result<(), String> {
    // Refresh cached deny-list
    *pty_mgr.deny_list.lock().unwrap() = config.ai.deny_commands.clone();
    save_config(&config)
}

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
    /// Cached deny-list from config (refreshed on save_app_config)
    deny_list: std::sync::Mutex<Vec<String>>,
}

impl PtyManager {
    fn new() -> Self {
        let config = load_config();
        Self {
            sessions: std::sync::Mutex::new(HashMap::new()),
            pending_readers: std::sync::Mutex::new(HashMap::new()),
            deny_list: std::sync::Mutex::new(config.ai.deny_commands),
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

#[derive(Deserialize)]
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
async fn get_git_info(cwd: Option<String>) -> Result<GitInfo, String> {
    let work_dir = cwd
        .or_else(|| std::env::var("HOME").ok())
        .unwrap_or_else(|| "/".to_string());

    // First check if this is even a git repo (fast, no network, no credentials)
    let is_git_repo = tokio::process::Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(&work_dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "/usr/bin/true")
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !is_git_repo {
        return Ok(GitInfo {
            branch: "—".to_string(),
            files_changed: 0,
        });
    }

    let branch = tokio::process::Command::new("git")
        .args([
            "-c",
            "credential.helper=",
            "rev-parse",
            "--abbrev-ref",
            "HEAD",
        ])
        .current_dir(&work_dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "/usr/bin/true")
        .env("SSH_ASKPASS", "/usr/bin/true")
        .env("DISPLAY", "") // Prevent X11 askpass
        .output()
        .await
        .map_err(|e| format!("git error: {e}"))
        .and_then(|o| {
            String::from_utf8(o.stdout)
                .map(|s| s.trim().to_string())
                .map_err(|e| format!("utf8 error: {e}"))
        })
        .unwrap_or_else(|_| "unknown".into());

    let files_changed = tokio::process::Command::new("git")
        .args(["-c", "credential.helper=", "status", "--short"])
        .current_dir(&work_dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_ASKPASS", "/usr/bin/true")
        .env("SSH_ASKPASS", "/usr/bin/true")
        .env("DISPLAY", "")
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
    window: tauri::Window,
    pty_mgr: tauri::State<PtyManager>,
    rows: Option<u16>,
    cols: Option<u16>,
    cwd: Option<String>,
    shell_cmd: Option<String>,
) -> Result<String, String> {
    if window.label() != "main" {
        return Err("create_pty restricted to main window".into());
    }
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

    // Use provided shell command, or config shell, or detect default
    let config = load_config();
    let shell = shell_cmd.unwrap_or_else(|| config.general.shell.clone());

    // Security: validate shell is a known shell binary
    let allowed_shells = [
        "/bin/zsh",
        "/bin/bash",
        "/bin/sh",
        "/usr/bin/zsh",
        "/usr/bin/bash",
        "/usr/local/bin/zsh",
        "/usr/local/bin/bash",
        "/usr/local/bin/fish",
        "/opt/homebrew/bin/zsh",
        "/opt/homebrew/bin/bash",
        "/opt/homebrew/bin/fish",
    ];
    // Strict allowlist — no ends_with fallback (prevents /tmp/evil/zsh bypass)
    if !allowed_shells.iter().any(|s| shell == *s) {
        return Err(format!("shell not in allowlist: {shell}"));
    }

    let mut cmd = CommandBuilder::new(&shell);
    cmd.env("TERM", "xterm-256color");
    // Set locale for Korean/CJK support (Tauri apps from Finder don't inherit shell env)
    let lang = std::env::var("LANG").unwrap_or_else(|_| "en_US.UTF-8".to_string());
    cmd.env("LANG", &lang);
    cmd.env("LC_ALL", &lang);
    cmd.env("SMUX_SHELL_INTEGRATION", "1");

    // Use provided cwd, or fall back to HOME. Validate it exists.
    let working_dir =
        cwd.unwrap_or_else(|| std::env::var("HOME").unwrap_or_else(|_| "/".to_string()));
    if !std::path::Path::new(&working_dir).is_dir() {
        return Err(format!("working directory does not exist: {working_dir}"));
    }
    cmd.cwd(&working_dir);

    // Install shell integration script for zsh
    if shell.ends_with("zsh") || shell.ends_with("zsh\"") {
        let smux_dir = dirs::home_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
            .join(".smux");
        let _ = std::fs::create_dir_all(&smux_dir);
        let integration_script = include_str!("../../src/shell-integration/shell-integration.zsh");
        let script_path = smux_dir.join("shell-integration.zsh");
        let _ = std::fs::write(&script_path, integration_script);
        // Inject sourcing via ZDOTDIR trick or direct env
        cmd.env("SMUX_INJECT_PROMPT", "1");
        cmd.env(
            "SMUX_INTEGRATION_PATH",
            script_path.to_string_lossy().to_string(),
        );
        // Source the integration script at shell startup via zshenv
        let zshenv_content = format!(
            "source \"{}\"\n[ -f \"$HOME/.zshenv\" ] && source \"$HOME/.zshenv\"\n",
            script_path.display()
        );
        let custom_zdotdir = smux_dir.join("zdotdir");
        let _ = std::fs::create_dir_all(&custom_zdotdir);
        // Set restrictive permissions on zdotdir (owner-only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ =
                std::fs::set_permissions(&custom_zdotdir, std::fs::Permissions::from_mode(0o700));
        }
        let _ = std::fs::write(custom_zdotdir.join(".zshenv"), &zshenv_content);
        // Source real zshrc/zprofile instead of copying (avoids stale copies + conda issues)
        let zshrc_content = "[ -f \"$HOME/.zshrc\" ] && source \"$HOME/.zshrc\" 2>/dev/null\n";
        let _ = std::fs::write(custom_zdotdir.join(".zshrc"), zshrc_content);
        let zprofile_content =
            "[ -f \"$HOME/.zprofile\" ] && source \"$HOME/.zprofile\" 2>/dev/null\n";
        let _ = std::fs::write(custom_zdotdir.join(".zprofile"), zprofile_content);
        cmd.env("ZDOTDIR", custom_zdotdir.to_string_lossy().to_string());
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
    window: tauri::Window,
    pty_mgr: tauri::State<PtyManager>,
    tab_id: String,
    data: String,
) -> Result<(), String> {
    // Block calls from browser WebView (prevent XSS→RCE)
    if window.label() != "main" {
        return Err("write_pty restricted to main window".into());
    }
    // Enforce deny-list from cached config (no disk read per keystroke)
    if data.contains('\n') || data.contains('\r') {
        let deny_list = pty_mgr.deny_list.lock().unwrap();
        for denied in deny_list.iter() {
            if data.contains(denied.as_str()) {
                return Err(format!("blocked by deny-list: command contains '{denied}'"));
            }
        }
    }

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
// Session persistence & API
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize)]
struct PtySessionInfo {
    id: String,
    active: bool,
}

/// List all active PTY sessions (for session persistence / detach-reattach).
#[tauri::command]
fn list_pty_sessions(pty_mgr: tauri::State<PtyManager>) -> Vec<PtySessionInfo> {
    pty_mgr
        .sessions
        .lock()
        .unwrap()
        .keys()
        .map(|id| PtySessionInfo {
            id: id.clone(),
            active: true,
        })
        .collect()
}

/// Save session metadata for persistence across app restarts.
#[tauri::command]
fn save_session_metadata(sessions: Vec<serde_json::Value>) -> Result<(), String> {
    let path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".smux")
        .join("sessions.json");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let content = serde_json::to_string_pretty(&sessions).map_err(|e| format!("serialize: {e}"))?;
    std::fs::write(&path, content).map_err(|e| format!("write: {e}"))
}

/// Load saved session metadata.
#[tauri::command]
fn load_session_metadata() -> Vec<serde_json::Value> {
    let path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".smux")
        .join("sessions.json");
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
        Err(_) => vec![],
    }
}

/// Socket API handler — execute a JSON-RPC style command.
/// This enables external tools/agents to control smux programmatically.
/// SECURITY: Only accessible from the main window (not the browser WebView).
#[tauri::command]
fn api_exec(
    window: tauri::Window,
    pty_mgr: tauri::State<PtyManager>,
    method: String,
    params: serde_json::Value,
) -> Result<serde_json::Value, String> {
    // Reject calls from the browser window to prevent XSS-to-RCE
    if window.label() != "main" {
        return Err("api_exec is only accessible from the main window".into());
    }
    match method.as_str() {
        "session.list" => {
            let sessions: Vec<String> = pty_mgr.sessions.lock().unwrap().keys().cloned().collect();
            Ok(serde_json::json!({ "sessions": sessions }))
        }
        "pane.write" => {
            let tab_id = params["id"].as_str().ok_or("missing id")?.to_string();
            let data = params["data"].as_str().ok_or("missing data")?.to_string();
            // Enforce deny-list from cache (same as write_pty)
            if data.contains('\n') || data.contains('\r') {
                let deny_list = pty_mgr.deny_list.lock().unwrap();
                for denied in deny_list.iter() {
                    if data.contains(denied.as_str()) {
                        return Err(format!("blocked by deny-list: '{denied}'"));
                    }
                }
            }
            let sessions = pty_mgr.sessions.lock().unwrap();
            let session = sessions.get(&tab_id).ok_or("session not found")?;
            session
                .writer
                .lock()
                .unwrap()
                .write_all(data.as_bytes())
                .map_err(|e| format!("write: {e}"))?;
            Ok(serde_json::json!({ "ok": true }))
        }
        "pane.close" => {
            let tab_id = params["id"].as_str().ok_or("missing id")?.to_string();
            pty_mgr.sessions.lock().unwrap().remove(&tab_id);
            Ok(serde_json::json!({ "ok": true }))
        }
        _ => Err(format!("unknown method: {method}")),
    }
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
// Agent detection
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize)]
struct AgentStatus {
    name: String,
    installed: bool,
    path: Option<String>,
}

#[tauri::command]
async fn detect_agents() -> Vec<AgentStatus> {
    let agents = ["claude", "codex", "gemini"];
    let mut results = Vec::new();
    for agent in &agents {
        let output = tokio::process::Command::new("which")
            .arg(agent)
            .output()
            .await;
        let (installed, path) = match output {
            Ok(o) if o.status.success() => {
                let p = String::from_utf8_lossy(&o.stdout).trim().to_string();
                (true, Some(p))
            }
            _ => (false, None),
        };
        results.push(AgentStatus {
            name: agent.to_string(),
            installed,
            path,
        });
    }
    results
}

// ---------------------------------------------------------------------------
// File explorer commands
// ---------------------------------------------------------------------------

#[derive(Clone, Serialize)]
struct DirEntry {
    name: String,
    path: String,
    is_dir: bool,
}

#[tauri::command]
fn list_directory(path: String) -> Result<Vec<DirEntry>, String> {
    let dir = std::path::Path::new(&path);
    if !dir.is_dir() {
        return Err(format!("not a directory: {path}"));
    }
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir).map_err(|e| format!("read_dir: {e}"))? {
        let entry = entry.map_err(|e| format!("entry: {e}"))?;
        let name = entry.file_name().to_string_lossy().to_string();
        let file_path = entry.path().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        entries.push(DirEntry {
            name,
            path: file_path,
            is_dir,
        });
    }
    Ok(entries)
}

#[tauri::command]
fn read_file(path: String) -> Result<String, String> {
    // Limit file size to 1MB to prevent OOM
    let metadata = std::fs::metadata(&path).map_err(|e| format!("metadata: {e}"))?;
    if metadata.len() > 1_048_576 {
        return Err("file too large (>1MB)".into());
    }
    std::fs::read_to_string(&path).map_err(|e| format!("read: {e}"))
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
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
            load_app_config,
            save_app_config,
            list_pty_sessions,
            list_directory,
            read_file,
            detect_agents,
            save_session_metadata,
            load_session_metadata,
            api_exec,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
