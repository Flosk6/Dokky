import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { SessionInfo } from "../types";

const sessions = ref<SessionInfo[]>([]);
const activeSessionId = ref<string | null>(null);
const creating = ref(false);

export function useSessions() {
  async function fetchSessions() {
    sessions.value = await invoke<SessionInfo[]>("list_sessions");
  }

  async function createSession(
    deviceSerial: string,
    appPackage: string,
    displayName?: string,
    displaySpec?: string,
    videoBitRate?: number,
    maxFps?: number,
    iframeInterval?: number,
    screenOff?: boolean,
  ) {
    if (creating.value) return;
    creating.value = true;
    try {
      const session = await invoke<SessionInfo>("create_session", {
        deviceSerial,
        appPackage,
        displayName,
        displaySpec,
        videoBitRate,
        maxFps,
        iframeInterval,
        screenOff,
      });
      sessions.value.push(session);
      activeSessionId.value = session.id;
      return session;
    } finally {
      creating.value = false;
    }
  }

  async function stopSession(sessionId: string) {
    await invoke("stop_session", { sessionId });
    const idx = sessions.value.findIndex((s) => s.id === sessionId);
    if (idx !== -1) {
      sessions.value.splice(idx, 1);
    }
    // Switch to another tab if we closed the active one
    if (activeSessionId.value === sessionId) {
      activeSessionId.value = sessions.value[0]?.id ?? null;
    }
  }

  function switchTo(sessionId: string) {
    activeSessionId.value = sessionId;
  }

  function switchByIndex(index: number) {
    if (index >= 0 && index < sessions.value.length) {
      activeSessionId.value = sessions.value[index].id;
    }
  }

  function switchNext() {
    if (sessions.value.length === 0) return;
    const idx = sessions.value.findIndex(
      (s) => s.id === activeSessionId.value
    );
    const next = (idx + 1) % sessions.value.length;
    activeSessionId.value = sessions.value[next].id;
  }

  function switchPrev() {
    if (sessions.value.length === 0) return;
    const idx = sessions.value.findIndex(
      (s) => s.id === activeSessionId.value
    );
    const prev = (idx - 1 + sessions.value.length) % sessions.value.length;
    activeSessionId.value = sessions.value[prev].id;
  }

  return {
    sessions,
    activeSessionId,
    creating,
    fetchSessions,
    createSession,
    stopSession,
    switchTo,
    switchByIndex,
    switchNext,
    switchPrev,
  };
}
