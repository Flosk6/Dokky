use std::path::Path;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::{Child, Command};
use tokio::sync::Mutex as AsyncMutex;
use tokio::time::{timeout, Duration};

use uuid::Uuid;

use crate::error::DokkyError;

const SCRCPY_VERSION: &str = "3.3.4";
const DEVICE_NAME_LEN: usize = 64;

pub struct ScrcpyConnection {
    pub device_name: String,
    pub width: u32,
    pub height: u32,
    pub video_stream: TcpStream,
    pub control_stream: Arc<AsyncMutex<TcpStream>>,
    pub server_process: Child,
    pub local_port: u16,
    pub device_serial: String,
}

pub struct VideoPacket {
    pub is_config: bool,
    pub is_keyframe: bool,
    pub pts_us: i64,
    pub data: Vec<u8>,
}

/// Start scrcpy-server on a device and connect using REVERSE tunnel
/// (same method as the official scrcpy client).
///
/// Reverse tunnel: PC listens on a port, device connects TO the PC.
/// This is the default and most reliable mode.
/// Extra video codec options for scrcpy-server.
pub struct VideoCodecOptions {
    pub iframe_interval: u32,
}

impl Default for VideoCodecOptions {
    fn default() -> Self {
        Self {
            iframe_interval: 2,
        }
    }
}

pub async fn connect(
    adb: &Path,
    server_jar: &Path,
    device_serial: &str,
    app_package: &str,
    display_spec: &str,
    video_bit_rate: u32,
    max_fps: u32,
    codec_options: &VideoCodecOptions,
) -> Result<ScrcpyConnection, DokkyError> {
    let scid = (Uuid::new_v4().as_u128() as u32) & 0x7FFF_FFFF;

    // 1. Push server jar to device
    log::info!("[scrcpy] Pushing server jar to device {}...", device_serial);
    let push = Command::new(adb)
        .args(["-s", device_serial, "push",
            server_jar.to_str().unwrap(),
            "/data/local/tmp/scrcpy-server.jar"])
        .output().await
        .map_err(|_| DokkyError::AdbNotFound)?;

    if !push.status.success() {
        let err = String::from_utf8_lossy(&push.stderr).to_string();
        log::error!("[scrcpy] Push failed: {}", err);
        return Err(DokkyError::AdbCommandFailed(err));
    }
    log::info!("[scrcpy] Push OK");

    // 2. Bind a local TCP port (PC listens, device will connect to it)
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| DokkyError::ScrcpyLaunchFailed(format!("bind listener: {}", e)))?;
    let local_port = listener.local_addr().unwrap().port();
    log::info!("[scrcpy] Listening on port {}", local_port);

    // 3. Set up ADB reverse tunnel: device connects to our local port
    let reverse = Command::new(adb)
        .args(["-s", device_serial, "reverse",
            &format!("localabstract:scrcpy_{:08x}", scid),
            &format!("tcp:{}", local_port)])
        .output().await
        .map_err(|_| DokkyError::AdbNotFound)?;

    if !reverse.status.success() {
        let err = String::from_utf8_lossy(&reverse.stderr).to_string();
        log::error!("[scrcpy] Reverse tunnel failed: {}", err);
        return Err(DokkyError::AdbCommandFailed(err));
    }
    log::info!("[scrcpy] Reverse tunnel set up: scrcpy_{:08x} -> tcp:{}", scid, local_port);

    // 4. Start scrcpy-server (NO tunnel_forward — uses reverse by default)
    // Build video_codec_options string
    // Build video_codec_options for the encoder
    let mut vco_parts: Vec<String> = Vec::new();
    vco_parts.push(format!("i-frame-interval={}", codec_options.iframe_interval));

    let mut server_cmd = format!(
        "CLASSPATH=/data/local/tmp/scrcpy-server.jar \
         app_process / com.genymobile.scrcpy.Server {} \
         scid={:08x} \
         log_level=info \
         audio=false \
         video_codec=h264 \
         max_fps={} \
         video_bit_rate={} \
         new_display={}",
        SCRCPY_VERSION, scid, max_fps, video_bit_rate, display_spec
    );

    if !vco_parts.is_empty() {
        server_cmd.push_str(&format!(" video_codec_options={}", vco_parts.join(",")));
    }
    // Note: screen_off_timeout is not used — it breaks touch input.
    // Screen dimming is handled via ADB brightness commands instead.
    log::info!("[scrcpy] Starting server on device...");

    let mut server_process = Command::new(adb)
        .args(["-s", device_serial, "shell", &server_cmd])
        .kill_on_drop(true)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| DokkyError::ScrcpyLaunchFailed(e.to_string()))?;

    // Log server output in background
    if let Some(stdout) = server_process.stdout.take() {
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut lines = tokio::io::BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::info!("[scrcpy-server] {}", line);
            }
        });
    }
    if let Some(stderr) = server_process.stderr.take() {
        tokio::spawn(async move {
            use tokio::io::AsyncBufReadExt;
            let mut lines = tokio::io::BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                log::warn!("[scrcpy-server stderr] {}", line);
            }
        });
    }

    // 5. Accept video connection (device connects to us)
    log::info!("[scrcpy] Waiting for video connection from device...");
    let (mut video_stream, _) = timeout(Duration::from_secs(10), listener.accept())
        .await
        .map_err(|_| DokkyError::ScrcpyLaunchFailed("timeout waiting for video connection".to_string()))?
        .map_err(|e| DokkyError::ScrcpyLaunchFailed(format!("accept video: {}", e)))?;
    log::info!("[scrcpy] Video socket connected!");

    // 6. Read video handshake: device_name(64) + codec_id(4) + width(4) + height(4)
    //    (NO dummy byte in reverse mode)
    let mut name_buf = [0u8; DEVICE_NAME_LEN];
    timeout(Duration::from_secs(5), video_stream.read_exact(&mut name_buf))
        .await
        .map_err(|_| DokkyError::ScrcpyLaunchFailed("timeout reading device name".to_string()))?
        .map_err(|e| DokkyError::ScrcpyLaunchFailed(format!("device name: {}", e)))?;
    let device_name = String::from_utf8_lossy(&name_buf).trim_end_matches('\0').to_string();
    log::info!("[scrcpy] Device name: '{}'", device_name);

    let mut meta = [0u8; 12];
    timeout(Duration::from_secs(5), video_stream.read_exact(&mut meta))
        .await
        .map_err(|_| DokkyError::ScrcpyLaunchFailed("timeout reading codec metadata".to_string()))?
        .map_err(|e| DokkyError::ScrcpyLaunchFailed(format!("codec metadata: {}", e)))?;

    let codec_id = u32::from_be_bytes([meta[0], meta[1], meta[2], meta[3]]);
    let width = u32::from_be_bytes([meta[4], meta[5], meta[6], meta[7]]);
    let height = u32::from_be_bytes([meta[8], meta[9], meta[10], meta[11]]);
    log::info!("[scrcpy] Codec={} Resolution={}x{}", codec_id, width, height);

    // 7. Accept control connection (second connection from device)
    log::info!("[scrcpy] Waiting for control connection...");
    let (control_stream, _) = timeout(Duration::from_secs(5), listener.accept())
        .await
        .map_err(|_| DokkyError::ScrcpyLaunchFailed("timeout waiting for control connection".to_string()))?
        .map_err(|e| DokkyError::ScrcpyLaunchFailed(format!("accept control: {}", e)))?;
    log::info!("[scrcpy] Control socket connected!");

    // 8. Launch the app on the virtual display via scrcpy START_APP
    let control_arc = Arc::new(AsyncMutex::new(control_stream));
    if !app_package.is_empty() {
        log::info!("[scrcpy] Sending START_APP for '{}'", app_package);
        send_start_app(&control_arc, app_package).await?;
    }

    log::info!(
        "[scrcpy] Fully connected! device='{}' {}x{} port={}",
        device_name, width, height, local_port
    );

    Ok(ScrcpyConnection {
        device_name,
        width,
        height,
        video_stream,
        control_stream: control_arc,
        server_process,
        local_port,
        device_serial: device_serial.to_string(),
    })
}

/// Read one video packet from the stream.
pub async fn read_video_packet(stream: &mut TcpStream) -> Result<VideoPacket, DokkyError> {
    let mut header = [0u8; 12];
    stream.read_exact(&mut header).await
        .map_err(|e| DokkyError::ScrcpyLaunchFailed(format!("video header: {}", e)))?;

    let raw = u64::from_be_bytes([
        header[0], header[1], header[2], header[3],
        header[4], header[5], header[6], header[7],
    ]);
    let packet_size = u32::from_be_bytes([header[8], header[9], header[10], header[11]]);

    let is_config = (raw >> 63) == 1;
    let is_keyframe = ((raw >> 62) & 1) == 1;
    let pts_us = (raw & ((1u64 << 62) - 1)) as i64;

    let mut data = vec![0u8; packet_size as usize];
    stream.read_exact(&mut data).await
        .map_err(|e| DokkyError::ScrcpyLaunchFailed(format!("video data: {}", e)))?;

    Ok(VideoPacket { is_config, is_keyframe, pts_us, data })
}

/// Send START_APP control message (type 0x10).
/// Format: [0x10][name_length: u8][name: UTF-8 bytes]
pub async fn send_start_app(
    control: &Arc<AsyncMutex<TcpStream>>,
    app_package: &str,
) -> Result<(), DokkyError> {
    let name_bytes = app_package.as_bytes();
    if name_bytes.len() > 255 {
        return Err(DokkyError::ScrcpyLaunchFailed("app package name too long".to_string()));
    }

    let mut msg = Vec::with_capacity(2 + name_bytes.len());
    msg.push(0x10);
    msg.push(name_bytes.len() as u8);
    msg.extend_from_slice(name_bytes);

    let mut stream = control.lock().await;
    stream.write_all(&msg).await
        .map_err(|e| DokkyError::ScrcpyLaunchFailed(format!("send start_app: {}", e)))?;

    log::info!("[scrcpy] START_APP sent ({} bytes)", msg.len());
    Ok(())
}

/// Send a touch event on the control socket.
pub async fn send_touch(
    control: &Arc<AsyncMutex<TcpStream>>,
    action: u8,
    x: i32,
    y: i32,
    screen_width: u16,
    screen_height: u16,
) -> Result<(), DokkyError> {
    let mut msg = [0u8; 32];
    msg[0] = 0x02; // INJECT_TOUCH_EVENT
    msg[1] = action;
    msg[2..10].copy_from_slice(&(-1i64).to_be_bytes());
    msg[10..14].copy_from_slice(&x.to_be_bytes());
    msg[14..18].copy_from_slice(&y.to_be_bytes());
    msg[18..20].copy_from_slice(&screen_width.to_be_bytes());
    msg[20..22].copy_from_slice(&screen_height.to_be_bytes());
    let pressure: u16 = if action == 1 { 0 } else { 0xFFFF };
    msg[22..24].copy_from_slice(&pressure.to_be_bytes());
    msg[24..28].copy_from_slice(&1u32.to_be_bytes());
    let button_state: u32 = if action == 1 { 0 } else { 1 };
    msg[28..32].copy_from_slice(&button_state.to_be_bytes());

    let mut stream = control.lock().await;
    stream.write_all(&msg).await
        .map_err(|e| DokkyError::ScrcpyLaunchFailed(format!("send touch: {}", e)))?;

    Ok(())
}

/// Send an INJECT_KEYCODE control message (type 0x00).
/// Format: [type:1][action:1][keycode:4BE][repeat:4BE][metastate:4BE] = 14 bytes
pub async fn send_key(
    control: &Arc<AsyncMutex<TcpStream>>,
    action: u8,
    keycode: u32,
    repeat: u32,
    metastate: u32,
) -> Result<(), DokkyError> {
    let mut msg = [0u8; 14];
    msg[0] = 0x00; // INJECT_KEYCODE
    msg[1] = action;
    msg[2..6].copy_from_slice(&keycode.to_be_bytes());
    msg[6..10].copy_from_slice(&repeat.to_be_bytes());
    msg[10..14].copy_from_slice(&metastate.to_be_bytes());

    let mut stream = control.lock().await;
    stream.write_all(&msg).await
        .map_err(|e| DokkyError::ScrcpyLaunchFailed(format!("send key: {}", e)))?;

    Ok(())
}

/// Send an INJECT_TEXT control message (type 0x01).
/// Format: [type:1][length:4BE][text:variable UTF-8]
pub async fn send_text(
    control: &Arc<AsyncMutex<TcpStream>>,
    text: &str,
) -> Result<(), DokkyError> {
    let text_bytes = text.as_bytes();
    let mut msg = Vec::with_capacity(5 + text_bytes.len());
    msg.push(0x01); // INJECT_TEXT
    msg.extend_from_slice(&(text_bytes.len() as u32).to_be_bytes());
    msg.extend_from_slice(text_bytes);

    let mut stream = control.lock().await;
    stream.write_all(&msg).await
        .map_err(|e| DokkyError::ScrcpyLaunchFailed(format!("send text: {}", e)))?;

    Ok(())
}

/// Remove the ADB reverse tunnel.
pub async fn remove_reverse(adb: &Path, device_serial: &str, scid: u32) {
    let _ = Command::new(adb)
        .args(["-s", device_serial, "reverse", "--remove",
            &format!("localabstract:scrcpy_{:08x}", scid)])
        .output().await;
}
