<script setup lang="ts">
defineEmits<{
  toggleShortcuts: [];
  toggleSettings: [];
}>();
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-top">
      <button class="sidebar-btn disabled" title="Raccourcis (bientôt)" @click="$emit('toggleShortcuts')">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="2" y="6" width="20" height="12" rx="2" />
          <line x1="6" y1="10" x2="6" y2="10.01" />
          <line x1="10" y1="10" x2="10" y2="10.01" />
          <line x1="14" y1="10" x2="14" y2="10.01" />
          <line x1="18" y1="10" x2="18" y2="10.01" />
          <line x1="8" y1="14" x2="16" y2="14" />
        </svg>
      </button>
    </div>
    <div class="sidebar-bottom">
      <button class="sidebar-btn" title="Paramètres" @click="$emit('toggleSettings')">
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
          <circle cx="12" cy="12" r="3" />
        </svg>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: var(--sidebar-width);
  background: var(--bg-primary);
  border-left: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  justify-content: space-between;
  padding: 8px 0;
}

.sidebar-top,
.sidebar-bottom {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.sidebar-btn {
  width: 38px;
  height: 38px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  border-radius: var(--radius-md);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.sidebar-btn:hover {
  background: var(--bg-hover);
  color: #fff;
}

.sidebar-btn.disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.sidebar-btn.disabled:hover {
  background: transparent;
  color: var(--text-secondary);
}
</style>
