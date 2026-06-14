import { onMounted, onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

export function useKeyboardShortcuts({
  closeSelect,
  copyItem,
  filteredHistory,
  openSelectKey,
  pasteItem,
  selectedId,
  setSelectedId,
  settings,
  showEditModal,
  isSettingsRoute,
  isHomeRoute,
  leaveSettings,
  clearEditing,
  quickPasteActive,
  commitQuickPaste,
  cancelQuickPaste,
}) {
  function isEditableTarget(target) {
    return (
      target instanceof HTMLElement &&
      (target.isContentEditable ||
        ["INPUT", "TEXTAREA", "SELECT"].includes(target.tagName))
    );
  }

  async function handleWindowAction(action) {
    const appWindow = getCurrentWindow();

    if (action === "minimize") {
      await appWindow.minimize();
      return;
    }

    if (action === "maximize") {
      if (await appWindow.isMaximized()) {
        await appWindow.unmaximize();
        return;
      }
      await appWindow.maximize();
      return;
    }

    if (action === "close") {
      await appWindow.close();
    }
  }

  function handleKeydown(event) {
    const key = event.key.toLowerCase();
    const withPrimary = event.ctrlKey || event.metaKey;

    if (quickPasteActive?.value) {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        cancelQuickPaste?.();
        return;
      }

    }

    const inspectOrReloadShortcut =
      event.key === "F5" ||
      event.key === "F12" ||
      (withPrimary && key === "r") ||
      (withPrimary && event.shiftKey && ["i", "j", "c"].includes(key)) ||
      (withPrimary && key === "u");

    if (!settings.debugEnabled && inspectOrReloadShortcut) {
      event.preventDefault();
      event.stopPropagation();
      return;
    }

    if (withPrimary && key === "f") {
      event.preventDefault();
      document.getElementById("history-search")?.focus();
    }

    if (withPrimary && /^\d$/.test(event.key) && isHomeRoute?.value && !showEditModal.value) {
      if (isEditableTarget(event.target)) {
        return;
      }

      const items = filteredHistory.value.slice(0, 10);
      const shortcutIndex = event.key === "0" ? 9 : Number(event.key) - 1;
      const item = items[shortcutIndex];
      if (!item) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();
      setSelectedId(item.id);
      void pasteItem(item.id);
      return;
    }

    if (withPrimary && key === "c" && selectedId.value && !showEditModal.value) {
      if (isEditableTarget(event.target)) {
        return;
      }
      event.preventDefault();
      void copyItem(selectedId.value);
      return;
    }

    if (event.key === "Escape") {
      if (openSelectKey.value) {
        closeSelect();
        return;
      }

      if (showEditModal.value) {
        clearEditing();
        return;
      }

      if (isSettingsRoute.value) {
        void leaveSettings();
      }
      return;
    }

    if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      const items = filteredHistory.value;
      if (!items.length) {
        return;
      }

      event.preventDefault();
      const currentIndex = items.findIndex((item) => item.id === selectedId.value);
      const delta = event.key === "ArrowDown" ? 1 : -1;
      const nextIndex =
        currentIndex === -1
          ? 0
          : Math.min(items.length - 1, Math.max(0, currentIndex + delta));
      setSelectedId(items[nextIndex].id);
    }

    if (event.key === "Enter" && selectedId.value && !showEditModal.value) {
      if (isEditableTarget(event.target)) {
        return;
      }
      event.preventDefault();
      void pasteItem(selectedId.value);
    }
  }

  function handleKeyup(event) {
    if (!quickPasteActive?.value) {
      return;
    }

    if (!event.ctrlKey && !event.metaKey) {
      event.preventDefault();
      event.stopPropagation();
      void commitQuickPaste?.();
    }
  }

  function handlePointerDown(event) {
    const target = event.target;
    if (!(target instanceof Element)) {
      return;
    }

    if (!target.closest(".custom-select")) {
      closeSelect();
    }
  }

  onMounted(() => {
    window.addEventListener("keydown", handleKeydown);
    window.addEventListener("keyup", handleKeyup);
    window.addEventListener("pointerdown", handlePointerDown);
  });

  onUnmounted(() => {
    window.removeEventListener("keydown", handleKeydown);
    window.removeEventListener("keyup", handleKeyup);
    window.removeEventListener("pointerdown", handlePointerDown);
  });

  return {
    handleWindowAction,
  };
}
