<script setup lang="ts">
import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Device } from "../types";

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
const packages = ref<string[]>([]);
const loadingPackages = ref(false);
let emitted = false;

function handleCreate(emit: (event: "create", deviceSerial: string, appPackage: string) => void) {
  if (emitted) return;
  if (selectedDevice.value && selectedPackage.value) {
    emitted = true;
    emit("create", selectedDevice.value, selectedPackage.value);
    setTimeout(() => { emitted = false; }, 500);
  }
}

// Load dofus packages when device selection changes
async function loadPackages(serial: string) {
  if (!serial) {
    packages.value = [];
    return;
  }
  loadingPackages.value = true;
  try {
    packages.value = await invoke<string[]>("get_packages", {
      deviceSerial: serial,
      filter: "dofus",
    });
    // Auto-select first package
    selectedPackage.value = packages.value[0] ?? "";
  } catch (e) {
    console.error("Failed to load packages:", e);
    packages.value = [];
  } finally {
    loadingPackages.value = false;
  }
}

watch(selectedDevice, (serial) => {
  loadPackages(serial);
});

// Auto-select first device + load packages when dialog opens
watch(() => props.visible, (visible) => {
  if (visible && !selectedDevice.value && props.devices.length > 0) {
    selectedDevice.value = props.devices[0].serial;
  }
});
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

      <label>
        Application
        <select v-model="selectedPackage" :disabled="loadingPackages || packages.length === 0">
          <option v-if="loadingPackages" value="" disabled>Chargement...</option>
          <option v-else-if="packages.length === 0" value="" disabled>Aucun package Dofus trouvé</option>
          <option
            v-for="pkg in packages"
            :key="pkg"
            :value="pkg"
          >
            {{ pkg }}
          </option>
        </select>
      </label>

      <div class="actions">
        <button class="btn-cancel" @click="$emit('close')">Annuler</button>
        <button
          class="btn-create"
          :disabled="!selectedDevice || !selectedPackage || loadingPackages"
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
  min-width: 360px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
}

h3 {
  margin-top: 0;
  margin-bottom: 1rem;
}

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

select:focus {
  outline: none;
  border-color: #533483;
}

select:disabled {
  opacity: 0.5;
}

.actions {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-end;
  margin-top: 1.5rem;
}

.btn-cancel,
.btn-create {
  padding: 0.5rem 1rem;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  font-size: 0.85rem;
}

.btn-cancel {
  background: #333;
  color: #aaa;
}

.btn-cancel:hover {
  background: #444;
}

.btn-create {
  background: #533483;
  color: #fff;
}

.btn-create:hover {
  background: #6441a5;
}

.btn-create:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>
