import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Device } from "../types";

export function useDevices(pollIntervalMs = 3000, slowIntervalMs = 10000) {
  const devices = ref<Device[]>([]);
  const error = ref<string | null>(null);
  let intervalId: ReturnType<typeof setInterval> | null = null;
  let currentInterval = pollIntervalMs;

  async function fetchDevices() {
    try {
      devices.value = await invoke<Device[]>("get_devices");
      error.value = null;
    } catch (e) {
      error.value = String(e);
    }
  }

  /** Switch to slower polling (e.g. when sessions are active). */
  function setSlowPolling(slow: boolean) {
    const desired = slow ? slowIntervalMs : pollIntervalMs;
    if (desired !== currentInterval) {
      currentInterval = desired;
      if (intervalId) clearInterval(intervalId);
      intervalId = setInterval(fetchDevices, currentInterval);
    }
  }

  onMounted(() => {
    fetchDevices();
    intervalId = setInterval(fetchDevices, currentInterval);
  });

  onUnmounted(() => {
    if (intervalId) clearInterval(intervalId);
  });

  return { devices, error, fetchDevices, setSlowPolling };
}
