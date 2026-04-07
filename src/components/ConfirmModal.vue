<script setup lang="ts">
defineProps<{
  visible: boolean;
  title?: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
}>();

defineEmits<{
  confirm: [];
  cancel: [];
}>();
</script>

<template>
  <div v-if="visible" class="overlay" @click.self="$emit('cancel')">
    <div class="modal">
      <h3 v-if="title">{{ title }}</h3>
      <p>{{ message }}</p>
      <div class="actions">
        <button class="btn-cancel" @click="$emit('cancel')">
          {{ cancelLabel ?? "Annuler" }}
        </button>
        <button
          class="btn-confirm"
          :class="{ danger }"
          @click="$emit('confirm')"
        >
          {{ confirmLabel ?? "Confirmer" }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.modal {
  background: #1a1a2e;
  border: 1px solid #333;
  border-radius: 12px;
  padding: 1.5rem;
  min-width: 320px;
  max-width: 420px;
  box-shadow: 0 12px 40px rgba(0, 0, 0, 0.5);
}

h3 {
  margin: 0 0 0.5rem;
  font-size: 1.1rem;
}

p {
  color: #bbb;
  font-size: 0.9rem;
  margin: 0 0 1.25rem;
  line-height: 1.5;
}

.actions {
  display: flex;
  gap: 0.5rem;
  justify-content: flex-end;
}

.btn-cancel,
.btn-confirm {
  padding: 0.5rem 1rem;
  border-radius: 6px;
  border: none;
  cursor: pointer;
  font-size: 0.85rem;
}

.btn-cancel {
  background: #333;
  color: #aaa;
}

.btn-cancel:hover {
  background: #444;
}

.btn-confirm {
  background: #533483;
  color: #fff;
}

.btn-confirm:hover {
  background: #6441a5;
}

.btn-confirm.danger {
  background: #f44336;
}

.btn-confirm.danger:hover {
  background: #d32f2f;
}
</style>
