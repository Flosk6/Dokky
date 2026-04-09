import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface LicenseStatus {
  is_pro: boolean;
  license_key: string | null;
  display_key: string | null;
  error: string | null;
}

const isPro = ref(false);
const displayKey = ref<string | null>(null);
const licenseError = ref<string | null>(null);
const loading = ref(false);

export function useLicense() {
  async function checkLicense() {
    loading.value = true;
    try {
      const status = await invoke<LicenseStatus>("check_license");
      isPro.value = status.is_pro;
      displayKey.value = status.display_key;
      licenseError.value = status.error;
    } catch {
      isPro.value = false;
    } finally {
      loading.value = false;
    }
  }

  async function activate(key: string): Promise<string | null> {
    loading.value = true;
    try {
      const status = await invoke<LicenseStatus>("activate_license", { licenseKey: key });
      isPro.value = status.is_pro;
      displayKey.value = status.display_key;
      licenseError.value = status.error;
      return status.error;
    } catch (e) {
      return String(e);
    } finally {
      loading.value = false;
    }
  }

  async function deactivate(): Promise<void> {
    loading.value = true;
    try {
      const status = await invoke<LicenseStatus>("deactivate_license");
      isPro.value = status.is_pro;
      displayKey.value = status.display_key;
      licenseError.value = status.error;
    } finally {
      loading.value = false;
    }
  }

  return {
    isPro,
    displayKey,
    licenseError,
    loading,
    checkLicense,
    activate,
    deactivate,
  };
}
