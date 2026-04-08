use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum DokkyError {
    AdbNotFound,
    AdbCommandFailed(String),
    ScrcpyLaunchFailed(String),
    SessionNotFound(String),
    ApkCloneFailed(String),
    ToolNotFound(String),
}

impl std::fmt::Display for DokkyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DokkyError::AdbNotFound => write!(f, "adb not found in PATH"),
            DokkyError::AdbCommandFailed(msg) => write!(f, "adb command failed: {}", msg),
            DokkyError::ScrcpyLaunchFailed(msg) => write!(f, "scrcpy launch failed: {}", msg),
            DokkyError::SessionNotFound(id) => write!(f, "session not found: {}", id),
            DokkyError::ApkCloneFailed(msg) => write!(f, "APK clone failed: {}", msg),
            DokkyError::ToolNotFound(tool) => write!(f, "{} not found in PATH", tool),
        }
    }
}
