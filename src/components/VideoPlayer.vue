<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Channel } from "@tauri-apps/api/core";
import { useShortcuts } from "../composables/useShortcuts";
import { defineAsyncComponent } from "vue";
const ShortcutOverlay = defineAsyncComponent(() => import("./ShortcutOverlay.vue"));

const props = defineProps<{
  sessionId: string;
  width: number;
  height: number;
  shortcutMode?: boolean;
}>();

const emit = defineEmits<{
  error: [message: string];
  closeShortcuts: [];
}>();

const { config, captureMode, captureCallback, endCapture, registerTouchSender } = useShortcuts();

const canvasRef = ref<HTMLCanvasElement | null>(null);
const wrapRef = ref<HTMLElement | null>(null);
const canvasStyle = ref({ width: '100%', height: '100%' });
let resizeObserver: ResizeObserver | null = null;

function updateCanvasSize() {
  const wrap = wrapRef.value;
  if (!wrap || !props.width || !props.height) return;
  const containerW = wrap.clientWidth;
  const containerH = wrap.clientHeight;
  const aspect = props.width / props.height;
  const containerAspect = containerW / containerH;

  let cssW: number, cssH: number;
  if (containerAspect > aspect) {
    // Container is wider → fit height
    cssH = containerH;
    cssW = cssH * aspect;
  } else {
    // Container is taller → fit width
    cssW = containerW;
    cssH = cssW / aspect;
  }
  canvasStyle.value = {
    width: `${Math.round(cssW)}px`,
    height: `${Math.round(cssH)}px`,
  };
}
let decoder: VideoDecoder | null = null;
let configData: Uint8Array | null = null;
let isConfigured = false;

// --- H.264 SPS parsing to extract codec string ---
function findSPSInAnnexB(data: Uint8Array): Uint8Array | null {
  for (let i = 0; i < data.length - 4; i++) {
    // Look for start code 00 00 00 01
    if (data[i] === 0 && data[i + 1] === 0 && data[i + 2] === 0 && data[i + 3] === 1) {
      const nalType = data[i + 4] & 0x1f;
      if (nalType === 7) {
        // SPS found — find end (next start code or end of data)
        let end = data.length;
        for (let j = i + 5; j < data.length - 3; j++) {
          if (
            data[j] === 0 &&
            data[j + 1] === 0 &&
            ((data[j + 2] === 0 && data[j + 3] === 1) || data[j + 2] === 1)
          ) {
            end = j;
            break;
          }
        }
        return data.slice(i + 4, end);
      }
    }
  }
  return null;
}

function codecStringFromSPS(sps: Uint8Array): string {
  // SPS NAL: [nal_header] [profile_idc] [constraint_flags] [level_idc] ...
  const profileIdc = sps[1];
  const constraintFlags = sps[2];
  const levelIdc = sps[3];
  return (
    "avc1." +
    profileIdc.toString(16).padStart(2, "0") +
    constraintFlags.toString(16).padStart(2, "0") +
    levelIdc.toString(16).padStart(2, "0")
  );
}

// --- Touch handling ---
function sendTouch(action: number, event: MouseEvent) {
  const canvas = canvasRef.value;
  if (!canvas) return;

  const rect = canvas.getBoundingClientRect();
  const scaleX = props.width / rect.width;
  const scaleY = props.height / rect.height;
  const x = Math.round((event.clientX - rect.left) * scaleX);
  const y = Math.round((event.clientY - rect.top) * scaleY);

  invoke("send_touch", {
    sessionId: props.sessionId,
    action,
    x,
    y,
    width: props.width,
    height: props.height,
  }).catch(() => {});
}

/// Send a tap at a random position within a normalized zone (simulates human-like taps).
function sendTapInZone(cx: number, cy: number, w: number, h: number) {
  // Randomize within the zone
  const normX = cx + (Math.random() - 0.5) * w;
  const normY = cy + (Math.random() - 0.5) * h;
  const x = Math.round(Math.max(0, Math.min(1, normX)) * props.width);
  const y = Math.round(Math.max(0, Math.min(1, normY)) * props.height);
  // Simulate DOWN then UP with slight random delay (30-80ms)
  invoke("send_touch", { sessionId: props.sessionId, action: 0, x, y, width: props.width, height: props.height }).catch(() => {});
  setTimeout(() => {
    invoke("send_touch", { sessionId: props.sessionId, action: 1, x, y, width: props.width, height: props.height }).catch(() => {});
  }, 30 + Math.random() * 50);
}

// Register this player's touch sender for game action shortcuts
registerTouchSender((key: string) => {
  const action = config.value.game_actions.find((a) => a.key.toLowerCase() === key.toLowerCase());
  if (action) {
    sendTapInZone(action.x, action.y, action.w ?? 0.02, action.h ?? 0.02);
  }
});

function onMouseDown(e: MouseEvent) {
  // Capture mode: record normalized coordinates
  if (captureMode.value && captureCallback.value) {
    const canvas = canvasRef.value;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const normX = (e.clientX - rect.left) / rect.width;
    const normY = (e.clientY - rect.top) / rect.height;
    captureCallback.value(normX, normY);
    endCapture();
    return;
  }
  sendTouch(0, e); // ACTION_DOWN
}

function onMouseUp(e: MouseEvent) {
  if (captureMode.value) return;
  sendTouch(1, e); // ACTION_UP
}

function onMouseMove(e: MouseEvent) {
  if (captureMode.value) return;
  if (e.buttons & 1) {
    sendTouch(2, e); // ACTION_MOVE (only while pressed)
  }
}

// --- Keyboard handling ---
// Map browser key names to Android keycodes
const KEY_MAP: Record<string, number> = {
  Enter: 66,
  Backspace: 67,
  Delete: 112,
  Tab: 61,
  Escape: 111,
  ArrowUp: 19,
  ArrowDown: 20,
  ArrowLeft: 21,
  ArrowRight: 22,
  Home: 122,
  End: 123,
};

function getMetaState(e: KeyboardEvent): number {
  let meta = 0;
  if (e.shiftKey) meta |= 0x1;
  if (e.altKey) meta |= 0x2;
  if (e.ctrlKey || e.metaKey) meta |= 0x1000;
  return meta;
}

function onKeyDown(e: KeyboardEvent) {
  if (captureMode.value) return;
  // Don't intercept browser/app shortcuts
  if (e.ctrlKey || e.metaKey) return;

  const keycode = KEY_MAP[e.key];
  if (keycode) {
    e.preventDefault();
    invoke("send_key", {
      sessionId: props.sessionId,
      action: 0, // ACTION_DOWN
      keycode,
      repeat: 0,
      metastate: getMetaState(e),
    }).catch(() => {});
  } else if (e.key.length === 1) {
    // Printable character → inject as text
    e.preventDefault();
    invoke("send_text", {
      sessionId: props.sessionId,
      text: e.key,
    }).catch(() => {});
  }
}

function onKeyUp(e: KeyboardEvent) {
  if (captureMode.value) return;
  if (e.ctrlKey || e.metaKey) return;

  const keycode = KEY_MAP[e.key];
  if (keycode) {
    e.preventDefault();
    invoke("send_key", {
      sessionId: props.sessionId,
      action: 1, // ACTION_UP
      keycode,
      repeat: 0,
      metastate: getMetaState(e),
    }).catch(() => {});
  }
}

// --- Video decoder setup ---
function setupDecoder() {
  const canvas = canvasRef.value;
  if (!canvas) return;

  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  decoder = new VideoDecoder({
    output: (frame: VideoFrame) => {
      ctx.drawImage(frame, 0, 0, canvas.width, canvas.height);
      frame.close();
    },
    error: (e: DOMException) => {
      console.error("VideoDecoder error:", e);
      emit("error", e.message);
    },
  });
}

function configureDecoder(annexBConfig: Uint8Array) {
  if (!decoder) return;

  const sps = findSPSInAnnexB(annexBConfig);
  if (!sps) {
    console.error("No SPS found in config packet");
    return;
  }

  const codec = codecStringFromSPS(sps);
  console.log("Configuring decoder with codec:", codec);

  decoder.configure({ codec });
  isConfigured = true;
}

// --- Start video stream ---
async function startStreaming() {
  setupDecoder();

  const channel = new Channel<{
    config: boolean;
    keyframe: boolean;
    pts: number;
    data: string;
  }>();

  let packetCount = 0;
  channel.onmessage = (packet) => {
    packetCount++;
    const raw = Uint8Array.from(atob(packet.data), (c) => c.charCodeAt(0));

    if (packetCount <= 5) {
      console.log(`[video] packet #${packetCount}: config=${packet.config} keyframe=${packet.keyframe} size=${raw.length} pts=${packet.pts}`);
    }

    if (packet.config) {
      console.log("[video] Received config packet, configuring decoder...");
      configData = raw;
      configureDecoder(raw);
      return;
    }

    if (!decoder || !isConfigured) return;

    // For keyframes, prepend SPS/PPS config data
    let frameData: Uint8Array;
    if (packet.keyframe && configData) {
      frameData = new Uint8Array(configData.length + raw.length);
      frameData.set(configData, 0);
      frameData.set(raw, configData.length);
    } else {
      frameData = raw;
    }

    try {
      decoder.decode(
        new EncodedVideoChunk({
          type: packet.keyframe ? "key" : "delta",
          timestamp: packet.pts,
          data: frameData,
        })
      );
    } catch (e) {
      console.error("Decode error:", e);
    }
  };

  try {
    console.log("[video] Calling start_video_stream for session:", props.sessionId);
    await invoke("start_video_stream", {
      sessionId: props.sessionId,
      onPacket: channel,
    });
    console.log("[video] Stream ended normally");
  } catch (e) {
    console.error("[video] Stream error:", e);
  }
}

onMounted(() => {
  startStreaming();
  // Track container size to scale canvas properly
  if (wrapRef.value) {
    resizeObserver = new ResizeObserver(() => updateCanvasSize());
    resizeObserver.observe(wrapRef.value);
    updateCanvasSize();
  }
});

onUnmounted(() => {
  if (decoder && decoder.state !== "closed") {
    decoder.close();
  }
  resizeObserver?.disconnect();
});

</script>

<template>
  <div ref="wrapRef" class="video-wrap">
    <div class="video-inner" :style="canvasStyle">
      <canvas
        ref="canvasRef"
        :width="width"
        :height="height"
        class="video-canvas"
        tabindex="0"
        @mousedown="onMouseDown"
        @mouseup="onMouseUp"
        @mousemove="onMouseMove"
        @keydown="onKeyDown"
        @keyup="onKeyUp"
        @contextmenu.prevent
      />
      <ShortcutOverlay
        :active="!!shortcutMode"
        :canvas-width="width"
        :canvas-height="height"
        @close="emit('closeShortcuts')"
      />
    </div>
  </div>
</template>

<style scoped>
.video-wrap {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 100%;
  height: 100%;
}

.video-inner {
  position: relative;
}

.video-canvas {
  width: 100%;
  height: 100%;
  display: block;
  cursor: crosshair;
  background: #000;
}
</style>
