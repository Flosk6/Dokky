<script setup lang="ts">
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Device, CloneInfo } from "../types";
import ConfirmModal from "./ConfirmModal.vue";

const props = defineProps<{
  devices: Device[];
  visible: boolean;
}>();

defineEmits<{
  close: [];
  create: [deviceSerial: string, appPackage: string];
}>();

const selectedDevice = ref("");
const selectedPackage = ref("");
const clones = ref<CloneInfo[]>([]);
const loadingClones = ref(false);
const cloning = ref(false);
const cloneError = ref<string | null>(null);
let emitted = false;

// Clone creation form
const cloneName = ref("");
const cloneColor = ref("#5865F2"); // Discord blue by default
const showCloneForm = ref(false);
const confirmDeletePkg = ref<string | null>(null);

const COLOR_PRESETS = [
  "#5865F2", // Blue
  "#ED4245", // Red
  "#57F287", // Green
  "#FEE75C", // Yellow
  "#EB459E", // Pink
  "#F47B67", // Orange
];

function handleCreate(emit: (event: "create", deviceSerial: string, appPackage: string) => void) {
  if (emitted) return;
  if (selectedDevice.value && selectedPackage.value) {
    emitted = true;
    emit("create", selectedDevice.value, selectedPackage.value);
    setTimeout(() => { emitted = false; }, 500);
  }
}

async function loadClones(serial: string) {
  if (!serial) {
    clones.value = [];
    return;
  }
  loadingClones.value = true;
  cloneError.value = null;
  try {
    clones.value = await invoke<CloneInfo[]>("get_dofus_clones", { deviceSerial: serial });
    selectedPackage.value = clones.value[0]?.package ?? "";
  } catch (e) {
    console.error("Failed to load clones:", e);
    clones.value = [];
  } finally {
    loadingClones.value = false;
  }
}

async function handleClone() {
  if (!selectedDevice.value || cloning.value) return;

  // Generate next number
  const existingNumbers = clones.value.map((c) => {
    const match = c.package.match(/dofustouch(\d+)$/);
    return match ? parseInt(match[1]) : 1;
  });
  const nextNumber = Math.max(1, ...existingNumbers) + 1;
  const suffix = String(nextNumber);
  const name = cloneName.value.trim() || `Dofus Touch ${nextNumber}`;

  cloning.value = true;
  cloneError.value = null;
  try {
    const newPackage = await invoke<string>("clone_dofus", {
      deviceSerial: selectedDevice.value,
      cloneSuffix: suffix,
      displayName: name,
      iconColor: cloneColor.value,
    });
    clones.value.push({
      package: newPackage,
      display_name: name,
      icon: null, // will be loaded on next dialog open
    });
    selectedPackage.value = newPackage;
    showCloneForm.value = false;
    cloneName.value = "";
  } catch (e: unknown) {
    const msg = typeof e === "object" && e !== null ? JSON.stringify(e) : String(e);
    cloneError.value = msg;
    console.error("Clone failed:", e);
  } finally {
    cloning.value = false;
  }
}

function handleRemoveClone(pkg: string) {
  if (!selectedDevice.value) return;
  if (pkg === "com.ankama.dofustouch") return;
  confirmDeletePkg.value = pkg;
}

async function confirmRemove() {
  const pkg = confirmDeletePkg.value;
  confirmDeletePkg.value = null;
  if (!pkg || !selectedDevice.value) return;

  try {
    await invoke("remove_dofus_clone", {
      deviceSerial: selectedDevice.value,
      package: pkg,
    });
    clones.value = clones.value.filter((c) => c.package !== pkg);
    if (selectedPackage.value === pkg) {
      selectedPackage.value = clones.value[0]?.package ?? "";
    }
  } catch (e) {
    cloneError.value = String(e);
  }
}

function getClone(pkg: string): CloneInfo | undefined {
  return clones.value.find((c) => c.package === pkg);
}

function displayName(pkg: string): string {
  const clone = getClone(pkg);
  if (clone) return clone.display_name;
  if (pkg === "com.ankama.dofustouch") return "Dofus Touch (original)";
  return pkg;
}

watch(selectedDevice, (serial) => {
  loadClones(serial);
});

watch(
  () => props.visible,
  (visible) => {
    if (visible && !selectedDevice.value && props.devices.length > 0) {
      selectedDevice.value = props.devices[0].serial;
    }
    if (visible) {
      cloneError.value = null;
      showCloneForm.value = false;
    }
  }
);
</script>

<template>
  <div v-if="visible" class="overlay" @click.self="$emit('close')">
    <div class="dialog">
      <h3>Nouvelle session</h3>

      <label>
        Device
        <select v-model="selectedDevice">
          <option value="" disabled>Sélectionner un device</option>
          <option
            v-for="device in devices"
            :key="device.serial"
            :value="device.serial"
          >
            {{ device.model ?? device.serial }} ({{ device.serial.slice(0, 12) }})
          </option>
        </select>
      </label>

      <div class="section-label">Compte Dofus Touch</div>
      <div class="clone-list">
        <div v-if="loadingClones" class="muted">Chargement...</div>
        <div v-else-if="clones.length === 0" class="muted">
          Aucun Dofus Touch installé.
        </div>
        <div
          v-for="clone in clones"
          :key="clone.package"
          class="clone-item"
          :class="{ selected: clone.package === selectedPackage }"
          @click="selectedPackage = clone.package"
        >
          <div v-if="clone.icon" class="clone-icon-wrap">
            <img
              :src="`data:image/png;base64,${clone.icon}`"
              class="clone-icon"
              alt=""
            />
          </div>
          <div class="clone-info">
            <span class="clone-name">{{ clone.display_name }}</span>
            <span class="clone-pkg">{{ clone.package }}</span>
          </div>
          <button
            v-if="clone.package !== 'com.ankama.dofustouch'"
            class="clone-remove"
            title="Supprimer"
            @click.stop.prevent="handleRemoveClone(clone.package)"
          >
            ×
          </button>
        </div>
      </div>

      <!-- Clone creation form -->
      <div v-if="showCloneForm" class="clone-form">
        <input
          v-model="cloneName"
          type="text"
          placeholder="Nom du clone (ex: Sacrieur PvP)"
          class="clone-name-input"
          @keyup.enter="handleClone"
        />
        <div class="color-picker">
          <span class="color-label">Couleur icône</span>
          <div class="color-presets">
            <button
              v-for="color in COLOR_PRESETS"
              :key="color"
              class="color-swatch"
              :class="{ active: cloneColor === color }"
              :style="{ backgroundColor: color }"
              @click="cloneColor = color"
            />
            <input v-model="cloneColor" type="color" class="color-custom" title="Couleur custom" />
          </div>
        </div>
        <div class="clone-form-actions">
          <button class="btn-cancel-small" @click="showCloneForm = false">Annuler</button>
          <button
            class="btn-create-clone"
            :disabled="cloning"
            @click="handleClone"
          >
            {{ cloning ? "Clonage en cours..." : "Cloner" }}
          </button>
        </div>
      </div>

      <button
        v-else
        class="btn-clone"
        :disabled="!selectedDevice || cloning || loadingClones"
        @click="showCloneForm = true"
      >
        + Créer un nouveau clone
      </button>

      <ConfirmModal
        :visible="!!confirmDeletePkg"
        title="Supprimer le clone"
        :message="`Supprimer ${confirmDeletePkg ? displayName(confirmDeletePkg) : ''} du device ? Cette action est irréversible.`"
        confirm-label="Supprimer"
        danger
        @confirm="confirmRemove"
        @cancel="confirmDeletePkg = null"
      />

      <p v-if="cloneError" class="error">{{ cloneError }}</p>

      <div class="actions">
        <button class="btn-cancel" @click="$emit('close')">Annuler</button>
        <button
          class="btn-create"
          :disabled="!selectedDevice || !selectedPackage || loadingClones"
          @click="handleCreate($emit)"
        >
          Lancer
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.dialog {
  background: #1a1a2e;
  border: 1px solid #333;
  border-radius: 12px;
  padding: 1.5rem;
  min-width: 440px;
  max-width: 520px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
}

h3 { margin-top: 0; margin-bottom: 1rem; }

label {
  display: block;
  margin-bottom: 1rem;
  font-size: 0.85rem;
  color: #aaa;
}

select {
  display: block;
  width: 100%;
  margin-top: 0.25rem;
  padding: 0.5rem 0.75rem;
  background: #16213e;
  border: 1px solid #333;
  border-radius: 6px;
  color: #e0e0e0;
  font-size: 0.9rem;
  box-sizing: border-box;
}

select:focus { outline: none; border-color: #533483; }

.section-label {
  font-size: 0.85rem;
  color: #aaa;
  margin-bottom: 0.5rem;
}

.clone-list {
  margin-top: 0.5rem;
  max-height: 220px;
  overflow-y: auto;
}

.clone-item {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.5rem 0.75rem;
  background: #16213e;
  border: 1px solid transparent;
  border-radius: 6px;
  margin-bottom: 0.25rem;
  cursor: pointer;
  transition: all 0.15s;
}

.clone-item:hover { background: #1a2744; }
.clone-item.selected { border-color: #533483; background: #1e2a4a; }

.clone-icon-wrap {
  width: 36px;
  height: 36px;
  border-radius: 8px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.clone-icon {
  width: 32px;
  height: 32px;
  border-radius: 6px;
}

.clone-info {
  flex: 1;
  min-width: 0;
}

.clone-name {
  display: block;
  font-size: 0.9rem;
  font-weight: 500;
}

.clone-pkg {
  display: block;
  font-size: 0.7rem;
  color: #555;
  font-family: monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.clone-remove {
  background: none;
  border: 1px solid #333;
  color: #888;
  font-size: 1rem;
  cursor: pointer;
  padding: 0.3rem 0.6rem;
  border-radius: 6px;
  line-height: 1;
  flex-shrink: 0;
  min-width: 32px;
  min-height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  z-index: 10;
  position: relative;
}

.clone-remove:hover { color: #f44336; background: rgba(244, 67, 54, 0.15); border-color: #f44336; }

/* Clone form */
.clone-form {
  background: #16213e;
  border: 1px solid #333;
  border-radius: 8px;
  padding: 0.75rem;
  margin-bottom: 1rem;
}

.clone-name-input {
  width: 100%;
  padding: 0.5rem 0.75rem;
  background: #0d1117;
  border: 1px solid #333;
  border-radius: 6px;
  color: #e0e0e0;
  font-size: 0.9rem;
  box-sizing: border-box;
  margin-bottom: 0.75rem;
}

.clone-name-input:focus { outline: none; border-color: #533483; }

.color-picker { margin-bottom: 0.75rem; }

.color-label {
  display: block;
  font-size: 0.8rem;
  color: #888;
  margin-bottom: 0.4rem;
}

.color-presets {
  display: flex;
  gap: 0.4rem;
  align-items: center;
}

.color-swatch {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  border: 2px solid transparent;
  cursor: pointer;
  transition: all 0.15s;
}

.color-swatch:hover { transform: scale(1.15); }
.color-swatch.active { border-color: #fff; box-shadow: 0 0 8px rgba(255,255,255,0.3); }

.color-custom {
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 50%;
  cursor: pointer;
  background: none;
  padding: 0;
}

.clone-form-actions {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-end;
}

.btn-cancel-small {
  padding: 0.4rem 0.75rem;
  background: #333;
  border: none;
  border-radius: 6px;
  color: #aaa;
  cursor: pointer;
  font-size: 0.8rem;
}

.btn-create-clone {
  padding: 0.4rem 0.75rem;
  background: #533483;
  border: none;
  border-radius: 6px;
  color: #fff;
  cursor: pointer;
  font-size: 0.8rem;
}

.btn-create-clone:disabled { opacity: 0.4; cursor: not-allowed; }

.btn-clone {
  width: 100%;
  padding: 0.5rem;
  background: #1e2a4a;
  border: 1px dashed #533483;
  border-radius: 6px;
  color: #888;
  cursor: pointer;
  font-size: 0.85rem;
  margin-bottom: 1rem;
  transition: all 0.15s;
}

.btn-clone:hover:not(:disabled) { background: #252f4a; color: #bbb; }
.btn-clone:disabled { opacity: 0.4; cursor: not-allowed; }

.actions {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-end;
  margin-top: 1rem;
}

.btn-cancel, .btn-create {
  padding: 0.5rem 1rem;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  font-size: 0.85rem;
}

.btn-cancel { background: #333; color: #aaa; }
.btn-cancel:hover { background: #444; }
.btn-create { background: #533483; color: #fff; }
.btn-create:hover { background: #6441a5; }
.btn-create:disabled { opacity: 0.4; cursor: not-allowed; }

.error { color: #f44336; font-size: 0.8rem; margin: 0.5rem 0; }
.muted { color: #666; font-size: 0.85rem; padding: 0.5rem; }
</style>
