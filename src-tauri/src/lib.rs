mod device_manager;
mod error;
mod scrcpy_server;
mod session_manager;

use base64::Engine;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::Manager;

use error::DokkiError;
use session_manager::SessionStore;

#[tauri::command]
async fn get_devices() -> Result<Vec<device_manager::Device>, DokkiError> {
    device_manager::list_devices().await
}

#[tauri::command]
async fn get_packages(device_serial: String, filter: String) -> Result<Vec<String>, DokkiError> {
    device_manager::list_packages(&device_serial, &filter).await
}

#[tauri::command]
async fn create_session(
    state: tauri::State<'_, SessionStore>,
    device_serial: String,
    app_package: String,
    display_spec: Option<String>,
) -> Result<session_manager::SessionInfo, DokkiError> {
    log::info!("[cmd] create_session: device={}, app={}", device_serial, app_package);
    let result = session_manager::create_session(
        &state,
        device_serial,
        app_package,
        display_spec.unwrap_or_else(|| "1920x1080".to_string()),
    )
    .await;
    match &result {
        Ok(info) => log::info!("[cmd] Session created: id={}, {}x{}", info.id, info.width, info.height),
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
) -> Result<(), DokkiError> {
    session_manager::stop_session(&state, &session_id).await
}

/// Payload sent to the frontend for each video packet.
#[derive(Clone, Serialize)]
struct VideoPacketPayload {
    /// true if this is a codec config packet (SPS/PPS)
    config: bool,
    /// true if this is a keyframe (IDR)
    keyframe: bool,
    /// presentation timestamp in microseconds
    pts: i64,
    /// base64-encoded H.264 Annex B data
    data: String,
}

/// Stream video packets from a session to the frontend via Channel.
/// This command runs indefinitely until the session is stopped or an error occurs.
#[tauri::command]
async fn start_video_stream(
    state: tauri::State<'_, SessionStore>,
    session_id: String,
    on_packet: Channel<VideoPacketPayload>,
) -> Result<(), DokkiError> {
    log::info!("[cmd] start_video_stream: session={}", session_id);
    let mut video_stream = session_manager::take_video_stream(&state, &session_id)?;

    let mut frame_count: u64 = 0;
    loop {
        match scrcpy_server::read_video_packet(&mut video_stream).await {
            Ok(packet) => {
                frame_count += 1;
                if frame_count <= 3 || frame_count % 100 == 0 {
                    log::info!(
                        "[video] packet #{}: config={} keyframe={} size={} pts={}",
                        frame_count, packet.is_config, packet.is_keyframe,
                        packet.data.len(), packet.pts_us
                    );
                }
                let payload = VideoPacketPayload {
                    config: packet.is_config,
                    keyframe: packet.is_keyframe,
                    pts: packet.pts_us,
                    data: base64::engine::general_purpose::STANDARD.encode(&packet.data),
                };
                if on_packet.send(payload).is_err() {
                    log::warn!("[video] Channel closed, stopping stream");
                    break;
                }
            }
            Err(e) => {
                log::error!("[video] Stream error: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Send a touch event to a session's scrcpy control channel.
#[tauri::command]
async fn send_touch(
    state: tauri::State<'_, SessionStore>,
    session_id: String,
    action: u8,
    x: f64,
    y: f64,
    width: u16,
    height: u16,
) -> Result<(), DokkiError> {
    let control = session_manager::get_control(&state, &session_id)?;
    scrcpy_server::send_touch(
        &control,
        action,
        x as i32,
        y as i32,
        width,
        height,
    )
    .await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let store = SessionStore::new();

    tauri::Builder::default()
        .manage(store)
        .plugin(
            tauri_plugin_log::Builder::default()
                .level(log::LevelFilter::Debug)
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Stdout,
                ))
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_devices,
            get_packages,
            create_session,
            list_sessions,
            stop_session,
            start_video_stream,
            send_touch,
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
