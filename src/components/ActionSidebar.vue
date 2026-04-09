<script setup lang="ts">
import { useLicense } from "../composables/useLicense";

const { isPro } = useLicense();

defineProps<{
  activePanel: string | null;
}>();

const emit = defineEmits<{
  toggleShortcuts: [];
  openPanel: [panel: string];
}>();

function toggle(panel: string) {
  emit("openPanel", panel);
}
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-spacer" />
    <div class="sidebar-icons">
      <!-- Shortcuts -->
      <button
        class="sidebar-btn"
        :class="{ disabled: !isPro }"
        :title="isPro ? 'Raccourcis clavier' : 'Raccourcis (Pro)'"
        @click="isPro && $emit('toggleShortcuts')"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
          <rect x="5" y="4" width="14" height="16" rx="2" />
          <text x="12" y="14.5" text-anchor="middle" font-size="9" font-weight="700" fill="currentColor" stroke="none">A</text>
        </svg>
      </button>

      <!-- Devices -->
      <button
        class="sidebar-btn"
        :class="{ active: activePanel === 'devices' }"
        title="Devices & Clones"
        @click="toggle('devices')"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <rect x="5" y="2" width="14" height="20" rx="2" ry="2" />
          <line x1="12" y1="18" x2="12" y2="18.01" />
        </svg>
      </button>

      <!-- Performance -->
      <button
        class="sidebar-btn"
        :class="{ active: activePanel === 'performance' }"
        title="Performance"
        @click="toggle('performance')"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <line x1="4" y1="21" x2="4" y2="14" />
          <line x1="4" y1="10" x2="4" y2="3" />
          <line x1="12" y1="21" x2="12" y2="12" />
          <line x1="12" y1="8" x2="12" y2="3" />
          <line x1="20" y1="21" x2="20" y2="16" />
          <line x1="20" y1="12" x2="20" y2="3" />
          <line x1="1" y1="14" x2="7" y2="14" />
          <line x1="9" y1="8" x2="15" y2="8" />
          <line x1="17" y1="16" x2="23" y2="16" />
        </svg>
      </button>

      <!-- Account -->
      <button
        class="sidebar-btn"
        :class="{ active: activePanel === 'account' }"
        title="Compte & Licence"
        @click="toggle('account')"
      >
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
          <circle cx="12" cy="7" r="4" />
        </svg>
      </button>

      <!-- Settings -->
      <button
        class="sidebar-btn"
        :class="{ active: activePanel === 'settings' }"
        title="Paramètres"
        @click="toggle('settings')"
      >
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
  padding: 8px 0;
}

.sidebar-spacer {
  flex: 1;
}

.sidebar-icons {
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

.sidebar-btn.active {
  background: var(--accent);
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
