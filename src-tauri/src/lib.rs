mod apk_manager;
mod config_manager;
mod device_manager;
mod error;
mod license_manager;
mod paths;
mod process_ext;
mod scrcpy_server;
mod session_manager;

use serde::Serialize;
use tauri::ipc::Response;
use tauri::Manager;

use error::DokkyError;
use paths::BundledPaths;
use process_ext::NoWindow;
use session_manager::SessionStore;

#[tauri::command]
async fn get_devices(
    paths: tauri::State<'_, BundledPaths>,
) -> Result<Vec<device_manager::Device>, DokkyError> {
    device_manager::list_devices(&paths.adb).await
}

#[tauri::command]
async fn get_packages(
    paths: tauri::State<'_, BundledPaths>,
    device_serial: String,
    filter: String,
) -> Result<Vec<String>, DokkyError> {
    device_manager::list_packages(&paths.adb, &device_serial, &filter).await
}

#[tauri::command]
async fn get_dofus_clones(
    paths: tauri::State<'_, BundledPaths>,
    device_serial: String,
) -> Result<Vec<apk_manager::CloneInfo>, DokkyError> {
    apk_manager::list_dofus_clones(&paths, &device_serial).await
}

#[tauri::command]
async fn clone_dofus(
    paths: tauri::State<'_, BundledPaths>,
    device_serial: String,
    clone_suffix: String,
    display_name: String,
    icon_color: Option<String>,
) -> Result<String, DokkyError> {
    log::info!(
        "[cmd] clone_dofus: device={}, suffix={}, name={}, color={:?}",
        device_serial, clone_suffix, display_name, icon_color
    );
    apk_manager::clone_dofus(
        &paths,
        &device_serial,
        &clone_suffix,
        &display_name,
        icon_color.as_deref(),
    )
    .await
}

#[tauri::command]
async fn get_dofus_icon(
    paths: tauri::State<'_, BundledPaths>,
    device_serial: String,
) -> Result<String, DokkyError> {
    apk_manager::get_dofus_icon(&paths, &device_serial).await
}

#[tauri::command]
async fn remove_dofus_clone(
    paths: tauri::State<'_, BundledPaths>,
    device_serial: String,
    package: String,
) -> Result<(), DokkyError> {
    log::info!("[cmd] remove_dofus_clone: {}", package);
    apk_manager::remove_clone(&paths, &device_serial, &package).await
}

#[tauri::command]
fn get_config() -> config_manager::AppConfig {
    config_manager::load_config()
}

#[tauri::command]
fn set_config(config: config_manager::AppConfig) -> Result<(), DokkyError> {
    config_manager::save_config(&config)
}

/// Check if the Android soft keyboard is currently visible on the device.
#[tauri::command]
async fn is_keyboard_visible(
    paths: tauri::State<'_, BundledPaths>,
    device_serial: String,
) -> Result<bool, DokkyError> {
    // Use grep on device side to minimize data transfer and speed up the check
    let output = tokio::process::Command::new(&paths.adb)
        .args(["-s", &device_serial, "shell", "dumpsys input_method | grep 'mServedInputConnection='"])
        .no_window()
        .output()
        .await
        .map_err(|_| DokkyError::AdbNotFound)?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // mServedInputConnection=null → no text field focused
    // mServedInputConnection=RemoteInputConnectionImpl{...} → text field has focus
    let has_input = !stdout.is_empty()
        && !stdout.contains("mServedInputConnection=null");
    Ok(has_input)
}

#[tauri::command]
async fn set_device_animations(
    paths: tauri::State<'_, BundledPaths>,
    device_serial: String,
    enabled: bool,
) -> Result<(), DokkyError> {
    let val = if enabled { "1.0" } else { "0" };
    for setting in &["window_animation_scale", "transition_animation_scale", "animator_duration_scale"] {
        let output = tokio::process::Command::new(&paths.adb)
            .args(["-s", &device_serial, "shell", "settings", "put", "global", setting, val])
            .no_window()
            .output()
            .await
            .map_err(|_| DokkyError::AdbNotFound)?;
        if !output.status.success() {
            log::warn!("[adb] Failed to set {}: {}", setting, String::from_utf8_lossy(&output.stderr));
        }
    }
    log::info!("[adb] Animations {} on {}", if enabled { "enabled" } else { "disabled" }, device_serial);
    Ok(())
}

/// Set device screen brightness to minimum (0) or restore to auto.
/// Unlike screen_off, this keeps the device awake and touch input working.
#[tauri::command]
async fn set_device_screen_dim(
    paths: tauri::State<'_, BundledPaths>,
    device_serial: String,
    dim: bool,
) -> Result<(), DokkyError> {
    if dim {
        // Switch to manual brightness and set to 0
        tokio::process::Command::new(&paths.adb)
            .args(["-s", &device_serial, "shell", "settings", "put", "system", "screen_brightness_mode", "0"])
            .no_window()
            .output().await.map_err(|_| DokkyError::AdbNotFound)?;
        tokio::process::Command::new(&paths.adb)
            .args(["-s", &device_serial, "shell", "settings", "put", "system", "screen_brightness", "0"])
            .no_window()
            .output().await.map_err(|_| DokkyError::AdbNotFound)?;
        log::info!("[adb] Screen dimmed to minimum on {}", device_serial);
    } else {
        // Restore auto brightness
        tokio::process::Command::new(&paths.adb)
            .args(["-s", &device_serial, "shell", "settings", "put", "system", "screen_brightness_mode", "1"])
            .no_window()
            .output().await.map_err(|_| DokkyError::AdbNotFound)?;
        log::info!("[adb] Screen brightness restored to auto on {}", device_serial);
    }
    Ok(())
}

#[tauri::command]
async fn check_license() -> license_manager::LicenseStatus {
    license_manager::validate_license().await
}

#[tauri::command]
async fn activate_license(license_key: String) -> license_manager::LicenseStatus {
    license_manager::activate_license(&license_key).await
}

#[tauri::command]
async fn deactivate_license() -> license_manager::LicenseStatus {
    license_manager::deactivate_license().await
}

#[tauri::command]
async fn create_session(
    state: tauri::State<'_, SessionStore>,
    paths: tauri::State<'_, BundledPaths>,
    device_serial: String,
    app_package: String,
    display_name: Option<String>,
    display_spec: Option<String>,
    video_bit_rate: Option<u32>,
    max_fps: Option<u32>,
    iframe_interval: Option<u32>,
) -> Result<session_manager::SessionInfo, DokkyError> {
    let spec = display_spec.unwrap_or_else(|| "1920x1080".to_string());
    let bitrate = video_bit_rate.unwrap_or(8_000_000);
    let fps = max_fps.unwrap_or(60);
    let name = display_name.unwrap_or_else(|| app_package.clone());
    let codec_options = scrcpy_server::VideoCodecOptions {
        iframe_interval: iframe_interval.unwrap_or(2),
    };
    log::info!(
        "[cmd] create_session: device={}, app={}, name={}, display={}, bitrate={}, fps={}",
        device_serial, app_package, name, spec, bitrate, fps
    );
    let result = session_manager::create_session(
        &state,
        &paths,
        device_serial,
        app_package,
        name,
        spec,
        bitrate,
        fps,
        codec_options,
    )
    .await;
    match &result {
        Ok(info) => log::info!(
            "[cmd] Session created: id={}, {}x{}",
            info.id, info.width, info.height
        ),
        Err(e) => log::error!("[cmd] Session creation failed: {}", e),
    }
    result
}

#[tauri::command]
fn list_sessions(state: tauri::State<'_, SessionStore>) -> Vec<session_manager::SessionInfo> {
    session_manager::list_sessions(&state)
}

#[tauri::command]
async fn stop_session(
    state: tauri::State<'_, SessionStore>,
    session_id: String,
) -> Result<(), DokkyError> {
    session_manager::stop_session(&state, &session_id).await
}

/// Video packet header returned as JSON metadata.
#[derive(Clone, Serialize)]
struct VideoPacketMeta {
    config: bool,
    keyframe: bool,
    pts: i64,
    size: u32,
}

/// Read the next video packet from a session. Returns raw H.264 bytes as Response
/// with metadata in a custom header. Called in a tight loop from the frontend.
#[tauri::command]
async fn read_video_packet(
    state: tauri::State<'_, SessionStore>,
    session_id: String,
) -> Result<Response, DokkyError> {
    let video_stream = session_manager::get_video_stream(&state, &session_id)?;
    let mut stream = video_stream.lock().await;
    let packet = scrcpy_server::read_video_packet(&mut stream).await?;

    // Pack metadata (13 bytes) + raw H.264 data into a single binary response:
    // [flags:1] [pts:8 BE] [size:4 BE] [data...]
    // flags: bit 0 = config, bit 1 = keyframe
    let mut buf = Vec::with_capacity(13 + packet.data.len());
    let flags: u8 = (packet.is_config as u8) | ((packet.is_keyframe as u8) << 1);
    buf.push(flags);
    buf.extend_from_slice(&packet.pts_us.to_be_bytes());
    buf.extend_from_slice(&(packet.data.len() as u32).to_be_bytes());
    buf.extend_from_slice(&packet.data);

    Ok(Response::new(buf))
}

#[tauri::command]
async fn send_touch(
    state: tauri::State<'_, SessionStore>,
    session_id: String,
    action: u8,
    x: f64,
    y: f64,
    width: u16,
    height: u16,
) -> Result<(), DokkyError> {
    let control = session_manager::get_control(&state, &session_id)?;
    scrcpy_server::send_touch(&control, action, x as i32, y as i32, width, height).await
}

#[tauri::command]
async fn send_key(
    state: tauri::State<'_, SessionStore>,
    session_id: String,
    action: u8,
    keycode: u32,
    repeat: u32,
    metastate: u32,
) -> Result<(), DokkyError> {
    let control = session_manager::get_control(&state, &session_id)?;
    scrcpy_server::send_key(&control, action, keycode, repeat, metastate).await
}

#[tauri::command]
async fn send_text(
    state: tauri::State<'_, SessionStore>,
    session_id: String,
    text: String,
) -> Result<(), DokkyError> {
    let control = session_manager::get_control(&state, &session_id)?;
    scrcpy_server::send_text(&control, &text).await
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = SessionStore::new();

    tauri::Builder::default()
        .manage(store)
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Debug)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("dokky".into()),
                    }),
                ])
                .build(),
        )
        .setup(|app| {
            let paths = BundledPaths::resolve(app.handle());
            app.manage(paths);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_devices,
            get_packages,
            get_dofus_clones,
            get_dofus_icon,
            clone_dofus,
            remove_dofus_clone,
            get_config,
            set_config,
            check_license,
            activate_license,
            deactivate_license,
            is_keyboard_visible,
            set_device_animations,
            set_device_screen_dim,
            create_session,
            list_sessions,
            stop_session,
            read_video_packet,
            send_touch,
            send_key,
            send_text,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                let store = app.state::<SessionStore>();
                session_manager::kill_all(&store);
            }
        });
}
