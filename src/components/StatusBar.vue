<script setup lang="ts">
import type { SessionInfo } from "../types";

defineProps<{
  sessions: SessionInfo[];
  activeSessionId: string | null;
}>();
</script>

<template>
  <div class="status-bar">
    <span>{{ sessions.length }} session{{ sessions.length !== 1 ? 's' : '' }}</span>
    <span v-if="activeSessionId" class="active-info">
      Active: {{ activeSessionId.slice(0, 8) }}
    </span>
  </div>
</template>

<style scoped>
.status-bar {
  display: flex;
  justify-content: space-between;
  padding: 0.35rem 1rem;
  background: #0d1117;
  border-top: 1px solid #1e2a3a;
  font-size: 0.75rem;
  color: #666;
}

.active-info {
  font-family: monospace;
}
</style>
