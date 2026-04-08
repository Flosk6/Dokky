<script setup lang="ts">
import type { SessionInfo } from "../types";
import SessionTab from "./SessionTab.vue";

defineProps<{
  sessions: SessionInfo[];
  activeSessionId: string | null;
  iconMap?: Record<string, string>;
}>();

defineEmits<{
  select: [sessionId: string];
  close: [sessionId: string];
  newSession: [];
}>();
</script>

<template>
  <div class="topbar">
    <div class="tabs">
      <SessionTab
        v-for="session in sessions"
        :key="session.id"
        :session="session"
        :active="session.id === activeSessionId"
        :icon="iconMap?.[session.app_package]"
        :display-name="session.display_name"
        @click="$emit('select', session.id)"
        @close="$emit('close', session.id)"
      />
    </div>
    <button class="new-btn" title="Nouvelle instance (Ctrl+T)" @click="$emit('newSession')">
      <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
        <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.topbar {
  height: var(--topbar-height);
  background: var(--bg-primary);
  border-bottom: 1px solid var(--border);
  display: flex;
  align-items: stretch;
}

.tabs {
  display: flex;
  flex: 1;
  overflow-x: auto;
  overflow-y: hidden;
}

.new-btn {
  width: var(--sidebar-width);
  height: var(--topbar-height);
  background: transparent;
  border: none;
  border-left: 1px solid var(--border);
  color: var(--text-secondary);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
  transition: all 0.15s;
}

.new-btn:hover {
  background: var(--accent);
  color: #fff;
}
</style>
