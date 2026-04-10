<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useDevices } from "./composables/useDevices";
import { useSessions } from "./composables/useSessions";
import { useShortcuts } from "./composables/useShortcuts";
import { useToast } from "./composables/useToast";
import { useClones } from "./composables/useClones";
import { useVideoPreset } from "./composables/useVideoPreset";
import { useLicense } from "./composables/useLicense";
import TabBar from "./components/TabBar.vue";
import ActionSidebar from "./components/ActionSidebar.vue";
import SettingsPanel from "./components/SettingsPanel.vue";
import NewSessionDialog from "./components/NewSessionDialog.vue";
import VideoPlayer from "./components/VideoPlayer.vue";
import Toast from "./components/Toast.vue";
import ProModal from "./components/ProModal.vue";

const { devices, error: devicesError, setSlowPolling } = useDevices();
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

// Slow down device polling when sessions are active
watch(
  () => sessions.value.length,
  (count) => setSlowPolling(count > 0),
);
const { error: toastError, success: toastSuccess } = useToast();
const { iconMap } = useClones(devices);
const { effectiveSettings, displaySpec } = useVideoPreset();
const { isPro, checkLicense } = useLicense();

const showNewSessionDialog = ref(false);
const activePanel = ref<string | null>(null);
const showShortcuts = ref(false);
const proModalFeature = ref<string | null>(null);

function handleShortcutToggle() {
  if (isPro.value) {
    showShortcuts.value = !showShortcuts.value;
  } else {
    proModalFeature.value = "Raccourcis clavier";
  }
}

function togglePanel(panel: string) {
  activePanel.value = activePanel.value === panel ? null : panel;
}

// Check license on startup
onMounted(() => checkLicense());

async function handleCreateSession(
  deviceSerial: string,
  appPackage: string,
  displayName: string,
) {
  // Multi-device gate
  if (!isPro.value && sessions.value.length > 0) {
    const existingDevice = sessions.value[0].device_serial;
    if (deviceSerial !== existingDevice) {
      proModalFeature.value = "Multi-device";
      return;
    }
  }

  const s = effectiveSettings.value;
  try {
    if (s.disable_animations) {
      await invoke("set_device_animations", { deviceSerial, enabled: false });
    }
    if (s.screen_off) {
      await invoke("set_device_screen_dim", { deviceSerial, dim: true });
    }
    await createSession(
      deviceSerial,
      appPackage,
      displayName,
      displaySpec.value,
      s.bitrate,
      s.fps,
      s.iframe_interval,
    );
    showNewSessionDialog.value = false;
    showShortcuts.value = false;
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

const { setActiveSession } = useShortcuts({
  newSession: () => (showNewSessionDialog.value = true),
  closeActive: handleCloseActive,
  switchByIndex,
  switchNext,
  switchPrev,
});

// Keep useShortcuts in sync with the active session for keyboard handling
watch(activeSessionId, (sid) => {
  const session = sessions.value.find((s) => s.id === sid);
  setActiveSession(sid, session?.device_serial ?? null);
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
    <main class="game" style="position: relative">
      <div v-if="sessions.length === 0" class="empty-state">
        <img src="./assets/dokky-logo.png" alt="Dokky" class="empty-logo" />
        <h1>Dokky</h1>
        <p class="empty-sub">Dofus Touch Mirroring</p>
        <div class="empty-status">
          <p v-if="devicesError" class="error">{{ devicesError }}</p>
          <p v-else-if="devices.length === 0">Branchez un téléphone en USB</p>
          <p v-else>
            {{ devices.length }} device{{
              devices.length !== 1 ? "s" : ""
            }}
            connecté{{ devices.length !== 1 ? "s" : "" }}
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
          :active="session.id === activeSessionId"
          :shortcut-mode="showShortcuts"
          @close-shortcuts="showShortcuts = false"
        />
      </div>
    </main>

    <!-- Sidebar -->
    <ActionSidebar
      :active-panel="activePanel"
      @toggle-shortcuts="handleShortcutToggle"
      @open-panel="togglePanel"
    />

    <!-- Panels -->
    <SettingsPanel
      :panel="activePanel"
      :devices="devices"
      @close="activePanel = null"
    />

    <NewSessionDialog
      :visible="showNewSessionDialog"
      :devices="devices.filter((d) => d.status === 'device')"
      @close="showNewSessionDialog = false"
      @create="handleCreateSession"
    />

    <ProModal
      :visible="!!proModalFeature"
      :feature="proModalFeature ?? ''"
      @close="proModalFeature = null"
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
  width: 96px;
  height: 96px;
  margin: 0 auto 16px;
  object-fit: contain;
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

.btn-primary:hover {
  background: var(--accent-hover);
}
.btn-primary:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.empty-hint {
  display: block;
  margin-top: 8px;
  color: var(--text-muted);
  font-size: 0.75rem;
}

.error {
  color: var(--danger);
}
</style>
