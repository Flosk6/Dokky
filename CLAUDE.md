# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Dokky** is a desktop application **dedicated exclusively to Dofus Touch** multi-accounting. It detects USB-connected Android devices, clones the Dofus Touch APK for independent accounts, opens multiple game instances on virtual displays (`--new-display`), and presents each instance as a tab (Chrome-style) with embedded video rendering and touch control.

**Important**: This app is NOT a generic Android mirroring tool. Every feature, UI decision, and optimization is focused on the Dofus Touch gaming experience.

## Stack

- **Backend**: Tauri v2 + Rust (`src-tauri/`)
- **Frontend**: Vue 3 + TypeScript (`src/`)
- **Build**: Vite
- **CI**: GitHub Actions (macOS + Windows builds)
- **External deps**: All bundled in the app — `adb`, `scrcpy-server`, `apktool`, `apksigner`, `zipalign`, minimal JRE (via jlink). scrcpy CLI is NOT used — the app communicates directly with scrcpy-server.

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

# Collect bundled dependencies (run before production build)
# Auto-detects macOS or Windows (Git Bash). Pass platform explicitly if needed.
bash scripts/collect-deps.sh            # auto-detect
bash scripts/collect-deps.sh windows    # force Windows branch

# Production build (.app + .dmg on macOS, .exe/.msi on Windows)
cargo tauri build
```

### Windows dev prerequisites

- **JDK 21** in PATH (for `jlink` — used to build the minimal bundled JRE).
- **Android SDK** for `apksigner.jar` and `zipalign.exe`: install Android Studio or set `ANDROID_HOME`.
- **Git Bash** (ships with Git for Windows) to run `scripts/collect-deps.sh`.
- `adb.exe`, `scrcpy-server.jar`, `apktool.jar` are downloaded by the script automatically (same URLs as the CI workflow).
- Logs are written to `%LOCALAPPDATA%\com.dokky.app\logs\dokky.log` (via `tauri-plugin-log`).

## Architecture

### Rust Backend (`src-tauri/src/`)

| File                  | Role                                                            |
|-----------------------|-----------------------------------------------------------------|
| `lib.rs`              | Tauri app builder, command registration, state management       |
| `device_manager.rs`   | `adb devices -l` parsing, `list_devices()` async function       |
| `session_manager.rs`  | Session lifecycle, scrcpy connection ownership, video/control stream management |
| `scrcpy_server.rs`    | Direct scrcpy-server protocol: push jar, start server, TCP handshake, video packet reading, touch/key event sending |
| `apk_manager.rs`      | APK cloning: pull, decompile (apktool), rename package, tint icon, recompile, sign, install |
| `config_manager.rs`   | Persistent config (`~/.dokky/config.json`): video settings, shortcuts, navigation |
| `license_manager.rs`  | Pro license: activate/validate/deactivate via LemonSqueezy API  |
| `paths.rs`            | `BundledPaths` — cross-platform resolution of bundled tools (adb, java, apktool, etc.) |
| `error.rs`            | `DokkyError` enum — serializable error type for Tauri IPC       |

### scrcpy-server Protocol (`scrcpy_server.rs`)

The app communicates directly with `scrcpy-server` on the Android device (no scrcpy CLI):

1. Push `scrcpy-server.jar` to device via `adb push`
2. Set up ADB reverse tunnel: device connects TO the PC
3. Start server: `adb shell CLASSPATH=... app_process / com.genymobile.scrcpy.Server 3.3.4 ...`
4. Accept video socket → 64-byte device name → 12-byte codec metadata → H.264 packets
5. Accept control socket → send touch/key/text events (binary protocol, big-endian)

**Video packet format**: 12-byte header (PTS+flags as u64 BE + size as u32 BE) then H.264 Annex B data. Bit 63 of PTS = config flag, bit 62 = keyframe flag.

**Server options**: `stay_awake=true`, `audio=false`, `video_codec=h264`, `video_codec_options=i-frame-interval=N`

### Session State Management

Sessions are stored in `SessionStore` (a `Mutex<HashMap>`) as Tauri managed state. Each session holds:
- `SessionInfo` (serializable metadata sent to frontend, includes `display_name`)
- `SessionConnection` (video Arc<AsyncMutex<TcpStream>>, control Arc<AsyncMutex<TcpStream>>, server Child process)

**Critical pattern**: The std::sync::Mutex must never be held across `.await` points. Lock, extract/clone, drop, then await. The control stream uses `tokio::sync::Mutex` since it's held across async writes.

### Vue Frontend (`src/`)

| File/Dir                        | Role                                                          |
|---------------------------------|---------------------------------------------------------------|
| `composables/useDevices.ts`     | Polls `get_devices` (3s normal, 10s when sessions active)     |
| `composables/useSessions.ts`    | Session CRUD + tab switching, shared reactive state            |
| `composables/useShortcuts.ts`   | Centralized keyboard: game shortcuts, text input detection, navigation |
| `composables/useVideoPreset.ts` | Video presets (Ultra/High/Medium/Low/Custom) + custom settings |
| `composables/useClones.ts`      | Auto-loads clone info per device                               |
| `composables/useLicense.ts`     | Pro license state (isPro), activate/deactivate                 |
| `components/VideoPlayer.vue`    | WebCodecs H.264 decoder → canvas rendering + mouse→touch      |
| `components/TabBar.vue`         | Chrome-style tab bar with session display names                |
| `components/ActionSidebar.vue`  | Right sidebar: shortcuts, devices, performance, account, settings icons |
| `components/SettingsPanel.vue`  | Slide-in panels for each sidebar section                       |
| `components/ShortcutOverlay.vue`| Visual shortcut zone editor (drag to draw, click to edit)      |
| `components/NewSessionDialog.vue` | Modal: pick device + clone to launch                         |
| `types/index.ts`                | TypeScript types mirroring Rust structs                        |

### Video Pipeline

1. Rust reads H.264 packets from scrcpy-server TCP socket
2. Packets sent as raw binary via `tauri::ipc::Response` (no base64)
3. Frontend parses 13-byte header + H.264 data, configures `VideoDecoder` (WebCodecs)
4. Decoded `VideoFrame`s drawn to `<canvas>` via `requestAnimationFrame` (frame dropping for inactive tabs)
5. Mouse events on canvas translated to scrcpy touch coordinates via `send_touch`

### Keyboard System (`useShortcuts.ts`)

Centralized in a singleton window listener (capture phase). Two modes auto-detected via ADB polling (`mServedInputConnection`):

- **Shortcut mode** (no text field focused): mapped keys trigger touch taps (with long press support), unmapped keys ignored
- **Typing mode** (text field focused): keys forwarded via `INJECT_TEXT` / `INJECT_KEYCODE`

**Known limitation**: Text input on Android virtual displays is buffered — text appears only when the input field loses/regains focus. This is an Android OS limitation, not a Dokky bug.

### IPC (Tauri Commands)

- `get_devices()` → `Vec<Device>`
- `get_packages(device_serial, filter)` → `Vec<String>`
- `get_dofus_clones(device_serial)` → `Vec<CloneInfo>`
- `clone_dofus(device_serial, clone_suffix, display_name, icon_color?)` → `String`
- `remove_dofus_clone(device_serial, package)` → `()`
- `create_session(device_serial, app_package, display_name?, display_spec?, video_bit_rate?, max_fps?, iframe_interval?)` → `SessionInfo`
- `list_sessions()` → `Vec<SessionInfo>`
- `stop_session(session_id)` → `()`
- `read_video_packet(session_id)` → `Response` (raw binary)
- `send_touch(session_id, action, x, y, width, height)` → `()`
- `send_key(session_id, action, keycode, repeat, metastate)` → `()`
- `send_text(session_id, text)` → `()`
- `get_config()` / `set_config(config)` → config persistence
- `check_license()` / `activate_license(key)` / `deactivate_license()` → Pro license
- `is_keyboard_visible(device_serial)` → `bool`
- `set_device_animations(device_serial, enabled)` → `()`
- `set_device_screen_dim(device_serial, dim)` → `()`

### Bundled Dependencies (`src-tauri/resources/`)

All external tools are bundled in the app for zero-install experience:

| Resource | Purpose |
|----------|---------|
| `bin/adb` | Android Debug Bridge |
| `scrcpy-server.jar` | scrcpy server (runs on device) |
| `apktool.jar` | APK decompile/recompile for cloning |
| `apksigner.jar` | APK signing |
| `bin/zipalign` | APK alignment |
| `jre/` | Minimal JRE (46MB via jlink) for apktool/apksigner/keytool |

Path resolution in `paths.rs`: checks bundled paths first, falls back to system PATH in dev mode.

### Pro License System

- **Free**: multi-instance on 1 device, APK cloning, video presets
- **Pro**: multi-device + keyboard shortcuts
- Validated via LemonSqueezy API at each app launch (online required)
- 1 activation per license key (machine-locked via instance_id)
- Stored in `~/.dokky/license.json`

## Key Concept

One tab = one session = one virtual display on a device. Video is rendered directly in the app via WebCodecs, not in a separate scrcpy window.

## Known Constraints

- `--new-display` is not stable on all Android devices/ROMs
- Text input on virtual displays is buffered by Android (appears on refocus)
- Some games may refuse virtual displays
- Performance limited by device CPU/GPU (encoding), USB bandwidth, and per-device session count
- scrcpy-server protocol is internal and may change between versions (currently pinned to 3.3.4)
