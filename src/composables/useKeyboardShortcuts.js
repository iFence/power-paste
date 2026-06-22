import { onMounted, onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { normalizeShortcutKey, normalizeShortcutValue } from "../utils/shortcut";

export function useKeyboardShortcuts({
  closeSelect,
  copyItem,
  activeFilterTab,
  filteredHistory,
  historyTabs,
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

  function eventShortcutValue(event, metaLabel = "Command") {
    const parts = [];
    if (event.ctrlKey) {
      parts.push("Ctrl");
    }
    if (event.altKey) {
      parts.push("Alt");
    }
    if (event.shiftKey) {
      parts.push("Shift");
    }
    if (event.metaKey) {
      parts.push(metaLabel);
    }

    const mainKey =
      event.code === "Backquote" ? "`" : normalizeShortcutKey(event.key);
    if (!mainKey || ["Ctrl", "Alt", "Shift", "Command", "Super"].includes(mainKey)) {
      return "";
    }

    return normalizeShortcutValue([...parts, mainKey].join("+"));
  }

  function matchesShortcut(event, shortcut) {
    return (
      eventShortcutValue(event, "Command") === shortcut ||
      eventShortcutValue(event, "Super") === shortcut
    );
  }

  function shortcutWithShift(shortcut) {
    const parts = shortcut.split("+").filter(Boolean);
    if (parts.length < 2 || parts.includes("Shift")) {
      return "";
    }

    const mainKey = parts.at(-1);
    return normalizeShortcutValue([...parts.slice(0, -1), "Shift", mainKey].join("+"));
  }

  function isSearchShortcut(event) {
    const shortcut = normalizeShortcutValue(settings.searchShortcut ?? "Ctrl+F");
    if (!shortcut) {
      return false;
    }

    if (shortcut === "Ctrl+F") {
      return (
        (event.ctrlKey || event.metaKey) &&
        !event.altKey &&
        !event.shiftKey &&
        event.key.toLowerCase() === "f"
      );
    }

    return matchesShortcut(event, shortcut);
  }

  function filterShortcutDirection(event) {
    const shortcut = normalizeShortcutValue(settings.filterShortcut ?? "Ctrl+Tab");
    if (!shortcut) {
      return 0;
    }

    if (matchesShortcut(event, shortcut)) {
      return 1;
    }

    const reverseShortcut = shortcutWithShift(shortcut);
    if (reverseShortcut && matchesShortcut(event, reverseShortcut)) {
      return -1;
    }

    return 0;
  }

  function cycleFilterTab(direction = 1) {
    const tabs = historyTabs?.value || [];
    if (!tabs.length || !activeFilterTab) {
      return;
    }

    const currentIndex = tabs.findIndex((tab) => tab.key === activeFilterTab.value);
    const safeIndex = currentIndex === -1 ? 0 : currentIndex;
    const nextIndex = (safeIndex + direction + tabs.length) % tabs.length;
    activeFilterTab.value = tabs[nextIndex].key;
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

    const filterDirection = filterShortcutDirection(event);
    if (filterDirection && isHomeRoute?.value && !showEditModal.value) {
      event.preventDefault();
      event.stopPropagation();
      cycleFilterTab(filterDirection);
      return;
    }

    if (isSearchShortcut(event)) {
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
