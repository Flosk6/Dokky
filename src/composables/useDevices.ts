import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Device } from "../types";

export function useDevices(pollIntervalMs = 3000) {
  const devices = ref<Device[]>([]);
  const error = ref<string | null>(null);
  let intervalId: ReturnType<typeof setInterval> | null = null;

  async function fetchDevices() {
    try {
      devices.value = await invoke<Device[]>("get_devices");
      error.value = null;
    } catch (e) {
      error.value = String(e);
    }
  }

  onMounted(() => {
    fetchDevices();
    intervalId = setInterval(fetchDevices, pollIntervalMs);
  });

  onUnmounted(() => {
    if (intervalId) clearInterval(intervalId);
  });

  return { devices, error, fetchDevices };
}
