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
import { useHistorySelection } from "./useHistorySelection";

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

function compareHistoryItems(left, right) {
  if (left.pinned !== right.pinned) {
    return Number(right.pinned) - Number(left.pinned);
  }

  const pinnedAtCompare = (right.pinnedAt ?? "").localeCompare(
    left.pinnedAt ?? "",
  );
  if (pinnedAtCompare !== 0) {
    return pinnedAtCompare;
  }

  if (left.favorite !== right.favorite) {
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

  async function refreshHistory() {
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
      reorderHistory(items);
      const { hasNewHistory } = restoreSelection(items);

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
        reorderHistory([...history.value, ...nextItems]);
      }

      updateHistoryPaginationState(items.length, limit);
      updateSelectedAfterListChange(history.value);
      syncPersistedHistoryState();
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
      history.value = [item, ...history.value];
      totalHistoryCount.value += 1;
    } else {
      history.value[index] = {
        ...history.value[index],
        ...item,
      };
      history.value = [...history.value];
    }

    reorderHistory();
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

    syncPersistedHistoryState();
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

    history.value[index] = {
      ...current,
      pinned: nextPinned,
      pinnedAt: nextPinned ? new Date().toISOString() : null,
    };
    reorderHistory();
      updateSelectedAfterListChange(filteredHistory.value);

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
        updateSelectedAfterListChange(filteredHistory.value);
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
    totalHistoryCount.value = Math.max(0, totalHistoryCount.value - 1);
    updateSelectedAfterListChange(filteredHistory.value, id);

    try {
      await deleteItem(id);
    } catch (error) {
      history.value.splice(index, 0, removedItem);
      history.value = [...history.value];
      totalHistoryCount.value += 1;
      reorderHistory();
      updateSelectedAfterListChange(filteredHistory.value);
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

    const previous = history.value[index].tagColors ?? [];
    history.value[index] = {
      ...history.value[index],
      tagColors,
    };
    history.value = [...history.value];

    try {
      await updateItemTagsRequest(id, tagColors);
    } catch (error) {
      const rollbackIndex = history.value.findIndex((item) => item.id === id);
      if (rollbackIndex !== -1) {
        history.value[rollbackIndex] = {
          ...history.value[rollbackIndex],
          tagColors: previous,
        };
        history.value = [...history.value];
      } else {
        await refreshHistory();
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
    void refreshHistory();
  });

  watch(selectedId, () => {
    syncPersistedHistoryState(history.value);
  });

  watch(history, () => {
    syncPersistedHistoryState();
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
