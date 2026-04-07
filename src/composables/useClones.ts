import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Device, CloneInfo } from "../types";

// Shared state: clones per device serial
const clonesByDevice = ref<Record<string, CloneInfo[]>>({});
const loadingDevices = ref<Set<string>>(new Set());
const iconMap = ref<Record<string, string>>({});

export function useClones(devices: { value: Device[] }) {
  // Auto-load clones when new devices appear
  watch(
    () => devices.value.filter((d) => d.status === "device").map((d) => d.serial),
    async (serials) => {
      for (const serial of serials) {
        if (!clonesByDevice.value[serial] && !loadingDevices.value.has(serial)) {
          loadDevice(serial);
        }
      }
      // Cleanup disconnected devices
      for (const key of Object.keys(clonesByDevice.value)) {
        if (!serials.includes(key)) {
          delete clonesByDevice.value[key];
        }
      }
    },
    { immediate: true }
  );

  async function loadDevice(serial: string) {
    loadingDevices.value.add(serial);
    try {
      const clones = await invoke<CloneInfo[]>("get_dofus_clones", { deviceSerial: serial });
      clonesByDevice.value[serial] = clones;
      // Update icon map
      for (const c of clones) {
        if (c.icon) iconMap.value[c.package] = c.icon;
      }
    } catch {
      clonesByDevice.value[serial] = [];
    } finally {
      loadingDevices.value.delete(serial);
    }
  }

  async function refreshDevice(serial: string) {
    await loadDevice(serial);
  }

  function isLoading(serial: string): boolean {
    return loadingDevices.value.has(serial);
  }

  function getClones(serial: string): CloneInfo[] {
    return clonesByDevice.value[serial] ?? [];
  }

  return {
    clonesByDevice,
    iconMap,
    loadingDevices,
    getClones,
    isLoading,
    refreshDevice,
  };
}
