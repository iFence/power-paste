import { computed, nextTick, ref, watch } from "vue";
import {
  clearHistory as clearHistoryRequest,
  copyItem as copyItemRequest,
  deleteItem,
  getHistory,
  pasteItem as pasteItemRequest,
  toggleFavorite as toggleFavoriteRequest,
  togglePin as togglePinRequest,
  updateTextItem,
} from "../services/tauriApi";

function formatActionError(error, t) {
  const message =
    typeof error === "string"
      ? error
      : error && typeof error === "object" && typeof error.message === "string"
        ? error.message
        : "";

  if (message === "unsupported_clipboard_write") {
    return t("unsupportedClipboardWrite");
  }
  if (message === "unsupported_direct_paste") {
    return t("unsupportedDirectPaste");
  }
  if (message === "unsupported_launch_on_startup") {
    return t("unsupportedLaunchOnStartup");
  }

  return message || t("unsupportedCurrentPlatform");
}

function compareHistoryItems(left, right) {
  if (left.pinned !== right.pinned) {
    return Number(right.pinned) - Number(left.pinned);
  }

  const pinnedAtCompare = (right.pinnedAt ?? "").localeCompare(left.pinnedAt ?? "");
  if (pinnedAtCompare !== 0) {
    return pinnedAtCompare;
  }

  if (left.favorite !== right.favorite) {
    return Number(right.favorite) - Number(left.favorite);
  }

  return (right.createdAt ?? "").localeCompare(left.createdAt ?? "");
}

export function useHistory({ platformCapabilities, settings, t }) {
  const query = ref("");
  const activeFilterTab = ref("all");
  const history = ref([]);
  const loading = ref(true);
  const selectedId = ref(null);
  const historyPanelRef = ref(null);
  const showEditModal = ref(false);
  const editingItemId = ref(null);
  const editDraft = ref("");
  const actionFeedback = ref("");

  const filteredHistory = computed(() =>
    history.value.filter((item) => {
      if (activeFilterTab.value === "mixed" && item.kind !== "mixed") {
        return false;
      }
      if (activeFilterTab.value === "text" && item.kind !== "text") {
        return false;
      }
      if (activeFilterTab.value === "image" && item.kind !== "image") {
        return false;
      }
      if (activeFilterTab.value === "pinned" && !item.pinned) {
        return false;
      }

      const lower = query.value.trim().toLowerCase();
      if (!lower) {
        return true;
      }

      const haystack = `${item.preview}\n${item.fullText ?? ""}\n${item.sourceApp ?? ""}`.toLowerCase();
      return haystack.includes(lower);
    }),
  );

  const historyTabs = computed(() => [
    { key: "all", label: t("filterAll") },
    { key: "pinned", label: t("filterPinned") },
    { key: "text", label: t("filterText") },
    { key: "image", label: t("filterImage") },
    { key: "mixed", label: t("filterMixed") },
  ]);

  const historyCountLabel = computed(() => {
    const count = filteredHistory.value.length;
    return t("itemCount", {
      count,
      shortcut: settings.globalShortcut || "--",
    });
  });

  function reorderHistory(nextHistory = history.value) {
    history.value = [...nextHistory].sort(compareHistoryItems);
  }

  function trimHistoryToLimit() {
    const limit = Number(settings.maxHistoryItems) || 0;
    if (limit <= 0) {
      return;
    }

    const next = [...history.value];
    while (next.length > limit) {
      const removableIndex = [...next]
        .reverse()
        .findIndex((item) => !item.pinned);

      if (removableIndex === -1) {
        break;
      }

      next.splice(next.length - 1 - removableIndex, 1);
    }

    history.value = next;
  }

  function updateSelectedAfterListChange(removedId = null) {
    const items = filteredHistory.value;
    if (!items.length) {
      selectedId.value = null;
      return;
    }

    if (removedId && selectedId.value === removedId) {
      selectedId.value = items[0]?.id ?? null;
      return;
    }

    if (!items.some((item) => item.id === selectedId.value)) {
      selectedId.value = items[0]?.id ?? null;
    }
  }

  async function refreshHistory() {
    loading.value = true;
    try {
      const items = await getHistory({
        query: query.value.trim() || null,
        limit: settings.maxHistoryItems,
      });
      reorderHistory(items);
      if (!selectedId.value || !items.some((item) => item.id === selectedId.value)) {
        selectedId.value = items[0]?.id ?? null;
      }
    } finally {
      loading.value = false;
    }
  }

  function applyHistoryUpdate(item) {
    if (!item || !item.id) {
      return;
    }

    const index = history.value.findIndex((entry) => entry.id === item.id);
    if (index === -1) {
      history.value = [item, ...history.value];
    } else {
      history.value[index] = {
        ...history.value[index],
        ...item,
      };
      history.value = [...history.value];
    }

    reorderHistory();
    trimHistoryToLimit();
    updateSelectedAfterListChange();
  }

  async function copyItem(id) {
    try {
      actionFeedback.value = "";
      await copyItemRequest(id);
      actionFeedback.value = t("statusCopied");
    } catch (error) {
      actionFeedback.value = formatActionError(error, t);
      throw error;
    }
  }

  async function pasteItem(id) {
    if (!platformCapabilities.value.supportsDirectPaste) {
      actionFeedback.value = t("unsupportedDirectPaste");
      return;
    }

    try {
      actionFeedback.value = "";
      await pasteItemRequest(id);
    } catch (error) {
      actionFeedback.value = formatActionError(error, t);
      throw error;
    }
  }

  async function togglePin(id) {
    const index = history.value.findIndex((item) => item.id === id);
    if (index === -1) {
      await togglePinRequest(id);
      await refreshHistory();
      return;
    }

    const current = history.value[index];
    const nextPinned = !current.pinned;
    const previous = {
      pinned: current.pinned,
      pinnedAt: current.pinnedAt,
    };

    history.value[index] = {
      ...current,
      pinned: nextPinned,
      pinnedAt: nextPinned ? new Date().toISOString() : null,
    };
    reorderHistory();
    updateSelectedAfterListChange();

    try {
      await togglePinRequest(id);
    } catch (error) {
      const rollbackIndex = history.value.findIndex((item) => item.id === id);
      if (rollbackIndex !== -1) {
        history.value[rollbackIndex] = {
          ...history.value[rollbackIndex],
          ...previous,
        };
        reorderHistory();
        updateSelectedAfterListChange();
      } else {
        await refreshHistory();
      }
      throw error;
    }
  }

  async function toggleFavorite(id) {
    await toggleFavoriteRequest(id);
    await refreshHistory();
  }

  async function removeItem(id) {
    const index = history.value.findIndex((item) => item.id === id);
    if (index === -1) {
      await deleteItem(id);
      await refreshHistory();
      return;
    }

    const [removedItem] = history.value.splice(index, 1);
    history.value = [...history.value];
    updateSelectedAfterListChange(id);

    try {
      await deleteItem(id);
    } catch (error) {
      history.value.splice(index, 0, removedItem);
      history.value = [...history.value];
      reorderHistory();
      updateSelectedAfterListChange();
      throw error;
    }
  }

  function openEditModal(item) {
    if (item.kind !== "text") {
      return;
    }

    editingItemId.value = item.id;
    editDraft.value = item.fullText ?? "";
    showEditModal.value = true;
  }

  async function saveEditedItem() {
    if (!editingItemId.value) {
      return;
    }

    await updateTextItem(editingItemId.value, editDraft.value);
    showEditModal.value = false;
    editingItemId.value = null;
    await refreshHistory();
  }

  async function clearHistory() {
    await clearHistoryRequest();
    await refreshHistory();
  }

  async function scrollSelectedIntoView() {
    await nextTick();

    const panel = historyPanelRef.value;
    if (!panel || !selectedId.value) {
      return;
    }

    const activeItem = panel.querySelector(`[data-history-id="${selectedId.value}"]`);
    if (!(activeItem instanceof HTMLElement)) {
      return;
    }

    const panelTop = panel.scrollTop;
    const panelBottom = panelTop + panel.clientHeight;
    const itemTop = activeItem.offsetTop;
    const itemBottom = itemTop + activeItem.offsetHeight;
    const margin = 12;

    if (itemTop - margin < panelTop) {
      panel.scrollTo({ top: Math.max(0, itemTop - margin), behavior: "smooth" });
      return;
    }

    if (itemBottom + margin > panelBottom) {
      panel.scrollTo({
        top: itemBottom - panel.clientHeight + margin,
        behavior: "smooth",
      });
    }
  }

  watch(selectedId, () => {
    void scrollSelectedIntoView();
  });

  watch(filteredHistory, (items) => {
    if (!items.some((item) => item.id === selectedId.value)) {
      selectedId.value = items[0]?.id ?? null;
    }
  });

  return {
    activeFilterTab,
    clearHistory,
    copyItem,
    editDraft,
    editingItemId,
    filteredHistory,
    history,
    historyCountLabel,
    historyPanelRef,
    historyTabs,
    loading,
    openEditModal,
    pasteItem,
    query,
    refreshHistory,
    applyHistoryUpdate,
    removeItem,
    saveEditedItem,
    selectedId,
    actionFeedback,
    setSelectedId: (id) => {
      selectedId.value = id;
    },
    showEditModal,
    toggleFavorite,
    togglePin,
  };
}
