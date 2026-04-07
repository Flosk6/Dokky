import { onMounted, onUnmounted } from "vue";

interface ShortcutActions {
  newSession: () => void;
  closeActive: () => void;
  switchByIndex: (index: number) => void;
  switchNext: () => void;
  switchPrev: () => void;
}

export function useShortcuts(actions: ShortcutActions) {
  function handleKeydown(e: KeyboardEvent) {
    const ctrl = e.ctrlKey || e.metaKey;

    if (ctrl && e.key === "t") {
      e.preventDefault();
      actions.newSession();
      return;
    }

    if (ctrl && e.key === "w") {
      e.preventDefault();
      actions.closeActive();
      return;
    }

    // Ctrl+1..9 → switch to session by index
    if (ctrl && e.key >= "1" && e.key <= "9") {
      e.preventDefault();
      actions.switchByIndex(parseInt(e.key) - 1);
      return;
    }

    if (ctrl && e.key === "Tab") {
      e.preventDefault();
      if (e.shiftKey) {
        actions.switchPrev();
      } else {
        actions.switchNext();
      }
      return;
    }
  }

  onMounted(() => {
    window.addEventListener("keydown", handleKeydown);
  });

  onUnmounted(() => {
    window.removeEventListener("keydown", handleKeydown);
  });
}
