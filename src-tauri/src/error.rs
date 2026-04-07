use serde::Serialize;

#[derive(Debug, Serialize)]
pub enum DokkiError {
    AdbNotFound,
    AdbCommandFailed(String),
    ScrcpyNotFound,
    ScrcpyLaunchFailed(String),
    SessionNotFound(String),
}

impl std::fmt::Display for DokkiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DokkiError::AdbNotFound => write!(f, "adb not found in PATH"),
            DokkiError::AdbCommandFailed(msg) => write!(f, "adb command failed: {}", msg),
            DokkiError::ScrcpyNotFound => write!(f, "scrcpy not found in PATH"),
            DokkiError::ScrcpyLaunchFailed(msg) => write!(f, "scrcpy launch failed: {}", msg),
            DokkiError::SessionNotFound(id) => write!(f, "session not found: {}", id),
        }
    }
}
