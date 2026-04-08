import { computed } from "vue";
import { useShortcuts } from "./useShortcuts";
import type { VideoSettings } from "../types";

export interface VideoPreset {
  name: string;
  label: string;
  description: string;
  width: number;
  height: number;
  dpi: number;
  fps: number;
  bitrate: number;
  baseline_profile: boolean;
  iframe_interval: number;
  no_vd_system_decorations: boolean;
  disable_animations: boolean;
  screen_off: boolean;
}

export const VIDEO_PRESETS: VideoPreset[] = [
  {
    name: "ultra", label: "Ultra", description: "2560x1440 · 320 dpi · 60 FPS · 16 Mbps",
    width: 2560, height: 1440, dpi: 320, fps: 60, bitrate: 16_000_000,
    baseline_profile: false, iframe_interval: 2, no_vd_system_decorations: false, disable_animations: false, screen_off: false,
  },
  {
    name: "high", label: "High", description: "1920x1080 · 240 dpi · 60 FPS · 8 Mbps",
    width: 1920, height: 1080, dpi: 240, fps: 60, bitrate: 8_000_000,
    baseline_profile: true, iframe_interval: 2, no_vd_system_decorations: true, disable_animations: false, screen_off: false,
  },
  {
    name: "medium", label: "Medium", description: "1280x720 · 160 dpi · 45 FPS · 4 Mbps",
    width: 1280, height: 720, dpi: 160, fps: 45, bitrate: 4_000_000,
    baseline_profile: true, iframe_interval: 2, no_vd_system_decorations: true, disable_animations: false, screen_off: false,
  },
  {
    name: "low", label: "Low", description: "960x540 · 120 dpi · 30 FPS · 2 Mbps",
    width: 960, height: 540, dpi: 120, fps: 30, bitrate: 2_000_000,
    baseline_profile: true, iframe_interval: 3, no_vd_system_decorations: true, disable_animations: true, screen_off: false,
  },
];

const DEFAULT_VIDEO_SETTINGS: VideoSettings = {
  width: 1920, height: 1080, dpi: 240, fps: 60, bitrate: 8_000_000,
  baseline_profile: true, iframe_interval: 2, no_vd_system_decorations: true, disable_animations: false, screen_off: false,
};

export function useVideoPreset() {
  const { config, saveConfig } = useShortcuts();

  const selectedPresetName = computed(() => config.value.video_preset ?? "high");
  const isCustom = computed(() => selectedPresetName.value === "custom");

  /** The active preset (null if custom) */
  const preset = computed(() =>
    VIDEO_PRESETS.find((p) => p.name === selectedPresetName.value) ?? null
  );

  /** Effective settings: from preset or custom */
  const effectiveSettings = computed<VideoSettings>(() => {
    if (preset.value) {
      return {
        width: preset.value.width,
        height: preset.value.height,
        dpi: preset.value.dpi,
        fps: preset.value.fps,
        bitrate: preset.value.bitrate,
        baseline_profile: preset.value.baseline_profile,
        iframe_interval: preset.value.iframe_interval,
        no_vd_system_decorations: preset.value.no_vd_system_decorations,
        disable_animations: preset.value.disable_animations,
        screen_off: preset.value.screen_off,
      };
    }
    return config.value.video_settings ?? DEFAULT_VIDEO_SETTINGS;
  });

  /** Display spec for scrcpy: WxH/DPI */
  const displaySpec = computed(() =>
    `${effectiveSettings.value.width}x${effectiveSettings.value.height}/${effectiveSettings.value.dpi}`
  );

  function selectPreset(name: string) {
    config.value.video_preset = name;
    // When selecting a named preset, also update video_settings to match
    // so switching to custom starts from a reasonable baseline
    const p = VIDEO_PRESETS.find((v) => v.name === name);
    if (p) {
      config.value.video_settings = {
        width: p.width, height: p.height, dpi: p.dpi, fps: p.fps, bitrate: p.bitrate,
        baseline_profile: p.baseline_profile, iframe_interval: p.iframe_interval,
        no_vd_system_decorations: p.no_vd_system_decorations, disable_animations: p.disable_animations, screen_off: p.screen_off,
      };
    }
    saveConfig();
  }

  function updateCustomSetting<K extends keyof VideoSettings>(key: K, value: VideoSettings[K]) {
    if (!config.value.video_settings) {
      config.value.video_settings = { ...DEFAULT_VIDEO_SETTINGS };
    }
    config.value.video_settings[key] = value;
    config.value.video_preset = "custom";
    saveConfig();
  }

  return {
    selectedPresetName,
    isCustom,
    preset,
    effectiveSettings,
    displaySpec,
    selectPreset,
    updateCustomSetting,
    VIDEO_PRESETS,
  };
}
