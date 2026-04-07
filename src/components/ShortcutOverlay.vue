<script setup lang="ts">
import { ref, computed } from "vue";
import { useShortcuts } from "../composables/useShortcuts";
import { useToast } from "../composables/useToast";
import type { GameAction } from "../types";

defineProps<{
  active: boolean;
  canvasWidth: number;
  canvasHeight: number;
}>();

defineEmits<{
  close: [];
}>();

const { config, addGameAction, removeGameAction } = useShortcuts();
const { success: toastSuccess } = useToast();

const badges = computed(() => config.value.game_actions);

// --- State ---
const showForm = ref(false);
const formX = ref(0);
const formY = ref(0);
const formNormX = ref(0);
const formNormY = ref(0);
const formW = ref(0);
const formH = ref(0);
const formKey = ref("");
const formLabel = ref("");
const editingKey = ref<string | null>(null); // non-null = editing existing
const keyInput = ref<HTMLInputElement | null>(null);

// Drag state
const dragging = ref(false);
const dragStartX = ref(0);
const dragStartY = ref(0);
const dragRect = ref({ x: 0, y: 0, w: 0, h: 0 });
const draggingBadge = ref<string | null>(null); // non-null = moving a badge
let zoneEl: HTMLElement | null = null;

// --- Helpers ---
function getZoneRect() {
  return zoneEl?.getBoundingClientRect() ?? { left: 0, top: 0, width: 1, height: 1 };
}

function openForm(px: number, py: number, nx: number, ny: number, w: number, h: number, existing?: GameAction) {
  formX.value = px;
  formY.value = py;
  formNormX.value = nx;
  formNormY.value = ny;
  formW.value = w;
  formH.value = h;
  formKey.value = existing?.key ?? "";
  formLabel.value = existing?.label ?? "";
  editingKey.value = existing?.key ?? null;
  showForm.value = true;
  setTimeout(() => keyInput.value?.focus(), 50);
}

function closeForm() {
  showForm.value = false;
  editingKey.value = null;
}

// --- Mouse handlers ---
function handleMouseDown(e: MouseEvent) {
  if (showForm.value) return;
  zoneEl = e.currentTarget as HTMLElement;
  const rect = getZoneRect();
  dragStartX.value = e.clientX - rect.left;
  dragStartY.value = e.clientY - rect.top;
  dragging.value = true;
  draggingBadge.value = null;
  dragRect.value = { x: dragStartX.value, y: dragStartY.value, w: 0, h: 0 };
}

function handleBadgeMouseDown(e: MouseEvent, action: GameAction) {
  e.stopPropagation();
  zoneEl = (e.currentTarget as HTMLElement).parentElement!;
  const rect = getZoneRect();
  dragStartX.value = e.clientX - rect.left;
  dragStartY.value = e.clientY - rect.top;
  dragging.value = true;
  draggingBadge.value = action.key;
  dragRect.value = { x: dragStartX.value, y: dragStartY.value, w: 0, h: 0 };
}

function handleMouseMove(e: MouseEvent) {
  if (!dragging.value || !zoneEl) return;
  const rect = getZoneRect();
  const curX = e.clientX - rect.left;
  const curY = e.clientY - rect.top;

  if (draggingBadge.value) {
    // Moving a badge — ghost follows cursor
    const action = config.value.game_actions.find((a) => a.key === draggingBadge.value);
    const bw = action ? (action.w ?? 0.03) * rect.width : 32;
    const bh = action ? (action.h ?? 0.03) * rect.height : 32;
    dragRect.value = { x: curX - bw / 2, y: curY - bh / 2, w: bw, h: bh };
  } else {
    // Drawing a new zone
    dragRect.value = {
      x: Math.min(dragStartX.value, curX),
      y: Math.min(dragStartY.value, curY),
      w: Math.abs(curX - dragStartX.value),
      h: Math.abs(curY - dragStartY.value),
    };
  }
}

function handleMouseUp(e: MouseEvent) {
  if (!dragging.value || !zoneEl) return;
  dragging.value = false;

  const rect = getZoneRect();
  const totalW = rect.width;
  const totalH = rect.height;
  const curX = e.clientX - rect.left;
  const curY = e.clientY - rect.top;
  const pixW = Math.abs(curX - dragStartX.value);
  const pixH = Math.abs(curY - dragStartY.value);
  const minDrag = 8;

  if (draggingBadge.value) {
    // Dropped a badge at new position
    const existing = config.value.game_actions.find((a) => a.key === draggingBadge.value);
    if (existing) {
      if (pixW > minDrag || pixH > minDrag) {
        // Actually moved — update position
        const nx = curX / totalW;
        const ny = curY / totalH;
        addGameAction({ ...existing, x: nx, y: ny });
        toastSuccess(`"${existing.label}" déplacé`);
      } else {
        // Just a click on badge — open edit form
        const px = (existing.x) * totalW;
        const py = (existing.y + (existing.h ?? 0.03) / 2) * totalH + 8;
        openForm(px, py, existing.x, existing.y, existing.w, existing.h, existing);
      }
    }
    draggingBadge.value = null;
    dragRect.value = { x: 0, y: 0, w: 0, h: 0 };
    return;
  }

  // New zone/click on empty area
  let nx: number, ny: number, nw: number, nh: number;
  if (pixW < minDrag && pixH < minDrag) {
    nx = dragStartX.value / totalW;
    ny = dragStartY.value / totalH;
    nw = 0.01;
    nh = 0.01;
  } else {
    const x1 = Math.min(dragStartX.value, curX) / totalW;
    const y1 = Math.min(dragStartY.value, curY) / totalH;
    const x2 = Math.max(dragStartX.value, curX) / totalW;
    const y2 = Math.max(dragStartY.value, curY) / totalH;
    nx = (x1 + x2) / 2;
    ny = (y1 + y2) / 2;
    nw = x2 - x1;
    nh = y2 - y1;
  }

  const px = dragRect.value.x + dragRect.value.w / 2;
  const py = dragRect.value.y + dragRect.value.h + 10;
  openForm(px, py, nx, ny, nw, nh);
  dragRect.value = { x: 0, y: 0, w: 0, h: 0 };
}

// --- Key capture + save ---
function handleKeyCapture(e: KeyboardEvent) {
  if (e.key === "Escape") { closeForm(); return; }
  if (e.key === "Enter" || e.key === "Tab") return;
  e.preventDefault();
  formKey.value = e.key.length === 1 ? e.key.toUpperCase() : e.key;
}

function saveAction() {
  if (!formKey.value) return;
  // If editing and key changed, remove old one
  if (editingKey.value && editingKey.value !== formKey.value) {
    removeGameAction(editingKey.value);
  }
  const label = formLabel.value.trim() || formKey.value;
  addGameAction({
    key: formKey.value,
    label,
    x: formNormX.value,
    y: formNormY.value,
    w: formW.value,
    h: formH.value,
  });
  const verb = editingKey.value ? "modifié" : "ajouté";
  closeForm();
  toastSuccess(`Raccourci "${label}" ${verb}`);
}

function deleteAction() {
  if (!editingKey.value) return;
  removeGameAction(editingKey.value);
  closeForm();
  toastSuccess("Raccourci supprimé");
}
</script>

<template>
  <div v-if="active" class="overlay" @click.self="$emit('close')">
    <div
      class="capture-zone"
      @mousedown="handleMouseDown"
      @mousemove="handleMouseMove"
      @mouseup="handleMouseUp"
    >
      <!-- Drag preview: new zone -->
      <div
        v-if="dragging && dragRect.w > 4 && !draggingBadge"
        class="drag-preview"
        :style="{ left: `${dragRect.x}px`, top: `${dragRect.y}px`, width: `${dragRect.w}px`, height: `${dragRect.h}px` }"
      />

      <!-- Drag preview: badge ghost following cursor -->
      <div
        v-if="dragging && draggingBadge"
        class="badge-ghost"
        :style="{ left: `${dragRect.x}px`, top: `${dragRect.y}px`, width: `${dragRect.w}px`, height: `${dragRect.h}px` }"
      >
        <span class="badge-key">{{ badges.find(b => b.key === draggingBadge)?.key }}</span>
      </div>

      <!-- Badges -->
      <div
        v-for="action in badges"
        :key="action.key"
        class="badge"
        :class="{ dragging: draggingBadge === action.key }"
        :style="{
          left: `${(action.x - (action.w ?? 0.03) / 2) * 100}%`,
          top: `${(action.y - (action.h ?? 0.03) / 2) * 100}%`,
          width: `${(action.w ?? 0.03) * 100}%`,
          height: `${(action.h ?? 0.03) * 100}%`,
        }"
        @mousedown="handleBadgeMouseDown($event, action)"
      >
        <span class="badge-key">{{ action.key }}</span>
        <span class="badge-label">{{ action.label }}</span>
      </div>

      <!-- Form popup (create or edit) -->
      <div
        v-if="showForm"
        class="form-popup"
        :style="{ left: `${formX}px`, top: `${formY + 20}px` }"
        @click.stop
        @mousedown.stop
      >
        <div class="form-row">
          <input
            ref="keyInput"
            :value="formKey"
            class="input key-input"
            :placeholder="editingKey ? editingKey : 'Touche'"
            readonly
            @keydown="handleKeyCapture"
          />
          <input
            v-model="formLabel"
            class="input label-input"
            placeholder="Label"
            @keyup.enter="saveAction"
          />
        </div>
        <div class="form-row">
          <button v-if="editingKey" class="btn delete" @click="deleteAction">Supprimer</button>
          <button class="btn cancel" @click="closeForm">Annuler</button>
          <button class="btn save" :disabled="!formKey" @click="saveAction">
            {{ editingKey ? 'Modifier' : 'OK' }}
          </button>
        </div>
      </div>
    </div>

    <div class="banner">
      <span>Mode raccourcis — Cliquez, dessinez ou déplacez</span>
      <button class="banner-close" @click="$emit('close')">Fermer</button>
    </div>
  </div>
</template>

<style scoped>
.overlay { position: absolute; inset: 0; z-index: 50; pointer-events: auto; }
.capture-zone { position: absolute; inset: 0; cursor: crosshair; }

.banner {
  position: absolute; top: 0; left: 0; right: 0;
  display: flex; align-items: center; justify-content: center; gap: 16px;
  padding: 8px 16px; background: var(--accent); color: #fff;
  font-size: 0.82rem; font-weight: 500; z-index: 60;
}
.banner-close {
  background: rgba(255,255,255,0.2); border: none; color: #fff;
  padding: 4px 12px; border-radius: var(--radius-sm); cursor: pointer; font-size: 0.8rem;
}
.banner-close:hover { background: rgba(255,255,255,0.3); }

.drag-preview {
  position: absolute; background: rgba(83,52,131,0.25);
  border: 2px dashed var(--accent); border-radius: var(--radius-sm);
  pointer-events: none; z-index: 56;
}

.badge-ghost {
  position: absolute;
  background: rgba(83,52,131,0.6);
  border: 2px dashed #fff;
  border-radius: var(--radius-sm);
  display: flex; align-items: center; justify-content: center;
  pointer-events: none;
  z-index: 57;
  min-width: 28px; min-height: 28px;
  padding: 4px;
}

.badge {
  position: absolute; min-width: 28px; min-height: 28px;
  background: rgba(83,52,131,0.4); border: 2px solid rgba(255,255,255,0.5);
  border-radius: var(--radius-sm);
  display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 1px;
  cursor: grab; transition: all 0.15s; z-index: 55;
}
.badge:hover { background: rgba(83,52,131,0.6); border-color: #fff; }
.badge.dragging { opacity: 0.4; }
.badge:active { cursor: grabbing; }

.badge-key { font-size: 0.7rem; font-weight: 700; color: #fff; font-family: monospace; }
.badge-label { font-size: 0.55rem; color: rgba(255,255,255,0.7); max-width: 100%; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

.form-popup {
  position: absolute; transform: translateX(-50%);
  background: var(--bg-secondary); border: 1px solid var(--border);
  border-radius: var(--radius-md); padding: 10px;
  display: flex; flex-direction: column; gap: 6px;
  z-index: 60; box-shadow: 0 8px 24px rgba(0,0,0,0.5); min-width: 240px;
}
.form-row { display: flex; gap: 6px; }

.input {
  padding: 6px 8px; background: var(--bg-primary); border: 1px solid var(--border);
  border-radius: var(--radius-sm); color: var(--text-primary); font-size: 0.8rem;
}
.input:focus { outline: none; border-color: var(--accent); }
.key-input { width: 56px; text-align: center; font-family: monospace; font-weight: 700; font-size: 0.9rem; cursor: default; }
.label-input { flex: 1; }

.btn {
  padding: 6px 10px; border: none; border-radius: var(--radius-sm);
  font-size: 0.78rem; cursor: pointer; flex: 1; text-align: center;
}
.btn.cancel { background: var(--border); color: var(--text-secondary); }
.btn.cancel:hover { background: var(--bg-hover); }
.btn.save { background: var(--accent); color: #fff; }
.btn.save:hover { background: var(--accent-hover); }
.btn.delete { background: var(--danger); color: #fff; }
.btn.delete:hover { background: var(--danger-hover); }
.btn:disabled { opacity: 0.4; cursor: not-allowed; }
</style>
