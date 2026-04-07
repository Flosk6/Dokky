<script setup lang="ts">
import type { SessionInfo } from "../types";

defineProps<{
  session: SessionInfo;
  active: boolean;
}>();

defineEmits<{
  click: [];
  close: [];
}>();
</script>

<template>
  <div class="tab" :class="{ active }" @click="$emit('click')">
    <span class="tab-label">
      {{ session.app_package.split('.').pop() }}
    </span>
    <span class="tab-device">{{ session.device_serial.slice(0, 8) }}</span>
    <button class="tab-close" @click.stop="$emit('close')" title="Fermer">&times;</button>
  </div>
</template>

<style scoped>
.tab {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: #16213e;
  border-radius: 8px 8px 0 0;
  cursor: pointer;
  min-width: 120px;
  max-width: 220px;
  user-select: none;
  border: 1px solid transparent;
  border-bottom: none;
  transition: background 0.15s;
}

.tab:hover {
  background: #1a2744;
}

.tab.active {
  background: #0f3460;
  border-color: #533483;
}

.tab-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.85rem;
  font-weight: 500;
}

.tab-device {
  font-size: 0.7rem;
  color: #666;
  font-family: monospace;
}

.tab-close {
  background: none;
  border: none;
  color: #666;
  cursor: pointer;
  font-size: 1.1rem;
  padding: 0 0.2rem;
  line-height: 1;
  border-radius: 4px;
}

.tab-close:hover {
  color: #f44336;
  background: rgba(244, 67, 54, 0.15);
}
</style>
