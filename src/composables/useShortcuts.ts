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

// Shared config state
const config = ref<AppConfig>({
  navigation: {
    new_session: "Ctrl+T",
    close_session: "Ctrl+W",
    next_tab: "Ctrl+Tab",
    prev_tab: "Ctrl+Shift+Tab",
  },
  game_actions: [],
});

const captureMode = ref(false);
const captureCallback = ref<((x: number, y: number) => void) | null>(null);
let sendTouchToActive: ((key: string) => void) | null = null;

export function useShortcuts(actions?: NavigationActions) {
  // Load config from backend
  async function loadConfig() {
    try {
      config.value = await invoke<AppConfig>("get_config");
    } catch {
      // Use defaults
    }
  }

  async function saveConfig() {
    try {
      await invoke("set_config", { config: config.value });
    } catch (e) {
      console.error("Failed to save config:", e);
    }
  }

  function addGameAction(action: GameAction) {
    // Remove existing action with same key
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

  // Register the touch sender (called by VideoPlayer)
  function registerTouchSender(fn: (key: string) => void) {
    sendTouchToActive = fn;
  }

  function handleKeydown(e: KeyboardEvent) {
    // Don't capture shortcuts when typing in inputs
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement || e.target instanceof HTMLSelectElement) {
      return;
    }

    const ctrl = e.ctrlKey || e.metaKey;

    // Navigation shortcuts
    if (actions) {
      if (ctrl && e.key === "t") {
        e.preventDefault();
        actions.newSession();
        return;
      }
      if (ctrl && e.key === "w") {
        e.preventDefault();
        actions.closeActive();
        return;
      }
      if (ctrl && e.key >= "1" && e.key <= "9") {
        e.preventDefault();
        actions.switchByIndex(parseInt(e.key) - 1);
        return;
      }
      if (ctrl && e.key === "Tab") {
        e.preventDefault();
        if (e.shiftKey) {
          actions.switchPrev();
        } else {
          actions.switchNext();
        }
        return;
      }
    }

    // Game action shortcuts (only when no modifier keys)
    if (!ctrl && !e.altKey && sendTouchToActive) {
      const action = config.value.game_actions.find(
        (a) => a.key.toLowerCase() === e.key.toLowerCase()
      );
      if (action) {
        e.preventDefault();
        sendTouchToActive(action.key);
      }
    }
  }

  onMounted(() => {
    loadConfig();
    window.addEventListener("keydown", handleKeydown);
  });

  onUnmounted(() => {
    window.removeEventListener("keydown", handleKeydown);
  });

  return {
    config,
    captureMode,
    captureCallback,
    loadConfig,
    saveConfig,
    addGameAction,
    removeGameAction,
    startCapture,
    endCapture,
    registerTouchSender,
  };
}
