<script setup lang="ts">
import type { SessionInfo } from "../types";
import SessionTab from "./SessionTab.vue";

defineProps<{
  sessions: SessionInfo[];
  activeSessionId: string | null;
}>();

defineEmits<{
  select: [sessionId: string];
  close: [sessionId: string];
  newSession: [];
}>();
</script>

<template>
  <div class="tab-bar">
    <div class="tabs">
      <SessionTab
        v-for="session in sessions"
        :key="session.id"
        :session="session"
        :active="session.id === activeSessionId"
        @click="$emit('select', session.id)"
        @close="$emit('close', session.id)"
      />
    </div>
    <button class="new-tab-btn" @click="$emit('newSession')" title="Nouvelle session (Ctrl+T)">
      +
    </button>
  </div>
</template>

<style scoped>
.tab-bar {
  display: flex;
  align-items: flex-end;
  background: #0d1117;
  padding: 0.5rem 0.5rem 0;
  gap: 0.25rem;
  -webkit-app-region: drag;
}

.tabs {
  display: flex;
  gap: 0.25rem;
  flex: 1;
  overflow-x: auto;
  -webkit-app-region: no-drag;
}

.new-tab-btn {
  background: none;
  border: 1px solid #333;
  color: #888;
  font-size: 1.2rem;
  width: 32px;
  height: 32px;
  border-radius: 8px 8px 0 0;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  -webkit-app-region: no-drag;
  transition: all 0.15s;
}

.new-tab-btn:hover {
  background: #16213e;
  color: #e0e0e0;
  border-color: #533483;
}
</style>
