# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Dokky** is a desktop application **dedicated exclusively to Dofus Touch** multi-accounting. It detects USB-connected Android devices, clones the Dofus Touch APK for independent accounts, opens multiple game instances on virtual displays (`--new-display`), and presents each instance as a tab (Chrome-style) with embedded video rendering and touch control.

**Important**: This app is NOT a generic Android mirroring tool. Every feature, UI decision, and optimization is focused on the Dofus Touch gaming experience.

## Stack

- **Backend**: Tauri v2 + Rust (`src-tauri/`)
- **Frontend**: Vue 3 + TypeScript (`src/`)
- **Build**: Vite
- **External deps**: `adb` and `scrcpy-server` (bundled with scrcpy) must be available. scrcpy CLI is NOT used — the app communicates directly with scrcpy-server.

## Commands

```bash
# Development (launches both Vite dev server + Tauri window)
cargo tauri dev

# Type-check TypeScript
npx vue-tsc --noEmit

# Check Rust compilation
cd src-tauri && cargo check

# Run Rust unit tests
cd src-tauri && cargo test

# Production build
cargo tauri build
```

## Architecture

### Rust Backend (`src-tauri/src/`)

| File                  | Role                                                            |
|-----------------------|-----------------------------------------------------------------|
| `lib.rs`              | Tauri app builder, command registration, state management       |
| `device_manager.rs`   | `adb devices -l` parsing, `list_devices()` async function       |
| `session_manager.rs`  | Session lifecycle, scrcpy connection ownership, video/control stream management |
| `scrcpy_server.rs`    | Direct scrcpy-server protocol: push jar, start server, TCP handshake, video packet reading, touch event sending |
| `error.rs`            | `DokkyError` enum — serializable error type for Tauri IPC       |

### scrcpy-server Protocol (`scrcpy_server.rs`)

The app communicates directly with `scrcpy-server` on the Android device (no scrcpy CLI):

1. Push `scrcpy-server.jar` to device via `adb push`
2. Set up ADB forward tunnel: `adb forward tcp:0 localabstract:scrcpy_<scid>`
3. Start server: `adb shell CLASSPATH=... app_process / com.genymobile.scrcpy.Server 3.3.4 ...`
4. Connect video socket → read dummy byte → 64-byte device name → 12-byte codec metadata → H.264 packets
5. Connect control socket → send touch/key events (binary protocol, big-endian)

**Video packet format**: 12-byte header (PTS+flags as u64 BE + size as u32 BE) then H.264 Annex B data. Bit 63 of PTS = config flag, bit 62 = keyframe flag.

### Session State Management

Sessions are stored in `SessionStore` (a `Mutex<HashMap>`) as Tauri managed state. Each session holds:
- `SessionInfo` (serializable metadata sent to frontend)
- `SessionConnection` (video TcpStream, control Arc<AsyncMutex<TcpStream>>, server Child process)

**Critical pattern**: The std::sync::Mutex must never be held across `.await` points. Lock, extract/clone, drop, then await. The control stream uses `tokio::sync::Mutex` since it's held across async writes.

### Vue Frontend (`src/`)

| File/Dir                       | Role                                                 |
|--------------------------------|------------------------------------------------------|
| `composables/useDevices.ts`    | Polls `get_devices` every 3s                         |
| `composables/useSessions.ts`   | Session CRUD + tab switching, shared reactive state   |
| `composables/useShortcuts.ts`  | Global keyboard shortcuts (Ctrl+T/W/1-9/Tab)         |
| `components/VideoPlayer.vue`   | WebCodecs H.264 decoder → canvas rendering + mouse→touch |
| `components/TabBar.vue`        | Chrome-style tab bar                                 |
| `components/NewSessionDialog.vue` | Modal: pick device + app package                  |
| `types/index.ts`               | TypeScript types mirroring Rust structs               |

### Video Pipeline

1. Rust reads H.264 packets from scrcpy-server TCP socket
2. Packets are base64-encoded and sent to frontend via Tauri `Channel`
3. Frontend decodes base64, parses SPS for codec string, configures `VideoDecoder` (WebCodecs)
4. Decoded `VideoFrame`s are drawn to `<canvas>` via `drawImage()`
5. Mouse events on canvas are translated to scrcpy touch coordinates and sent via `send_touch` command

### IPC (Tauri Commands)

- `get_devices()` → `Vec<Device>`
- `create_session(device_serial, app_package, display_spec?)` → `SessionInfo`
- `list_sessions()` → `Vec<SessionInfo>`
- `stop_session(session_id)` → `()`
- `start_video_stream(session_id, on_packet: Channel)` → streams `VideoPacketPayload` indefinitely
- `send_touch(session_id, action, x, y, width, height)` → `()`

## Key Concept

One tab = one session = one virtual display on a device. Video is rendered directly in the app via WebCodecs, not in a separate scrcpy window.

## Known Constraints

- `--new-display` is not stable on all Android devices/ROMs
- Some games may refuse virtual displays
- Performance limited by CPU (video decoding), USB bandwidth, and per-device session count
- scrcpy-server protocol is internal and may change between versions (currently pinned to 3.3.4)
