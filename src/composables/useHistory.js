import { computed, nextTick, ref, watch } from "vue";
import {
  clearHistory as clearHistoryRequest,
  copyItem as copyItemRequest,
  deleteItem,
  getHistory,
  openExternalUrl as openExternalUrlRequest,
  pasteItem as pasteItemRequest,
  toggleFavorite as toggleFavoriteRequest,
  togglePin as togglePinRequest,
  updateItemTags as updateItemTagsRequest,
  updateTextItem,
} from "../services/tauriApi";
import { playCopySoundFallback } from "./useCopySound";
import { useHistoryFilters } from "./useHistoryFilters";
import {
  getLatestHistoryItem,
  useHistorySelection,
} from "./useHistorySelection";

const HISTORY_PAGE_SIZE = 30;

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
  if (message === "linux_x11_tools_missing") {
    return t("linuxX11ToolsMissing");
  }
  if (message === "linux_wayland_tools_missing") {
    return t("linuxWaylandToolsMissing");
  }
  if (message === "unsupported_direct_paste") {
    return t("unsupportedDirectPaste");
  }
  if (message === "unsupported_launch_on_startup") {
    return t("unsupportedLaunchOnStartup");
  }
  if (message === "paste_target_focus_failed") {
    return t("pasteTargetFocusFailed");
  }
  if (message === "paste_target_permission_denied") {
    return t("pasteTargetPermissionDenied");
  }

  return message || t("unsupportedCurrentPlatform");
}

function directPasteUnavailableMessage(platformCapabilities, t) {
  const capabilities = platformCapabilities?.value;
  if (!capabilities || capabilities.supportsDirectPaste) {
    return t("unsupportedDirectPaste");
  }

  if (capabilities.platform === "linux") {
    if (capabilities.directPasteStrategy === "wayland-wtype-required") {
      return t("linuxWaylandToolsMissing");
    }
    if (capabilities.directPasteStrategy === "x11-tooling-required") {
      return t("linuxX11ToolsMissing");
    }
  }

  return t("unsupportedDirectPaste");
}

function compareHistoryItems(left, right, statsMode = "") {
  if (left.pinned !== right.pinned) {
    return Number(right.pinned) - Number(left.pinned);
  }

  if (left.pinned && right.pinned) {
    const pinnedAtCompare = (right.pinnedAt ?? right.createdAt ?? "").localeCompare(
      left.pinnedAt ?? left.createdAt ?? "",
    );
    if (pinnedAtCompare !== 0) {
      return pinnedAtCompare;
    }
  }

  if (statsMode === "paste") {
    const pasteCountCompare =
      (Number(right.pasteCount) || 0) - (Number(left.pasteCount) || 0);
    if (pasteCountCompare !== 0) {
      return pasteCountCompare;
    }
  } else if (statsMode === "copy") {
    const copyCountCompare =
      (Number(right.copyCount) || 0) - (Number(left.copyCount) || 0);
    if (copyCountCompare !== 0) {
      return copyCountCompare;
    }
  }

  if (!statsMode && left.favorite !== right.favorite) {
    return Number(right.favorite) - Number(left.favorite);
  }

  return (right.createdAt ?? "").localeCompare(left.createdAt ?? "");
}

export function useHistory({ platformCapabilities, settings, t }) {
  const history = ref([]);
  const loading = ref(true);
  const relativeTimeVersion = ref(0);
  const showEditModal = ref(false);
  const editingItemId = ref(null);
  const editDraft = ref("");
  const actionFeedback = ref("");
  const loadingMore = ref(false);
  const hasMoreHistory = ref(true);
  const loadedHistoryOffset = ref(0);
  const totalHistoryCount = ref(0);
  const skipNextFilterRefresh = ref(false);
  const {
    activeFilterTab,
    activeTagFilter,
    availableTagFilters,
    buildHistoryQueryPayload,
    historyTabs,
    query,
  } = useHistoryFilters({ history, settings, t });
  const {
    historyPanelRef,
    restoreSelection,
    scrollSelectedIntoView,
    selectedId,
    syncPersistedHistoryState,
    updateSelectedAfterListChange,
  } = useHistorySelection();

  const filteredHistory = computed(() => history.value);

  const historyCountLabel = computed(() => {
    return t("itemCount", {
      count: totalHistoryCount.value,
      shortcut: settings.globalShortcut || "--",
    });
  });

  function sortHistory(nextHistory = history.value) {
    const statsMode = settings.pasteStatsEnabled
      ? "paste"
      : settings.copyStatsEnabled
        ? "copy"
        : "";
    return [...nextHistory].sort((left, right) =>
      compareHistoryItems(left, right, statsMode),
    );
  }

  function replaceHistory(nextHistory) {
    history.value = nextHistory;
  }

  function updateHistoryItemAt(index, updater) {
    if (index < 0 || index >= history.value.length) {
      return null;
    }

    const current = history.value[index];
    const nextItem =
      typeof updater === "function" ? updater(current) : updater;
    history.value[index] = nextItem;
    return nextItem;
  }

  function reorderHistory(nextHistory = history.value) {
    replaceHistory(sortHistory(nextHistory));
  }

  function upsertHistoryItem(nextItem) {
    const next = [...history.value];
    const index = next.findIndex((item) => item.id === nextItem.id);
    if (index !== -1) {
      next.splice(index, 1);
    }
    next.push(nextItem);
    replaceHistory(sortHistory(next));
    return index;
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

  function configuredHistoryLimit() {
    const limit = Number(settings.maxHistoryItems) || 0;
    return limit > 0 ? limit : Number.POSITIVE_INFINITY;
  }

  function nextHistoryPageLimit() {
    const maxLimit = configuredHistoryLimit();
    if (!Number.isFinite(maxLimit)) {
      return HISTORY_PAGE_SIZE;
    }

    return Math.max(
      0,
      Math.min(HISTORY_PAGE_SIZE, maxLimit - loadedHistoryOffset.value),
    );
  }

  function updateHistoryPaginationState(receivedCount, requestedLimit) {
    const maxLimit = configuredHistoryLimit();
    const effectiveTotal = Math.min(totalHistoryCount.value, maxLimit);
    hasMoreHistory.value =
      receivedCount === requestedLimit &&
      loadedHistoryOffset.value < effectiveTotal;
  }

  async function refreshHistory(options = {}) {
    const { detectNewHistory = true } = options;
    loading.value = true;
    loadedHistoryOffset.value = 0;
    hasMoreHistory.value = true;
    try {
      const limit = nextHistoryPageLimit();
      const page = await getHistory(buildHistoryQueryPayload(limit, 0));
      const items = page.items;
      totalHistoryCount.value = page.totalCount;
      loadedHistoryOffset.value = items.length;
      updateHistoryPaginationState(items.length, limit);
      replaceHistory(items);
      const { hasNewHistory } = restoreSelection(items, {
        detectNewHistory,
      });

      if (hasNewHistory) {
        activeFilterTab.value = "all";
      }

      syncPersistedHistoryState(items);
    } finally {
      loading.value = false;
    }
  }

  async function loadMoreHistory() {
    if (loading.value || loadingMore.value || !hasMoreHistory.value) {
      return;
    }

    const limit = nextHistoryPageLimit();
    if (limit <= 0) {
      hasMoreHistory.value = false;
      return;
    }

    loadingMore.value = true;
    try {
      const page = await getHistory(
        buildHistoryQueryPayload(limit, loadedHistoryOffset.value),
      );
      const items = page.items;
      totalHistoryCount.value = page.totalCount;
      loadedHistoryOffset.value += items.length;

      const loadedIds = new Set(history.value.map((item) => item.id));
      const nextItems = items.filter((item) => !loadedIds.has(item.id));
      if (nextItems.length) {
        replaceHistory([...history.value, ...nextItems]);
      }

      updateHistoryPaginationState(items.length, limit);
      updateSelectedAfterListChange(history.value);
      syncPersistedHistoryState(history.value);
    } finally {
      loadingMore.value = false;
    }
  }

  function refreshRelativeTimes() {
    relativeTimeVersion.value += 1;
  }

  function applyHistoryUpdate(item) {
    if (!item || !item.id) {
      return;
    }

    const previousLatestHistoryId =
      getLatestHistoryItem(history.value)?.id ?? null;
    const index = history.value.findIndex((entry) => entry.id === item.id);
    if (index === -1) {
      upsertHistoryItem(item);
      totalHistoryCount.value += 1;
    } else {
      upsertHistoryItem({
        ...history.value[index],
        ...item,
      });
    }

    trimHistoryToLimit();
    const latestHistoryItem = getLatestHistoryItem(history.value);

    if (
      index === -1 &&
      latestHistoryItem?.id &&
      latestHistoryItem.id !== previousLatestHistoryId
    ) {
      activeFilterTab.value = "all";
      selectedId.value = latestHistoryItem.id;
    } else {
      updateSelectedAfterListChange(filteredHistory.value);
    }

    syncPersistedHistoryState(history.value);
  }

  async function copyItem(id) {
    try {
      actionFeedback.value = "";
      if (settings.soundEnabled) {
        playCopySoundFallback();
      }
      await copyItemRequest(id);
      actionFeedback.value = t("statusCopied");
    } catch (error) {
      actionFeedback.value = formatActionError(error, t);
      throw error;
    }
  }

  async function pasteItem(id) {
    if (!platformCapabilities.value.supportsDirectPaste) {
      actionFeedback.value = directPasteUnavailableMessage(platformCapabilities, t);
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

  async function openExternalUrl(url) {
    await openExternalUrlRequest(url);
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

    updateHistoryItemAt(index, {
      ...current,
      pinned: nextPinned,
      pinnedAt: nextPinned ? new Date().toISOString() : null,
    });
    reorderHistory();
    updateSelectedAfterListChange(filteredHistory.value);
    syncPersistedHistoryState(history.value);

    try {
      await togglePinRequest(id);
    } catch (error) {
      const rollbackIndex = history.value.findIndex((item) => item.id === id);
      if (rollbackIndex !== -1) {
        updateHistoryItemAt(rollbackIndex, {
          ...history.value[rollbackIndex],
          ...previous,
        });
        reorderHistory();
        updateSelectedAfterListChange(filteredHistory.value);
        syncPersistedHistoryState(history.value);
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
    totalHistoryCount.value = Math.max(0, totalHistoryCount.value - 1);
    updateSelectedAfterListChange(filteredHistory.value, id);
    syncPersistedHistoryState(history.value);

    try {
      await deleteItem(id);
    } catch (error) {
      history.value.splice(index, 0, removedItem);
      totalHistoryCount.value += 1;
      reorderHistory();
      updateSelectedAfterListChange(filteredHistory.value);
      syncPersistedHistoryState(history.value);
      throw error;
    }
  }

  async function updateTags(id, tagColors) {
    const index = history.value.findIndex((item) => item.id === id);
    if (index === -1) {
      await updateItemTagsRequest(id, tagColors);
      await refreshHistory();
      return;
    }

    const currentFilterTag = activeTagFilter.value;
    const originalItem = history.value[index];
    const previous = originalItem.tagColors ?? [];
    const nextItem = {
      ...originalItem,
      tagColors,
    };
    const shouldRemoveFromCurrentList =
      Boolean(currentFilterTag) && !tagColors.includes(currentFilterTag);
    let removedItem = null;
    let removedIndex = -1;

    if (shouldRemoveFromCurrentList) {
      skipNextFilterRefresh.value = true;
      removedIndex = index;
      removedItem = originalItem;
      history.value.splice(index, 1);
      updateSelectedAfterListChange(filteredHistory.value, id);
      syncPersistedHistoryState(history.value);
    } else {
      updateHistoryItemAt(index, nextItem);
    }

    try {
      await updateItemTagsRequest(id, tagColors);
      if (shouldRemoveFromCurrentList) {
        await refreshHistory({
          detectNewHistory: false,
        });
      }
    } catch (error) {
      if (removedItem && removedIndex !== -1) {
        history.value.splice(removedIndex, 0, {
          ...removedItem,
          tagColors: previous,
        });
        await refreshHistory({
          detectNewHistory: false,
        });
      } else {
        const rollbackIndex = history.value.findIndex((item) => item.id === id);
        if (rollbackIndex !== -1) {
          updateHistoryItemAt(rollbackIndex, {
            ...history.value[rollbackIndex],
            tagColors: previous,
          });
        } else {
          await refreshHistory();
        }
      }
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
    totalHistoryCount.value = 0;
    await refreshHistory();
  }

  async function loadMoreIfPanelHasRoom() {
    await nextTick();

    const panel = historyPanelRef.value;
    if (
      !panel ||
      loading.value ||
      loadingMore.value ||
      !hasMoreHistory.value ||
      panel.scrollHeight > panel.clientHeight + 24
    ) {
      return;
    }

    void loadMoreHistory();
  }

  watch(selectedId, () => {
    void scrollSelectedIntoView();
  });

  watch([query, activeFilterTab, activeTagFilter], () => {
    if (skipNextFilterRefresh.value) {
      skipNextFilterRefresh.value = false;
      return;
    }

    void refreshHistory({
      detectNewHistory: false,
    });
  });

  watch(
    () => [settings.copyStatsEnabled, settings.pasteStatsEnabled],
    () => {
      void refreshHistory({
        detectNewHistory: false,
      });
    },
  );

  watch(selectedId, () => {
    syncPersistedHistoryState(history.value);
  });

  watch(filteredHistory, (items) => {
    if (!items.length && hasMoreHistory.value && !loading.value && loadedHistoryOffset.value > 0) {
      void loadMoreHistory();
    }

    void loadMoreIfPanelHasRoom();

    if (!items.some((item) => item.id === selectedId.value)) {
      selectedId.value = items[0]?.id ?? null;
    }
  });

  return {
    activeFilterTab,
    activeTagFilter,
    clearHistory,
    copyItem,
    editDraft,
    editingItemId,
    filteredHistory,
    hasMoreHistory,
    history,
    historyCountLabel,
    historyPanelRef,
    historyTabs,
    loading,
    loadingMore,
    loadMoreHistory,
    openEditModal,
    openExternalUrl,
    pasteItem,
    query,
    refreshHistory,
    refreshRelativeTimes,
    relativeTimeVersion,
    availableTagFilters,
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
    updateTags,
    totalHistoryCount,
  };
}
