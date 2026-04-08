use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tokio::net::TcpStream;
use tokio::process::Child;
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

use crate::error::DokkyError;
use crate::paths::BundledPaths;
use crate::scrcpy_server;

#[derive(Debug, Clone, Serialize)]
pub struct SessionInfo {
    pub id: String,
    pub device_serial: String,
    pub app_package: String,
    pub display_name: String,
    pub display_spec: String,
    pub width: u32,
    pub height: u32,
    pub status: SessionStatus,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum SessionStatus {
    Starting,
    Running,
    Stopped,
    Error(String),
}

/// Internal connection state (not serializable).
struct SessionConnection {
    video_stream: Arc<AsyncMutex<TcpStream>>,
    control: Arc<AsyncMutex<TcpStream>>,
    server_process: Child,
}

struct SessionEntry {
    info: SessionInfo,
    connection: Option<SessionConnection>,
}

pub struct SessionStore {
    sessions: Mutex<HashMap<String, SessionEntry>>,
}

impl SessionStore {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

/// Create a session by starting scrcpy-server directly (embedded video).
pub async fn create_session(
    store: &SessionStore,
    paths: &BundledPaths,
    device_serial: String,
    app_package: String,
    display_name: String,
    display_spec: String,
    video_bit_rate: u32,
    max_fps: u32,
) -> Result<SessionInfo, DokkyError> {
    let id = Uuid::new_v4().to_string();

    let conn = scrcpy_server::connect(
        &paths.adb,
        &paths.scrcpy_server,
        &device_serial,
        &app_package,
        &display_spec,
        video_bit_rate,
        max_fps,
    ).await?;

    let info = SessionInfo {
        id: id.clone(),
        device_serial: device_serial.clone(),
        app_package,
        display_name,
        display_spec,
        width: conn.width,
        height: conn.height,
        status: SessionStatus::Running,
    };

    let entry = SessionEntry {
        info: info.clone(),
        connection: Some(SessionConnection {
            video_stream: Arc::new(AsyncMutex::new(conn.video_stream)),
            control: conn.control_stream,
            server_process: conn.server_process,
        }),
    };

    store.sessions.lock().unwrap().insert(id, entry);

    Ok(info)
}

/// Get an Arc to the video stream for reading packets.
pub fn get_video_stream(
    store: &SessionStore,
    session_id: &str,
) -> Result<Arc<AsyncMutex<TcpStream>>, DokkyError> {
    let sessions = store.sessions.lock().unwrap();
    let entry = sessions
        .get(session_id)
        .ok_or_else(|| DokkyError::SessionNotFound(session_id.to_string()))?;
    let conn = entry
        .connection
        .as_ref()
        .ok_or_else(|| DokkyError::SessionNotFound(session_id.to_string()))?;
    Ok(conn.video_stream.clone())
}

/// Get a clone of the control stream Arc for a session.
pub fn get_control(
    store: &SessionStore,
    session_id: &str,
) -> Result<Arc<AsyncMutex<TcpStream>>, DokkyError> {
    let sessions = store.sessions.lock().unwrap();
    let entry = sessions
        .get(session_id)
        .ok_or_else(|| DokkyError::SessionNotFound(session_id.to_string()))?;
    let conn = entry
        .connection
        .as_ref()
        .ok_or_else(|| DokkyError::SessionNotFound(session_id.to_string()))?;
    Ok(conn.control.clone())
}

/// Lists all sessions.
pub fn list_sessions(store: &SessionStore) -> Vec<SessionInfo> {
    store
        .sessions
        .lock()
        .unwrap()
        .values()
        .map(|entry| entry.info.clone())
        .collect()
}

/// Stop a session: kill server process and clean up.
pub async fn stop_session(store: &SessionStore, session_id: &str) -> Result<(), DokkyError> {
    let connection = {
        let mut sessions = store.sessions.lock().unwrap();
        let entry = sessions
            .get_mut(session_id)
            .ok_or_else(|| DokkyError::SessionNotFound(session_id.to_string()))?;
        entry.info.status = SessionStatus::Stopped;
        entry.connection.take()
    };

    if let Some(mut conn) = connection {
        let _ = conn.server_process.kill().await;
        // Reverse tunnel cleanup not strictly needed (server exit cleans it up)
    }

    Ok(())
}

/// Kill all sessions. Called on app exit.
pub fn kill_all(store: &SessionStore) {
    let mut sessions = store.sessions.lock().unwrap();
    for entry in sessions.values_mut() {
        if let Some(ref mut conn) = entry.connection {
            let _ = conn.server_process.start_kill();
        }
        entry.info.status = SessionStatus::Stopped;
        entry.connection = None;
    }
}
