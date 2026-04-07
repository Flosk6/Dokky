<script setup lang="ts">
import type { SessionInfo } from "../types";

defineProps<{
  session: SessionInfo;
  active: boolean;
  icon?: string | null;
}>();

defineEmits<{
  click: [];
  close: [];
}>();
</script>

<template>
  <div class="tab" :class="{ active }" @click="$emit('click')">
    <img v-if="icon" :src="`data:image/png;base64,${icon}`" class="tab-icon" alt="" />
    <span class="tab-label">
      {{ session.app_package.split('.').pop() }}
    </span>
    <button class="tab-close" @click.stop="$emit('close')" title="Fermer">&times;</button>
    <div v-if="active" class="tab-indicator" />
  </div>
</template>

<style scoped>
.tab {
  position: relative;
  display: flex;
  align-items: center;
  gap: 6px;
  height: var(--topbar-height);
  padding: 0 14px;
  background: transparent;
  border-right: 1px solid var(--border);
  cursor: pointer;
  user-select: none;
  min-width: 80px;
  max-width: 180px;
  transition: background 0.15s;
}

.tab:hover {
  background: var(--bg-hover);
}

.tab.active {
  background: var(--bg-secondary);
}

.tab-icon {
  width: 20px;
  height: 20px;
  border-radius: var(--radius-sm);
  flex-shrink: 0;
}

.tab-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 0.82rem;
  font-weight: 500;
  color: var(--text-secondary);
}

.tab.active .tab-label {
  color: #fff;
  font-weight: 600;
}

.tab-close {
  background: none;
  border: none;
  color: transparent;
  cursor: pointer;
  font-size: 1rem;
  padding: 0 2px;
  border-radius: var(--radius-sm);
  line-height: 1;
  flex-shrink: 0;
  transition: all 0.15s;
}

.tab:hover .tab-close {
  color: var(--text-muted);
}

.tab-close:hover {
  color: var(--danger) !important;
  background: rgba(244, 67, 54, 0.15);
}

.tab-indicator {
  position: absolute;
  bottom: 0;
  left: 8px;
  right: 8px;
  height: 2px;
  background: var(--accent);
  border-radius: 1px;
}
</style>
