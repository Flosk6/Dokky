<script setup lang="ts">
import { ref } from "vue";
import { useDevices } from "./composables/useDevices";
import { useSessions } from "./composables/useSessions";
import { useShortcuts } from "./composables/useShortcuts";
import { useToast } from "./composables/useToast";
import { useClones } from "./composables/useClones";
import { useVideoPreset } from "./composables/useVideoPreset";
import TabBar from "./components/TabBar.vue";
import ActionSidebar from "./components/ActionSidebar.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import NewSessionDialog from "./components/NewSessionDialog.vue";
import VideoPlayer from "./components/VideoPlayer.vue";
import Toast from "./components/Toast.vue";

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
const { error: toastError, success: toastSuccess } = useToast();
const { iconMap } = useClones(devices);
const { preset, displaySpec } = useVideoPreset();

const showNewSessionDialog = ref(false);
const showSettings = ref(false);
const showShortcuts = ref(false);

async function handleCreateSession(deviceSerial: string, appPackage: string) {
  try {
    await createSession(
      deviceSerial,
      appPackage,
      displaySpec.value,
      preset.value.bitrate,
      preset.value.fps,
    );
    showNewSessionDialog.value = false;
    toastSuccess("Session lancée");
  } catch (e) {
    toastError(`Erreur: ${typeof e === "object" ? JSON.stringify(e) : e}`);
  }
}

async function handleCloseSession(sessionId: string) {
  try {
    await stopSession(sessionId);
  } catch (e) {
    toastError(`Erreur: ${e}`);
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

// Clones are auto-loaded by useClones() when devices connect
</script>

<template>
  <div class="app-layout">
    <!-- Topbar -->
    <TabBar
      :sessions="sessions"
      :active-session-id="activeSessionId"
      :icon-map="iconMap"
      @select="switchTo"
      @close="handleCloseSession"
      @new-session="showNewSessionDialog = true"
    />

    <!-- Game area -->
    <main class="game" style="position: relative;">
      <div v-if="sessions.length === 0" class="empty-state">
        <div class="empty-logo">D</div>
        <h1>Dokki</h1>
        <p class="empty-sub">Multi-compte Dofus Touch</p>
        <div class="empty-status">
          <p v-if="devicesError" class="error">{{ devicesError }}</p>
          <p v-else-if="devices.length === 0">Branchez un téléphone en USB</p>
          <p v-else>
            {{ devices.length }} device{{ devices.length !== 1 ? "s" : "" }} connecté{{ devices.length !== 1 ? "s" : "" }}
          </p>
        </div>
        <button
          class="btn-primary"
          :disabled="devices.length === 0"
          @click="showNewSessionDialog = true"
        >
          Nouvelle instance
        </button>
        <span class="empty-hint">Ctrl+T</span>
      </div>

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
          :shortcut-mode="showShortcuts"
          @close-shortcuts="showShortcuts = false"
        />
      </div>
    </main>

    <!-- Sidebar -->
    <ActionSidebar
      @toggle-shortcuts="() => { console.log('TOGGLE SHORTCUTS', showShortcuts); showShortcuts = !showShortcuts; }"
      @toggle-settings="showSettings = !showSettings"
    />

    <!-- Overlays -->
    <SettingsPanel
      :visible="showSettings"
      :devices="devices"
      @close="showSettings = false"
    />

    <NewSessionDialog
      :visible="showNewSessionDialog"
      :devices="devices.filter((d) => d.status === 'device')"
      @close="showNewSessionDialog = false"
      @create="handleCreateSession"
    />

    <Toast />
  </div>
</template>

<style>
.app-layout {
  display: grid;
  grid-template-columns: 1fr var(--sidebar-width);
  grid-template-rows: var(--topbar-height) 1fr;
  height: 100vh;
  overflow: hidden;
}

/* Topbar spans full width */
.app-layout > :first-child {
  grid-column: 1 / -1;
}

.game {
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
  background: #000;
}

.session-view {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}

/* Empty state */
.empty-state {
  text-align: center;
  color: var(--text-secondary);
}

.empty-logo {
  width: 64px;
  height: 64px;
  margin: 0 auto 16px;
  background: var(--accent);
  border-radius: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 2rem;
  font-weight: 800;
  color: #fff;
}

.empty-state h1 {
  font-size: 1.5rem;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.empty-sub {
  font-size: 0.85rem;
  margin-bottom: 24px;
}

.empty-status {
  margin-bottom: 20px;
  font-size: 0.85rem;
}

.btn-primary {
  background: var(--accent);
  color: #fff;
  border: none;
  padding: 10px 24px;
  border-radius: var(--radius-md);
  font-size: 0.9rem;
  cursor: pointer;
  transition: background 0.15s;
}

.btn-primary:hover { background: var(--accent-hover); }
.btn-primary:disabled { opacity: 0.4; cursor: not-allowed; }

.empty-hint {
  display: block;
  margin-top: 8px;
  color: var(--text-muted);
  font-size: 0.75rem;
}

.error { color: var(--danger); }
</style>
