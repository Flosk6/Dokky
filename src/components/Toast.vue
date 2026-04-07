<script setup lang="ts">
import { useToast } from "../composables/useToast";

const { toasts } = useToast();
</script>

<template>
  <Transition name="toast-group">
    <div v-if="toasts.length" class="toast-container">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          class="toast"
          :class="toast.type"
        >
          {{ toast.message }}
        </div>
      </TransitionGroup>
    </div>
  </Transition>
</template>

<style scoped>
.toast-container {
  position: fixed;
  bottom: 16px;
  right: 60px;
  z-index: 300;
  display: flex;
  flex-direction: column-reverse;
  gap: 8px;
  max-width: 360px;
}

.toast {
  padding: 10px 16px;
  border-radius: var(--radius-md);
  font-size: 0.85rem;
  color: #fff;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  backdrop-filter: blur(8px);
}

.toast.success { background: rgba(87, 242, 135, 0.9); color: #000; }
.toast.error { background: rgba(244, 67, 54, 0.9); }
.toast.info { background: rgba(88, 101, 242, 0.9); }

.toast-enter-active { transition: all 0.3s ease-out; }
.toast-leave-active { transition: all 0.2s ease-in; }
.toast-enter-from { opacity: 0; transform: translateX(40px); }
.toast-leave-to { opacity: 0; transform: translateX(40px); }
</style>
