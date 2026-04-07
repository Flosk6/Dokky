<script setup lang="ts">
defineProps<{
  size?: number;
  label?: string;
}>();
</script>

<template>
  <div class="loader-wrap">
    <div class="spinner" :style="{ width: `${size ?? 24}px`, height: `${size ?? 24}px` }" />
    <span v-if="label" class="loader-label">{{ label }}</span>
  </div>
</template>

<style scoped>
.loader-wrap {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 0;
}

.spinner {
  border: 2.5px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  to { transform: rotate(360deg); }
}

.loader-label {
  font-size: 0.82rem;
  color: var(--text-secondary);
}
</style>
