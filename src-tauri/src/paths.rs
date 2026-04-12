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
        let adb = strip_unc_prefix(resolve_binary(app, "adb"));
        let zipalign = strip_unc_prefix(resolve_binary(app, "zipalign"));
        let java = strip_unc_prefix(resolve_jre_binary(app, "java"));
        let keytool = strip_unc_prefix(resolve_jre_binary(app, "keytool"));
        let scrcpy_server = strip_unc_prefix(resolve_resource(app, "scrcpy-server.jar"));
        let apktool_jar = strip_unc_prefix(resolve_resource(app, "apktool.jar"));
        let apksigner_jar = strip_unc_prefix(resolve_resource(app, "apksigner.jar"));

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
/// Falls back to common install locations, then to the bare name (PATH lookup) in dev mode.
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

    // Windows dev fallback: look in common Android Studio / Scoop / Chocolatey locations.
    #[cfg(windows)]
    {
        if let Some(path) = find_windows_binary(name) {
            return path;
        }
    }

    // Dev fallback: bare name (resolved via PATH)
    PathBuf::from(&bin_name)
}

/// Windows dev fallback: locate `adb.exe` and `zipalign.exe` in common SDK install paths.
#[cfg(windows)]
fn find_windows_binary(name: &str) -> Option<PathBuf> {
    let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let android_home = std::env::var_os("ANDROID_HOME").map(PathBuf::from);
    let android_sdk_root = std::env::var_os("ANDROID_SDK_ROOT").map(PathBuf::from);

    let sdk_roots: Vec<PathBuf> = [
        android_home,
        android_sdk_root,
        local_app_data.as_ref().map(|p| p.join("Android/Sdk")),
    ]
    .into_iter()
    .flatten()
    .collect();

    match name {
        "adb" => {
            for sdk in &sdk_roots {
                let candidate = sdk.join("platform-tools/adb.exe");
                if candidate.exists() {
                    return Some(candidate);
                }
            }
        }
        "zipalign" => {
            for sdk in &sdk_roots {
                if let Some(p) = latest_build_tools_binary(sdk, "zipalign.exe") {
                    return Some(p);
                }
            }
        }
        _ => {}
    }
    None
}

/// Pick the highest-versioned `build-tools/<version>/<binary>` under an Android SDK root.
#[cfg(windows)]
fn latest_build_tools_binary(sdk_root: &std::path::Path, binary: &str) -> Option<PathBuf> {
    let build_tools = sdk_root.join("build-tools");
    let mut versions: Vec<PathBuf> = std::fs::read_dir(&build_tools)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    versions.sort();
    for v in versions.into_iter().rev() {
        let candidate = v.join(binary);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Resolve a JRE binary from the bundled JRE.
/// Falls back to $JAVA_HOME, then system PATH.
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

    // Dev fallback: $JAVA_HOME/bin/<binary>
    if let Some(java_home) = std::env::var_os("JAVA_HOME") {
        let candidate = PathBuf::from(java_home).join("bin").join(&bin_name);
        if candidate.exists() {
            return candidate;
        }
    }

    // Dev fallback: bare name (resolved via PATH)
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
    let mut candidates: Vec<PathBuf> = Vec::new();

    if cfg!(target_os = "macos") {
        candidates.push(PathBuf::from("/opt/homebrew/share/scrcpy/scrcpy-server"));
        candidates.push(PathBuf::from("/usr/local/share/scrcpy/scrcpy-server"));
    } else if cfg!(windows) {
        // Scoop
        if let Some(home) = dirs::home_dir() {
            candidates.push(home.join("scoop/apps/scrcpy/current/scrcpy-server"));
        }
        // Chocolatey
        if let Ok(entries) = glob::glob("C:/ProgramData/chocolatey/lib/scrcpy/tools/scrcpy-*/scrcpy-server") {
            for e in entries.flatten() {
                candidates.push(e);
            }
        }
        // Fallback locations shipped alongside a manual scrcpy install
        if let Some(data_local) = dirs::data_local_dir() {
            candidates.push(data_local.join("scrcpy/scrcpy-server"));
        }
        candidates.push(PathBuf::from("C:/scrcpy/scrcpy-server"));
    }

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

/// Strip the Windows `\\?\` extended-length path prefix.
/// Java and some other tools don't handle this prefix.
/// Tauri's resource resolver adds it on Windows production builds.
fn strip_unc_prefix(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    if s.starts_with("\\\\?\\") {
        PathBuf::from(&s[4..])
    } else {
        p
    }
}
