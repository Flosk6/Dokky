<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useShortcuts } from "../composables/useShortcuts";
import { defineAsyncComponent } from "vue";
const ShortcutOverlay = defineAsyncComponent(() => import("./ShortcutOverlay.vue"));

const props = defineProps<{
  sessionId: string;
  width: number;
  height: number;
  shortcutMode?: boolean;
  active?: boolean;
}>();

const emit = defineEmits<{
  error: [message: string];
  closeShortcuts: [];
}>();

const { captureMode, captureCallback, endCapture, registerSession, unregisterSession } = useShortcuts();

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

// Keyboard handling is centralized in useShortcuts (auto-detects Android keyboard state)

function setupDecoder() {
  // Decoder is set up inside startStreaming with rAF-based rendering
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

// --- Start video stream (poll-based, raw binary) ---
let streaming = true;

async function startStreaming() {
  setupDecoder();

  let packetCount = 0;
  let pendingFrame: VideoFrame | null = null;

  // rAF loop: always draw to keep canvas up-to-date (even for hidden tabs)
  function drawLoop() {
    if (!streaming) return;
    if (pendingFrame) {
      const canvas = canvasRef.value;
      const ctx = canvas?.getContext("2d");
      if (ctx && canvas) {
        ctx.drawImage(pendingFrame, 0, 0, canvas.width, canvas.height);
      }
      pendingFrame.close();
      pendingFrame = null;
    }
    requestAnimationFrame(drawLoop);
  }
  requestAnimationFrame(drawLoop);

  if (decoder) {
    decoder.close();
  }
  decoder = new VideoDecoder({
    output: (frame: VideoFrame) => {
      if (pendingFrame) pendingFrame.close();
      pendingFrame = frame;
    },
    error: (e: DOMException) => {
      console.error("VideoDecoder error:", e);
      emit("error", e.message);
    },
  });

  while (streaming) {
    try {
      const buf: ArrayBuffer = await invoke("read_video_packet", {
        sessionId: props.sessionId,
      });
      const view = new DataView(buf);
      const raw = new Uint8Array(buf);

      const flags = view.getUint8(0);
      const isConfig = (flags & 1) !== 0;
      const isKeyframe = (flags & 2) !== 0;
      const ptsHi = view.getInt32(1);
      const ptsLo = view.getUint32(5);
      const pts = ptsHi * 0x100000000 + ptsLo;
      const h264Data = raw.subarray(13);

      packetCount++;
      if (packetCount <= 3) {
        console.log(`[video] packet #${packetCount}: config=${isConfig} keyframe=${isKeyframe} size=${h264Data.length}`);
      }

      if (isConfig) {
        configData = h264Data;
        configureDecoder(h264Data);
        continue;
      }

      if (!decoder || !isConfigured) continue;

      let frameData: Uint8Array;
      if (isKeyframe && configData) {
        frameData = new Uint8Array(configData.length + h264Data.length);
        frameData.set(configData, 0);
        frameData.set(h264Data, configData.length);
      } else {
        frameData = h264Data;
      }

      decoder.decode(
        new EncodedVideoChunk({
          type: isKeyframe ? "key" : "delta",
          timestamp: pts,
          data: frameData,
        })
      );
    } catch (e) {
      if (streaming) {
        console.error("[video] Stream error:", e);
      }
      break;
    }
  }
}

onMounted(() => {
  registerSession(props.sessionId, props.width, props.height);
  startStreaming();
  if (wrapRef.value) {
    resizeObserver = new ResizeObserver(() => updateCanvasSize());
    resizeObserver.observe(wrapRef.value);
    updateCanvasSize();
  }
});

onUnmounted(() => {
  streaming = false;
  unregisterSession(props.sessionId);
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
  cursor: default;
  background: #000;
}
</style>
