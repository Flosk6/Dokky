<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { Channel } from "@tauri-apps/api/core";

const props = defineProps<{
  sessionId: string;
  width: number;
  height: number;
}>();

const emit = defineEmits<{
  error: [message: string];
}>();

const canvasRef = ref<HTMLCanvasElement | null>(null);
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
  sendTouch(0, e); // ACTION_DOWN
}

function onMouseUp(e: MouseEvent) {
  sendTouch(1, e); // ACTION_UP
}

function onMouseMove(e: MouseEvent) {
  if (e.buttons & 1) {
    sendTouch(2, e); // ACTION_MOVE (only while pressed)
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
});

onUnmounted(() => {
  if (decoder && decoder.state !== "closed") {
    decoder.close();
  }
});

</script>

<template>
  <canvas
    ref="canvasRef"
    :width="width"
    :height="height"
    class="video-canvas"
    @mousedown="onMouseDown"
    @mouseup="onMouseUp"
    @mousemove="onMouseMove"
    @contextmenu.prevent
  />
</template>

<style scoped>
.video-canvas {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  cursor: crosshair;
  background: #000;
  border-radius: 8px;
}
</style>
