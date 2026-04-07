<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Device } from "../types";
import { useToast } from "../composables/useToast";
import { useClones } from "../composables/useClones";
import { useVideoPreset } from "../composables/useVideoPreset";
import { useShortcuts } from "../composables/useShortcuts";
import ConfirmModal from "./ConfirmModal.vue";
import Loader from "./Loader.vue";

const props = defineProps<{
  visible: boolean;
  devices: Device[];
}>();

defineEmits<{
  close: [];
}>();

const { success: toastSuccess, error: toastError } = useToast();
const devicesRef = computed(() => props.devices);
const { getClones, isLoading, refreshDevice } = useClones(devicesRef);

const activeTab = ref<"devices" | "settings">("devices");
const selectedDeviceSerial = ref("");

// Clone form
const cloning = ref(false);
const showCloneForm = ref(false);
const cloneName = ref("");
const cloneColor = ref("#5865F2");
const confirmDeletePkg = ref<string | null>(null);

const COLOR_PRESETS = ["#5865F2", "#ED4245", "#57F287", "#FEE75C", "#EB459E", "#F47B67"];

const { selectedPresetName: selectedPreset, select: selectPreset, VIDEO_PRESETS } = useVideoPreset();
const { config: shortcutConfig, saveConfig } = useShortcuts();

// Editable nav shortcut
const editingNav = ref<string | null>(null);
const navLabels: Record<string, string> = {
  new_session: "Nouvelle instance",
  close_session: "Fermer instance",
  next_tab: "Instance suivante",
  prev_tab: "Instance précédente",
};

function startEditNav(key: string) {
  editingNav.value = key;
}

function captureNavKey(key: string, e: KeyboardEvent) {
  if (e.key === "Escape") {
    editingNav.value = null;
    return;
  }
  e.preventDefault();

  const parts: string[] = [];
  if (e.ctrlKey || e.metaKey) parts.push("Ctrl");
  if (e.shiftKey) parts.push("Shift");
  if (e.altKey) parts.push("Alt");

  const k = e.key;
  if (!["Control", "Shift", "Alt", "Meta"].includes(k)) {
    parts.push(k === " " ? "Space" : k.length === 1 ? k.toUpperCase() : k);
    const combo = parts.join("+");
    (shortcutConfig.value.navigation as Record<string, string>)[key] = combo;
    saveConfig();
    editingNav.value = null;
    toastSuccess(`Raccourci "${navLabels[key]}" → ${combo}`);
  }
}

const connectedDevices = computed(() =>
  props.devices.filter((d) => d.status === "device")
);

const selectedDevice = computed(() =>
  connectedDevices.value.find((d) => d.serial === selectedDeviceSerial.value)
);

// Auto-select first device
watch(connectedDevices, (devs) => {
  if (!selectedDeviceSerial.value && devs.length > 0) {
    selectedDeviceSerial.value = devs[0].serial;
  }
}, { immediate: true });

function selectDevice(serial: string) {
  selectedDeviceSerial.value = serial;
}

const clones = computed(() => getClones(selectedDeviceSerial.value));
const loadingClones = computed(() => isLoading(selectedDeviceSerial.value));

async function handleClone() {
  if (!selectedDeviceSerial.value || cloning.value) return;

  const existingNumbers = clones.value.map((c) => {
    const match = c.package.match(/dofustouch(\d+)$/);
    return match ? parseInt(match[1]) : 1;
  });
  const nextNumber = Math.max(1, ...existingNumbers) + 1;
  const name = cloneName.value.trim() || `Dofus Touch ${nextNumber}`;

  cloning.value = true;
  try {
    await invoke<string>("clone_dofus", {
      deviceSerial: selectedDeviceSerial.value,
      cloneSuffix: String(nextNumber),
      displayName: name,
      iconColor: cloneColor.value,
    });
    toastSuccess(`${name} cloné avec succès`);
    showCloneForm.value = false;
    cloneName.value = "";
    refreshDevice(selectedDeviceSerial.value);
  } catch (e: unknown) {
    toastError(typeof e === "object" && e !== null ? JSON.stringify(e) : String(e));
  } finally {
    cloning.value = false;
  }
}

function requestRemove(pkg: string) {
  if (pkg === "com.ankama.dofustouch") return;
  confirmDeletePkg.value = pkg;
}

async function confirmRemove() {
  const pkg = confirmDeletePkg.value;
  confirmDeletePkg.value = null;
  if (!pkg || !selectedDeviceSerial.value) return;

  try {
    await invoke("remove_dofus_clone", {
      deviceSerial: selectedDeviceSerial.value,
      package: pkg,
    });
    toastSuccess("Clone supprimé");
    refreshDevice(selectedDeviceSerial.value);
  } catch (e) {
    toastError(String(e));
  }
}

function cloneDisplayName(pkg: string): string {
  return clones.value.find((c) => c.package === pkg)?.display_name ?? pkg;
}
</script>

<template>
  <Transition name="panel">
    <div v-if="visible" class="panel-overlay" @click.self="$emit('close')">
      <div class="panel">
        <!-- Header with tabs -->
        <div class="panel-header">
          <div class="panel-tabs">
            <button
              class="panel-tab"
              :class="{ active: activeTab === 'devices' }"
              @click="activeTab = 'devices'"
            >
              Devices
            </button>
            <button
              class="panel-tab"
              :class="{ active: activeTab === 'settings' }"
              @click="activeTab = 'settings'"
            >
              Général
            </button>
          </div>
          <button class="panel-close" @click="$emit('close')">&times;</button>
        </div>

        <!-- DEVICES TAB -->
        <div v-if="activeTab === 'devices'" class="panel-body">
          <!-- Device selector -->
          <div class="device-list">
            <div
              v-for="dev in connectedDevices"
              :key="dev.serial"
              class="device-card"
              :class="{ selected: dev.serial === selectedDeviceSerial }"
              @click="selectDevice(dev.serial)"
            >
              <div class="device-dot" />
              <div class="device-info">
                <span class="device-name">{{ dev.model ?? "Device" }}</span>
                <span class="device-serial">{{ dev.serial }}</span>
              </div>
            </div>
            <div v-if="connectedDevices.length === 0" class="empty">
              Aucun device connecté
            </div>
          </div>

          <!-- Selected device content -->
          <div v-if="selectedDevice" class="device-section">

            <!-- CLONES APK (primary) -->
            <div class="section-title">Comptes Dofus Touch</div>

            <Loader v-if="loadingClones" label="Chargement..." />
            <div v-else>
              <div v-for="clone in clones" :key="clone.package" class="clone-row">
                <img v-if="clone.icon" :src="`data:image/png;base64,${clone.icon}`" class="clone-icon" />
                <div class="clone-info">
                  <span class="clone-name">{{ clone.display_name }}</span>
                  <span class="clone-pkg">{{ clone.package }}</span>
                </div>
                <button
                  v-if="clone.package !== 'com.ankama.dofustouch'"
                  class="btn-x"
                  @click="requestRemove(clone.package)"
                >×</button>
              </div>

              <div v-if="showCloneForm" class="clone-form">
                <input
                  v-model="cloneName"
                  type="text"
                  placeholder="Nom (ex: Sacrieur PvP)"
                  class="input"
                  @keyup.enter="handleClone"
                />
                <div class="color-row">
                  <button
                    v-for="c in COLOR_PRESETS" :key="c"
                    class="color-dot"
                    :class="{ active: cloneColor === c }"
                    :style="{ background: c }"
                    @click="cloneColor = c"
                  />
                  <input v-model="cloneColor" type="color" class="color-pick" />
                </div>
                <div class="form-actions">
                  <button class="btn-sm" @click="showCloneForm = false">Annuler</button>
                  <button class="btn-sm accent" :disabled="cloning" @click="handleClone">
                    {{ cloning ? "Clonage..." : "Cloner" }}
                  </button>
                </div>
              </div>
              <button v-else class="btn-dashed" @click="showCloneForm = true">+ Nouveau clone APK</button>
            </div>
          </div>

          <div v-else-if="connectedDevices.length > 0" class="empty">
            Sélectionnez un device
          </div>
        </div>

        <!-- SETTINGS TAB -->
        <div v-if="activeTab === 'settings'" class="panel-body">
          <div class="section-title">Qualité vidéo</div>
          <div class="preset-grid">
            <button
              v-for="preset in VIDEO_PRESETS"
              :key="preset.name"
              class="preset-card"
              :class="{ active: selectedPreset === preset.name }"
              @click="selectPreset(preset.name)"
            >
              <span class="preset-name">{{ preset.label }}</span>
              <span class="preset-detail">{{ preset.resolution }} · {{ preset.dpi }} dpi</span>
              <span class="preset-detail">{{ preset.fps }} FPS · {{ preset.bitrate / 1000000 }} Mbps</span>
            </button>
          </div>

          <div class="divider" />

          <div class="section-title">Navigation</div>
          <div
            v-for="(navKey, navId) in shortcutConfig.navigation"
            :key="navId"
            class="action-row editable"
            @click="startEditNav(navId as string)"
          >
            <kbd v-if="editingNav !== navId" class="action-key">{{ navKey }}</kbd>
            <input
              v-else
              class="input key-capture"
              placeholder="Appuyez..."
              readonly
              autofocus
              @keydown="captureNavKey(navId as string, $event)"
              @blur="editingNav = null"
            />
            <span class="action-label">{{ navLabels[navId as string] ?? navId }}</span>
            <span class="action-edit-hint">clic pour modifier</span>
          </div>

          <div class="divider" />

          <div class="section-title">À propos</div>
          <div class="setting-row">
            <span class="setting-label">Dokky</span>
            <span class="setting-value">v0.1.0</span>
          </div>
          <div class="setting-row">
            <span class="setting-label">scrcpy</span>
            <span class="setting-value">3.3.4</span>
          </div>
        </div>
      </div>
    </div>
  </Transition>

  <ConfirmModal
    :visible="!!confirmDeletePkg"
    title="Supprimer le clone"
    :message="`Supprimer ${confirmDeletePkg ? cloneDisplayName(confirmDeletePkg) : ''} de ${selectedDevice?.model ?? 'ce device'} ?`"
    confirm-label="Supprimer"
    danger
    @confirm="confirmRemove"
    @cancel="confirmDeletePkg = null"
  />
</template>

<style scoped>
/* Panel overlay + slide */
.panel-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.4);
  z-index: 100;
  display: flex;
  justify-content: flex-end;
}

.panel {
  width: 340px;
  height: 100%;
  background: var(--bg-secondary);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
}

.panel-enter-active { transition: all 0.25s ease-out; }
.panel-leave-active { transition: all 0.2s ease-in; }
.panel-enter-from .panel,
.panel-leave-to .panel { transform: translateX(100%); }
.panel-enter-from,
.panel-leave-to { background: rgba(0, 0, 0, 0); }

/* Header */
.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 8px 0 0;
  border-bottom: 1px solid var(--border);
  height: var(--topbar-height);
  flex-shrink: 0;
}

.panel-tabs {
  display: flex;
  height: 100%;
}

.panel-tab {
  padding: 0 16px;
  height: 100%;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--text-secondary);
  font-size: 0.82rem;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.15s;
}

.panel-tab:hover { color: var(--text-primary); }
.panel-tab.active {
  color: #fff;
  border-bottom-color: var(--accent);
}

.panel-close {
  background: none;
  border: none;
  color: var(--text-muted);
  font-size: 1.3rem;
  cursor: pointer;
  padding: 4px 8px;
  border-radius: var(--radius-sm);
}

.panel-close:hover { color: #fff; background: var(--bg-hover); }

/* Body */
.panel-body {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
}

/* Device list */
.device-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 16px;
}

.device-card {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.15s;
}

.device-card:hover { border-color: var(--text-muted); }
.device-card.selected { border-color: var(--accent); background: var(--bg-hover); }

.device-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--success);
  flex-shrink: 0;
}

.device-info { flex: 1; min-width: 0; }

.device-name {
  display: block;
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-primary);
}

.device-serial {
  display: block;
  font-size: 0.7rem;
  color: var(--text-muted);
  font-family: monospace;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Section */
.device-section { margin-top: 4px; }

.section-title {
  font-size: 0.75rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  margin-bottom: 10px;
}

/* Clone rows */
.clone-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
}

.clone-row:last-of-type { border-bottom: none; }

.clone-icon { width: 28px; height: 28px; border-radius: var(--radius-sm); flex-shrink: 0; }

.clone-info { flex: 1; min-width: 0; }

.clone-name {
  display: block;
  font-size: 0.82rem;
  font-weight: 500;
  color: var(--text-primary);
}

.clone-pkg {
  display: block;
  font-size: 0.65rem;
  color: var(--text-muted);
  font-family: monospace;
  overflow: hidden;
  text-overflow: ellipsis;
}

.btn-x {
  background: none;
  border: 1px solid transparent;
  color: var(--text-muted);
  width: 24px;
  height: 24px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1rem;
  flex-shrink: 0;
}

.btn-x:hover { border-color: var(--danger); color: var(--danger); }

/* Clone form */
.clone-form { margin-top: 10px; }

.input {
  width: 100%;
  padding: 8px 10px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-size: 0.82rem;
  margin-bottom: 8px;
}

.input:focus { outline: none; border-color: var(--accent); }

.color-row { display: flex; gap: 6px; align-items: center; margin-bottom: 10px; }

.color-dot {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  transition: all 0.15s;
}

.color-dot:hover { transform: scale(1.15); }
.color-dot.active { border-color: #fff; }

.color-pick {
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 50%;
  cursor: pointer;
  padding: 0;
  background: none;
}

.form-actions { display: flex; gap: 6px; justify-content: flex-end; }

.btn-sm {
  padding: 6px 12px;
  border: none;
  border-radius: var(--radius-md);
  font-size: 0.8rem;
  cursor: pointer;
  background: var(--border);
  color: var(--text-secondary);
}

.btn-sm:hover { background: var(--bg-hover); color: var(--text-primary); }
.btn-sm.accent { background: var(--accent); color: #fff; }
.btn-sm.accent:hover { background: var(--accent-hover); }
.btn-sm:disabled { opacity: 0.4; cursor: not-allowed; }

.btn-dashed {
  width: 100%;
  padding: 8px;
  background: none;
  border: 1px dashed var(--border);
  border-radius: var(--radius-md);
  color: var(--text-muted);
  font-size: 0.82rem;
  cursor: pointer;
  margin-top: 10px;
}

.btn-dashed:hover { border-color: var(--accent); color: var(--text-secondary); }

/* Settings tab */
.setting-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
}

.setting-label { font-size: 0.82rem; color: var(--text-secondary); }
.setting-value { font-size: 0.8rem; color: var(--text-primary); font-family: monospace; }

/* Navigation shortcuts */
.action-row {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}

.action-row:last-of-type { border-bottom: none; }

.action-key {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 28px;
  height: 24px;
  padding: 0 6px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  font-family: monospace;
  font-size: 0.75rem;
  color: var(--text-primary);
  flex-shrink: 0;
}

.action-label {
  flex: 1;
  font-size: 0.8rem;
  color: var(--text-secondary);
}

.action-row.editable {
  cursor: pointer;
  border-radius: var(--radius-sm);
  padding: 6px 4px;
  margin: 0 -4px;
}

.action-row.editable:hover { background: var(--bg-hover); }

.action-edit-hint {
  font-size: 0.6rem;
  color: transparent;
  transition: color 0.15s;
}

.action-row.editable:hover .action-edit-hint {
  color: var(--text-muted);
}

.key-capture {
  width: 80px;
  min-width: 28px;
  height: 24px;
  padding: 0 6px;
  text-align: center;
  font-family: monospace;
  font-size: 0.75rem;
  font-weight: 700;
  flex-shrink: 0;
  animation: pulse-border 1s ease-in-out infinite;
}

@keyframes pulse-border {
  0%, 100% { border-color: var(--accent); }
  50% { border-color: var(--accent-hover); }
}

/* Video presets */
.preset-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 6px;
}

.preset-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 10px 8px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all 0.15s;
}

.preset-card:hover { border-color: var(--text-muted); }
.preset-card.active { border-color: var(--accent); background: var(--bg-hover); }

.preset-name {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-primary);
}

.preset-card.active .preset-name { color: #fff; }

.preset-detail {
  font-size: 0.65rem;
  color: var(--text-muted);
  font-family: monospace;
}

.divider {
  height: 1px;
  background: var(--border);
  margin: 16px 0;
}

.empty {
  color: var(--text-muted);
  font-size: 0.82rem;
  padding: 8px 0;
}
</style>
