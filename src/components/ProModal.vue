<script setup lang="ts">
import { open } from "@tauri-apps/plugin-shell";

defineProps<{
  visible: boolean;
  feature: string;
}>();

const emit = defineEmits<{
  close: [];
}>();

const checkoutUrl =
  "https://dokky.lemonsqueezy.com/checkout/buy/6be03ef3-0abd-4406-a4be-6b8920549a62";

async function openCheckout() {
  await open(checkoutUrl);
  emit("close");
}
</script>

<template>
  <div v-if="visible" class="overlay" @click.self="$emit('close')">
    <div class="modal">
      <div class="modal-icon">
        <svg
          width="32"
          height="32"
          viewBox="0 0 24 24"
          fill="none"
          stroke="var(--accent)"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z" />
        </svg>
      </div>
      <h3>Fonctionnalité Pro</h3>
      <p class="desc">
        <strong>{{ feature }}</strong> est disponible avec la licence Pro.
      </p>
      <ul class="benefits">
        <li>Multi-device (plusieurs téléphones)</li>
        <li>Raccourcis clavier personnalisables</li>
      </ul>
      <p class="price">À partir de <strong>1,99€/mois</strong></p>
      <div class="actions">
        <button class="btn cancel" @click="$emit('close')">Plus tard</button>
        <button class="btn primary" @click="openCheckout">
          Passer à Pro
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.modal {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 24px;
  max-width: 360px;
  text-align: center;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
}

.modal-icon {
  margin-bottom: 12px;
}

h3 {
  margin: 0 0 8px;
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--text-primary);
}

.desc {
  font-size: 0.85rem;
  color: var(--text-secondary);
  margin-bottom: 16px;
  line-height: 1.4;
}

.benefits {
  text-align: left;
  list-style: none;
  padding: 0;
  margin: 0 0 16px;
}

.benefits li {
  font-size: 0.82rem;
  color: var(--text-secondary);
  padding: 4px 0;
}

.benefits li::before {
  content: "✓ ";
  color: var(--accent);
  font-weight: 700;
}

.price {
  font-size: 0.9rem;
  color: var(--text-primary);
  margin-bottom: 20px;
}

.actions {
  display: flex;
  gap: 8px;
  justify-content: center;
}

.btn {
  padding: 8px 20px;
  border-radius: var(--radius-md);
  border: none;
  cursor: pointer;
  font-size: 0.85rem;
  font-weight: 500;
  text-decoration: none;
  display: inline-flex;
  align-items: center;
}

.btn.cancel {
  background: var(--border);
  color: var(--text-secondary);
}

.btn.cancel:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.btn.primary {
  background: var(--accent);
  color: #fff;
}

.btn.primary:hover {
  background: var(--accent-hover);
}
</style>
