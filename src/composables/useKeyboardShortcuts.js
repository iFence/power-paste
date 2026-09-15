import { onMounted, onUnmounted, watch } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { normalizeShortcutKey, normalizeShortcutValue } from "../utils/shortcut";
import {
  QUICK_PASTE_HOLD_TIMEOUT_MS,
  QUICK_PASTE_RELEASE_FALLBACK_MS,
} from "../utils/constants";

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
  isHomeRoute,
  clearEditing,
  quickPasteActive,
  commitQuickPaste,
  cancelQuickPaste,
}) {
  // 快速粘贴（门户托管快捷键）期间是否收到过键盘事件：用来判断按键是否真的
  // 会进入面板。桌面把按键完全吞掉时，只能靠延迟兜底结束快速粘贴。
  let quickPasteSawKeyEvent = false;
  // 桌面报告快捷键失活后的兜底提交定时器。
  let quickPasteReleaseTimer = null;

  function clearQuickPasteReleaseTimer() {
    if (quickPasteReleaseTimer === null) {
      return;
    }

    window.clearTimeout(quickPasteReleaseTimer);
    quickPasteReleaseTimer = null;
  }

  function scheduleQuickPasteCommit(delay) {
    clearQuickPasteReleaseTimer();
    quickPasteReleaseTimer = window.setTimeout(() => {
      quickPasteReleaseTimer = null;
      if (!quickPasteActive?.value) {
        return;
      }

      void commitQuickPaste?.();
    }, delay);
  }

  // 门户托管快捷键（Linux Wayland）时桌面只通知“快捷键已失活”，不说明 Ctrl 之类的
  // 修饰键是否还按着。面板能收到键盘事件时，就等所有修饰键松开再提交（与 Windows /
  // macOS 行为一致）；收不到时退回短延迟提交，避免面板一直挂着。
  function handleQuickPasteReleased() {
    if (!quickPasteActive?.value) {
      return;
    }

    scheduleQuickPasteCommit(
      quickPasteSawKeyEvent
        ? QUICK_PASTE_HOLD_TIMEOUT_MS
        : QUICK_PASTE_RELEASE_FALLBACK_MS,
    );
  }

  // 快速粘贴快捷键是否带修饰键：带修饰键时要等修饰键松开才算结束。门户托管
  // （Linux Wayland）时面板收到的按键事件可能缺少修饰键状态，只看 ctrlKey 这类
  // 标志会把“松开主键”误判成“松手”，因此以修饰键自身的 keyup 作为结束信号。
  function quickPasteUsesModifier() {
    return /(^|\+)(Ctrl|Control|Alt|Shift|Command|Cmd|Meta|Super|Win)(\+|$)/i.test(
      settings.quickPasteShortcut ?? "",
    );
  }

  // 事件里的修饰键是否都已松开：不含修饰键的快捷键用它判断“松手了”。
  function noModifierHeld(event) {
    return (
      !event.ctrlKey && !event.metaKey && !event.altKey && !event.shiftKey
    );
  }

  // 松开的是否是修饰键本身：带修饰键的快捷键以它作为结束信号，避开失真的标志。
  function isModifierKeyRelease(event) {
    return [
      "Control",
      "Alt",
      "AltGraph",
      "Shift",
      "Meta",
      "OS",
      "Super",
    ].includes(event.key);
  }

  watch(
    () => quickPasteActive?.value === true,
    (active) => {
      if (active) {
        // 新一轮快速粘贴：重新观察按键是否会进入面板。
        quickPasteSawKeyEvent = false;
        return;
      }

      clearQuickPasteReleaseTimer();
    },
  );

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
      quickPasteSawKeyEvent = true;

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
      const searchInput = document.getElementById("history-search");
      if (searchInput === document.activeElement) {
        searchInput.blur();
      } else {
        searchInput?.focus();
      }
      return;
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

    quickPasteSawKeyEvent = true;

    // 带修饰键的快捷键只认修饰键松开（门户托管时主键松开会带着失真的标志，
    // 容易误判成“松手”）；不含修饰键的快捷键仍是松开主键即结束。
    const shouldCommit = quickPasteUsesModifier()
      ? isModifierKeyRelease(event)
      : noModifierHeld(event);

    if (!shouldCommit) {
      return;
    }

    event.preventDefault();
    event.stopPropagation();
    clearQuickPasteReleaseTimer();
    void commitQuickPaste?.();
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
    clearQuickPasteReleaseTimer();
  });

  return {
    handleWindowAction,
    handleQuickPasteReleased,
    cancelQuickPasteRelease: clearQuickPasteReleaseTimer,
  };
}
