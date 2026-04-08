import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AppConfig, GameAction } from "../types";

interface NavigationActions {
  newSession: () => void;
  closeActive: () => void;
  switchByIndex: (index: number) => void;
  switchNext: () => void;
  switchPrev: () => void;
}

const config = ref<AppConfig>({
  navigation: {
    new_session: "Ctrl+T",
    close_session: "Ctrl+W",
    next_tab: "Ctrl+Tab",
    prev_tab: "Ctrl+Shift+Tab",
  },
  game_actions: [],
  video_preset: "high",
  video_settings: {
    width: 1920, height: 1080, dpi: 240, fps: 60, bitrate: 8_000_000,
    baseline_profile: true, iframe_interval: 2, no_vd_system_decorations: true, disable_animations: false, screen_off: false,
  },
});

const captureMode = ref(false);
const captureCallback = ref<((x: number, y: number) => void) | null>(null);

// Active session
const activeSessionId = ref<string | null>(null);
const activeDeviceSerial = ref<string | null>(null);
const sessionDimensions: Record<string, { width: number; height: number }> = {};

// Typing mode: auto-detected by polling ADB for text field focus state.
// ADB dumpsys takes ~600ms so we must avoid stacking requests.
const typingMode = ref(false);
let typingPollActive = false;
let typingPollRunning = false;

async function pollTypingModeLoop() {
  if (typingPollRunning) return;
  typingPollRunning = true;
  while (typingPollActive) {
    const serial = activeDeviceSerial.value;
    if (serial) {
      try {
        typingMode.value = await invoke<boolean>("is_keyboard_visible", { deviceSerial: serial });
      } catch {
        typingMode.value = false;
      }
    } else {
      typingMode.value = false;
    }
    // Wait 100ms between checks (the ADB call itself takes ~600ms,
    // so effective interval is ~700ms, no stacking possible)
    await new Promise((r) => setTimeout(r, 100));
  }
  typingPollRunning = false;
}

function startTypingPoll() {
  typingPollActive = true;
  pollTypingModeLoop();
}

function stopTypingPoll() {
  typingPollActive = false;
  typingMode.value = false;
}

// Singleton listeners
let listenersRegistered = false;
let navActions: NavigationActions | null = null;

// ---- Game action touch ----
// Track active presses: key → {x, y, sessionId, dims} so we can release on keyup
const activePresses: Map<string, { sid: string; x: number; y: number; w: number; h: number }> = new Map();

function sendTouchDown(action: GameAction) {
  const sid = activeSessionId.value;
  if (!sid) return;
  const dims = sessionDimensions[sid];
  if (!dims) return;

  const normX = action.x + (Math.random() - 0.5) * (action.w ?? 0.02);
  const normY = action.y + (Math.random() - 0.5) * (action.h ?? 0.02);
  const x = Math.round(Math.max(0, Math.min(1, normX)) * dims.width);
  const y = Math.round(Math.max(0, Math.min(1, normY)) * dims.height);

  activePresses.set(action.key.toLowerCase(), { sid, x, y, w: dims.width, h: dims.height });
  invoke("send_touch", { sessionId: sid, action: 0, x, y, width: dims.width, height: dims.height }).catch(() => {});
}

function sendTouchUp(key: string) {
  const press = activePresses.get(key.toLowerCase());
  if (!press) return;
  activePresses.delete(key.toLowerCase());
  invoke("send_touch", { sessionId: press.sid, action: 1, x: press.x, y: press.y, width: press.w, height: press.h }).catch(() => {});
}

// ---- Key handlers ----

function handleKeydown(e: KeyboardEvent) {
  // Skip app UI inputs (settings, etc.)
  if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) {
    return;
  }

  const ctrl = e.ctrlKey || e.metaKey;

  // Navigation shortcuts (Ctrl+...) — always work
  if (navActions && ctrl) {
    if (e.key === "t") { e.preventDefault(); navActions.newSession(); return; }
    if (e.key === "w") { e.preventDefault(); navActions.closeActive(); return; }
    if (e.key >= "1" && e.key <= "9") { e.preventDefault(); navActions.switchByIndex(parseInt(e.key) - 1); return; }
    if (e.key === "Tab") {
      e.preventDefault();
      if (e.shiftKey) { navActions.switchPrev(); } else { navActions.switchNext(); }
      return;
    }
  }

  // In typing mode (text field focused on device)
  if (typingMode.value && !ctrl) {
    const sid = activeSessionId.value;
    if (!sid) return;

    // Special keys via INJECT_KEYCODE
    const SPECIAL_KEYS: Record<string, number> = {
      Enter: 66, Backspace: 67, Delete: 112, Escape: 111, Tab: 61,
      ArrowUp: 19, ArrowDown: 20, ArrowLeft: 21, ArrowRight: 22,
    };
    const keycode = SPECIAL_KEYS[e.key];
    if (keycode) {
      e.preventDefault();
      let meta = 0;
      if (e.shiftKey) meta |= 0x1;
      if (e.altKey) meta |= 0x2;
      invoke("send_key", { sessionId: sid, action: 0, keycode, repeat: 0, metastate: meta }).catch(() => {});
      setTimeout(() => {
        invoke("send_key", { sessionId: sid, action: 1, keycode, repeat: 0, metastate: meta }).catch(() => {});
      }, 20);
      return;
    }

    // Printable characters via INJECT_TEXT
    if (e.key.length === 1) {
      e.preventDefault();
      invoke("send_text", { sessionId: sid, text: e.key }).catch(() => {});
    }
    return;
  }

  // Default mode: game action shortcuts
  if (!ctrl && !e.altKey) {
    const action = config.value.game_actions.find(
      (a) => a.key.toLowerCase() === e.key.toLowerCase()
    );
    if (action) {
      e.preventDefault();
      e.stopImmediatePropagation();
      if (!e.repeat) {
        sendTouchDown(action);
      }
    }
  }
}

function handleKeyup(e: KeyboardEvent) {
  if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) {
    return;
  }
  // Release touch on key up for game action shortcuts
  const action = config.value.game_actions.find(
    (a) => a.key.toLowerCase() === e.key.toLowerCase()
  );
  if (action) {
    e.preventDefault();
    sendTouchUp(action.key);
  }
}

function registerListeners() {
  if (listenersRegistered) return;
  listenersRegistered = true;
  window.addEventListener("keydown", handleKeydown, true);
  window.addEventListener("keyup", handleKeyup, true);
}

function unregisterListeners() {
  if (!listenersRegistered) return;
  listenersRegistered = false;
  window.removeEventListener("keydown", handleKeydown, true);
  window.removeEventListener("keyup", handleKeyup, true);
}

// ---- Composable ----

export function useShortcuts(actions?: NavigationActions) {
  if (actions) {
    navActions = actions;
  }

  async function loadConfig() {
    try { config.value = await invoke<AppConfig>("get_config"); } catch {}
  }

  async function saveConfig() {
    try { await invoke("set_config", { config: config.value }); } catch (e) { console.error("Failed to save config:", e); }
  }

  function addGameAction(action: GameAction) {
    config.value.game_actions = config.value.game_actions.filter((a) => a.key !== action.key);
    config.value.game_actions.push(action);
    saveConfig();
  }

  function removeGameAction(key: string) {
    config.value.game_actions = config.value.game_actions.filter((a) => a.key !== key);
    saveConfig();
  }

  function startCapture(callback: (x: number, y: number) => void) {
    captureMode.value = true;
    captureCallback.value = callback;
  }

  function endCapture() {
    captureMode.value = false;
    captureCallback.value = null;
  }

  function setActiveSession(sessionId: string | null, deviceSerial: string | null) {
    activeSessionId.value = sessionId;
    activeDeviceSerial.value = deviceSerial;
    typingMode.value = false;
  }

  function registerSession(sessionId: string, width: number, height: number) {
    sessionDimensions[sessionId] = { width, height };
  }

  function unregisterSession(sessionId: string) {
    delete sessionDimensions[sessionId];
  }

  // Only register listeners once (from App.vue which provides actions)
  if (actions) {
    onMounted(() => {
      loadConfig();
      registerListeners();
      startTypingPoll();
    });
    onUnmounted(() => {
      unregisterListeners();
      stopTypingPoll();
    });
  }

  return {
    config,
    captureMode,
    captureCallback,
    typingMode,
    loadConfig,
    saveConfig,
    addGameAction,
    removeGameAction,
    startCapture,
    endCapture,
    setActiveSession,
    registerSession,
    unregisterSession,
  };
}
