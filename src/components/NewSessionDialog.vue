<script setup lang="ts">
import { ref, computed, watch } from "vue";
import type { Device } from "../types";
import { useClones } from "../composables/useClones";
import Loader from "./Loader.vue";

const props = defineProps<{
  devices: Device[];
  visible: boolean;
}>();

defineEmits<{
  close: [];
  create: [deviceSerial: string, appPackage: string];
}>();

const connectedDevices = computed(() =>
  props.devices.filter((d) => d.status === "device")
);

const selectedDevice = ref("");
const selectedPackage = ref("");
let emitted = false;

const devicesRef = computed(() => props.devices);
const { getClones, isLoading } = useClones(devicesRef);

const clones = computed(() => getClones(selectedDevice.value));
const loading = computed(() => isLoading(selectedDevice.value));

function handleCreate(emit: (event: "create", deviceSerial: string, appPackage: string) => void) {
  if (emitted || !selectedDevice.value || !selectedPackage.value) return;
  emitted = true;
  emit("create", selectedDevice.value, selectedPackage.value);
  setTimeout(() => { emitted = false; }, 500);
}

watch(selectedDevice, () => {
  selectedPackage.value = "";
});

watch(clones, (list) => {
  if (list.length > 0 && !selectedPackage.value) {
    selectedPackage.value = list[0].package;
  }
});

watch(() => props.visible, (visible) => {
  if (visible) {
    if (!selectedDevice.value && connectedDevices.value.length > 0) {
      selectedDevice.value = connectedDevices.value[0].serial;
    }
  }
});
</script>

<template>
  <div v-if="visible" class="overlay" @click.self="$emit('close')">
    <div class="dialog">
      <h3>Lancer une instance</h3>

      <!-- Device select (only if multiple) -->
      <div v-if="connectedDevices.length > 1" class="field">
        <span class="field-label">Device</span>
        <select v-model="selectedDevice">
          <option v-for="dev in connectedDevices" :key="dev.serial" :value="dev.serial">
            {{ dev.model ?? dev.serial }}
          </option>
        </select>
      </div>

      <!-- Clone list -->
      <div class="field">
        <span class="field-label">Compte</span>
        <Loader v-if="loading" label="Chargement..." :size="18" />
        <div v-else-if="clones.length === 0" class="empty">
          Aucun Dofus Touch installé.<br />
          Créez des clones dans les paramètres ⚙
        </div>
        <div v-else class="clone-list">
          <div
            v-for="clone in clones"
            :key="clone.package"
            class="clone-item"
            :class="{ selected: clone.package === selectedPackage }"
            @click="selectedPackage = clone.package"
          >
            <img
              v-if="clone.icon"
              :src="`data:image/png;base64,${clone.icon}`"
              class="clone-icon"
            />
            <div class="clone-info">
              <span class="clone-name">{{ clone.display_name }}</span>
              <span class="clone-pkg">{{ clone.package }}</span>
            </div>
          </div>
        </div>
      </div>

      <div class="actions">
        <button class="btn cancel" @click="$emit('close')">Annuler</button>
        <button
          class="btn primary"
          :disabled="!selectedDevice || !selectedPackage || loading"
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
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 20px;
  min-width: 360px;
  max-width: 440px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

h3 { margin: 0 0 16px; font-size: 1rem; font-weight: 600; }

.field { margin-bottom: 16px; }

.field-label {
  display: block;
  font-size: 0.75rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted);
  margin-bottom: 8px;
}

select {
  width: 100%;
  padding: 8px 10px;
  background: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-size: 0.85rem;
}

select:focus { outline: none; border-color: var(--accent); }

.clone-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 240px;
  overflow-y: auto;
}

.clone-item {
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

.clone-item:hover { border-color: var(--text-muted); }
.clone-item.selected { border-color: var(--accent); background: var(--bg-hover); }

.clone-icon {
  width: 32px;
  height: 32px;
  border-radius: var(--radius-sm);
  flex-shrink: 0;
}

.clone-info { flex: 1; min-width: 0; }

.clone-name {
  display: block;
  font-size: 0.85rem;
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

.actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-top: 20px;
}

.btn {
  padding: 8px 16px;
  border-radius: var(--radius-md);
  border: none;
  cursor: pointer;
  font-size: 0.85rem;
  font-weight: 500;
}

.btn.cancel { background: var(--border); color: var(--text-secondary); }
.btn.cancel:hover { background: var(--bg-hover); color: var(--text-primary); }
.btn.primary { background: var(--accent); color: #fff; }
.btn.primary:hover { background: var(--accent-hover); }
.btn.primary:disabled { opacity: 0.4; cursor: not-allowed; }

.empty {
  color: var(--text-muted);
  font-size: 0.82rem;
  text-align: center;
  padding: 16px;
  line-height: 1.5;
}
</style>
