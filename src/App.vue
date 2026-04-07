<script setup lang="ts">
import { ref } from "vue";
import { useDevices } from "./composables/useDevices";
import { useSessions } from "./composables/useSessions";
import { useShortcuts } from "./composables/useShortcuts";
import TabBar from "./components/TabBar.vue";
import NewSessionDialog from "./components/NewSessionDialog.vue";
import StatusBar from "./components/StatusBar.vue";
import VideoPlayer from "./components/VideoPlayer.vue";

const { devices, error: devicesError } = useDevices();
const {
  sessions,
  activeSessionId,
  createSession,
  stopSession,
  switchTo,
  switchByIndex,
  switchNext,
  switchPrev,
} = useSessions();

const showNewSessionDialog = ref(false);

async function handleCreateSession(deviceSerial: string, appPackage: string) {
  try {
    console.log("[app] Creating session:", deviceSerial, appPackage);
    const session = await createSession(deviceSerial, appPackage);
    console.log("[app] Session created:", session);
    showNewSessionDialog.value = false;
  } catch (e) {
    console.error("[app] Session creation failed:", e);
    alert(`Erreur: ${e}`);
  }
}

async function handleCloseSession(sessionId: string) {
  try {
    await stopSession(sessionId);
  } catch (e) {
    alert(`Erreur: ${e}`);
  }
}

function handleCloseActive() {
  if (activeSessionId.value) {
    handleCloseSession(activeSessionId.value);
  }
}

useShortcuts({
  newSession: () => (showNewSessionDialog.value = true),
  closeActive: handleCloseActive,
  switchByIndex,
  switchNext,
  switchPrev,
});
</script>

<template>
  <div class="app-layout">
    <TabBar
      :sessions="sessions"
      :active-session-id="activeSessionId"
      @select="switchTo"
      @close="handleCloseSession"
      @new-session="showNewSessionDialog = true"
    />

    <main class="content">
      <!-- No sessions yet -->
      <div v-if="sessions.length === 0" class="empty-state">
        <h1>Dokki</h1>
        <p class="subtitle">Multi-Instance Android Desktop</p>

        <div class="devices-info">
          <p v-if="devicesError" class="error">{{ devicesError }}</p>
          <p v-else-if="devices.length === 0" class="muted">
            Aucun device détecté. Branchez un téléphone en USB.
          </p>
          <p v-else class="muted">
            {{ devices.length }} device{{ devices.length !== 1 ? "s" : "" }}
            connecté{{ devices.length !== 1 ? "s" : "" }}.
          </p>
        </div>

        <button
          class="btn-new"
          :disabled="devices.length === 0"
          @click="showNewSessionDialog = true"
        >
          + Nouvelle session
        </button>
        <p class="shortcut-hint">ou Ctrl+T</p>
      </div>

      <!-- All sessions: keep all VideoPlayers alive, show/hide -->
      <div
        v-for="session in sessions"
        :key="session.id"
        v-show="session.id === activeSessionId"
        class="session-view"
      >
        <VideoPlayer
          :session-id="session.id"
          :width="session.width"
          :height="session.height"
        />
      </div>
    </main>

    <StatusBar :sessions="sessions" :active-session-id="activeSessionId" />

    <NewSessionDialog
      :visible="showNewSessionDialog"
      :devices="devices.filter((d) => d.status === 'device')"
      @close="showNewSessionDialog = false"
      @create="handleCreateSession"
    />
  </div>
</template>

<style>
* {
  box-sizing: border-box;
}

:root {
  font-family: Inter, system-ui, -apple-system, sans-serif;
  color: #e0e0e0;
  background-color: #1a1a2e;
}

body {
  margin: 0;
  min-height: 100vh;
}

.app-layout {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

.content {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  padding: 0.5rem;
}

.empty-state {
  text-align: center;
}

.empty-state h1 {
  font-size: 2.5rem;
  margin-bottom: 0.25rem;
}

.subtitle {
  color: #888;
  margin-top: 0;
  margin-bottom: 2rem;
}

.devices-info {
  margin-bottom: 1.5rem;
}

.btn-new {
  background: #533483;
  color: #fff;
  border: none;
  padding: 0.75rem 1.5rem;
  border-radius: 8px;
  font-size: 1rem;
  cursor: pointer;
  transition: background 0.15s;
}

.btn-new:hover {
  background: #6441a5;
}

.btn-new:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.shortcut-hint {
  color: #555;
  font-size: 0.8rem;
  margin-top: 0.5rem;
}

.session-view {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

.error {
  color: #f44336;
}

.muted {
  color: #666;
}
</style>
