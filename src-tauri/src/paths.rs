use std::path::PathBuf;

use tauri::{AppHandle, Manager};

/// All resolved paths for bundled/external tools.
/// Initialized at app startup and stored as Tauri managed state.
#[derive(Debug, Clone)]
pub struct BundledPaths {
    pub adb: PathBuf,
    pub scrcpy_server: PathBuf,
    pub java: PathBuf,
    pub keytool: PathBuf,
    pub apktool_jar: PathBuf,
    pub apksigner_jar: PathBuf,
    pub zipalign: PathBuf,
}

impl BundledPaths {
    /// Resolve all tool paths. In production, look inside the app bundle's resources.
    /// In dev mode, fall back to system PATH.
    pub fn resolve(app: &AppHandle) -> Self {
        let adb = resolve_binary(app, "adb");
        let zipalign = resolve_binary(app, "zipalign");
        let java = resolve_jre_binary(app, "java");
        let keytool = resolve_jre_binary(app, "keytool");
        let scrcpy_server = resolve_resource(app, "scrcpy-server.jar");
        let apktool_jar = resolve_resource(app, "apktool.jar");
        let apksigner_jar = resolve_resource(app, "apksigner.jar");

        log::info!("[paths] adb: {:?}", adb);
        log::info!("[paths] scrcpy-server: {:?}", scrcpy_server);
        log::info!("[paths] java: {:?}", java);
        log::info!("[paths] keytool: {:?}", keytool);
        log::info!("[paths] apktool.jar: {:?}", apktool_jar);
        log::info!("[paths] apksigner.jar: {:?}", apksigner_jar);
        log::info!("[paths] zipalign: {:?}", zipalign);

        Self {
            adb,
            scrcpy_server,
            java,
            keytool,
            apktool_jar,
            apksigner_jar,
            zipalign,
        }
    }
}

/// Resolve a native binary from the bundled resources/bin/ directory.
/// Falls back to the bare name (PATH lookup) in dev mode.
fn resolve_binary(app: &AppHandle, name: &str) -> PathBuf {
    let bin_name = if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };

    // Try bundled path: resources/bin/<binary>
    if let Ok(path) = app
        .path()
        .resolve(format!("resources/bin/{}", bin_name), tauri::path::BaseDirectory::Resource)
    {
        if path.exists() {
            return path;
        }
    }

    // Dev fallback: bare name (resolved via PATH)
    PathBuf::from(&bin_name)
}

/// Resolve a JRE binary from the bundled JRE.
/// Falls back to system PATH.
fn resolve_jre_binary(app: &AppHandle, name: &str) -> PathBuf {
    let bin_name = if cfg!(windows) {
        format!("{}.exe", name)
    } else {
        name.to_string()
    };

    // Try bundled JRE: resources/jre/bin/<binary>
    if let Ok(path) = app
        .path()
        .resolve(format!("resources/jre/bin/{}", bin_name), tauri::path::BaseDirectory::Resource)
    {
        if path.exists() {
            return path;
        }
    }

    // Dev fallback: bare name
    PathBuf::from(&bin_name)
}

/// Resolve a JAR or other resource file from the bundle.
/// Falls back to common system locations in dev mode.
fn resolve_resource(app: &AppHandle, name: &str) -> PathBuf {
    // Try bundled path: resources/<name>
    if let Ok(path) = app
        .path()
        .resolve(format!("resources/{}", name), tauri::path::BaseDirectory::Resource)
    {
        if path.exists() {
            return path;
        }
    }

    // Dev fallback: try common locations
    match name {
        "scrcpy-server.jar" => find_scrcpy_server(),
        "apktool.jar" => find_jar("apktool"),
        "apksigner.jar" => find_jar("apksigner"),
        _ => PathBuf::from(name),
    }
}

/// Find scrcpy-server in common install locations (dev mode).
fn find_scrcpy_server() -> PathBuf {
    let candidates = if cfg!(target_os = "macos") {
        vec![
            PathBuf::from("/opt/homebrew/share/scrcpy/scrcpy-server"),
            PathBuf::from("/usr/local/share/scrcpy/scrcpy-server"),
        ]
    } else {
        // Windows: scrcpy is typically installed alongside the executable
        vec![
            // Common Windows install paths
            dirs::data_local_dir()
                .unwrap_or_default()
                .join("scrcpy/scrcpy-server"),
            PathBuf::from("C:/scrcpy/scrcpy-server"),
        ]
    };

    for path in candidates {
        if path.exists() {
            return path;
        }
    }

    PathBuf::from("scrcpy-server")
}

/// Find a JAR by looking for the wrapper script and resolving the actual jar (dev mode).
fn find_jar(name: &str) -> PathBuf {
    // On macOS with Homebrew, JARs are often at predictable paths
    if cfg!(target_os = "macos") {
        let brew_candidates = [
            format!("/opt/homebrew/opt/{}/libexec/{}.jar", name, name),
            format!("/opt/homebrew/Cellar/{}/*/libexec/{}.jar", name, name),
            format!("/usr/local/opt/{}/libexec/{}.jar", name, name),
        ];
        for pattern in &brew_candidates {
            // Simple glob for the Cellar path
            if let Ok(entries) = glob::glob(pattern) {
                for entry in entries.flatten() {
                    if entry.exists() {
                        return entry;
                    }
                }
            }
            let path = PathBuf::from(pattern);
            if path.exists() {
                return path;
            }
        }
    }

    PathBuf::from(format!("{}.jar", name))
}
