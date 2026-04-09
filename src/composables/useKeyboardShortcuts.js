import { onMounted, onUnmounted } from "vue";
import { PhysicalPosition, PhysicalSize, currentMonitor, getCurrentWindow } from "@tauri-apps/api/window";

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
  showSettings,
  clearEditing,
}) {
  let zoomedWindowBounds = null;

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
      if (zoomedWindowBounds) {
        await appWindow.setPosition(
          new PhysicalPosition(zoomedWindowBounds.position.x, zoomedWindowBounds.position.y),
        );
        await appWindow.setSize(
          new PhysicalSize(zoomedWindowBounds.size.width, zoomedWindowBounds.size.height),
        );
        zoomedWindowBounds = null;
        return;
      }

      const monitor = await currentMonitor();
      if (!monitor) {
        return;
      }

      const [position, size] = await Promise.all([
        appWindow.outerPosition(),
        appWindow.outerSize(),
      ]);

      zoomedWindowBounds = {
        position,
        size,
      };

      await appWindow.setPosition(
        new PhysicalPosition(monitor.workArea.position.x, monitor.workArea.position.y),
      );
      await appWindow.setSize(
        new PhysicalSize(monitor.workArea.size.width, monitor.workArea.size.height),
      );
      return;
    }

    if (action === "close") {
      await appWindow.close();
    }
  }

  function handleKeydown(event) {
    const key = event.key.toLowerCase();
    const withPrimary = event.ctrlKey || event.metaKey;
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

      if (showSettings.value) {
        showSettings.value = false;
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
      event.preventDefault();
      void pasteItem(selectedId.value);
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
    window.addEventListener("pointerdown", handlePointerDown);
  });

  onUnmounted(() => {
    window.removeEventListener("keydown", handleKeydown);
    window.removeEventListener("pointerdown", handlePointerDown);
  });

  return {
    handleWindowAction,
  };
}
