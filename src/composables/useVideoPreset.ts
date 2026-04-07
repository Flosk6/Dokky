import { ref, computed } from "vue";

export interface VideoPreset {
  name: string;
  resolution: string;
  dpi: number;
  fps: number;
  bitrate: number;
  label: string;
}

export const VIDEO_PRESETS: VideoPreset[] = [
  { name: "ultra", resolution: "2560x1440", dpi: 320, fps: 60, bitrate: 16_000_000, label: "Ultra" },
  { name: "high", resolution: "1920x1080", dpi: 240, fps: 60, bitrate: 8_000_000, label: "High" },
  { name: "medium", resolution: "1280x720", dpi: 160, fps: 45, bitrate: 4_000_000, label: "Medium" },
  { name: "low", resolution: "960x540", dpi: 120, fps: 30, bitrate: 2_000_000, label: "Low" },
];

const selectedPresetName = ref("high");

export function useVideoPreset() {
  const preset = computed(() =>
    VIDEO_PRESETS.find((p) => p.name === selectedPresetName.value) ?? VIDEO_PRESETS[1]
  );

  /** Display spec for scrcpy: WxH/DPI */
  const displaySpec = computed(() =>
    `${preset.value.resolution}/${preset.value.dpi}`
  );

  function select(name: string) {
    selectedPresetName.value = name;
  }

  return { preset, displaySpec, selectedPresetName, select, VIDEO_PRESETS };
}
