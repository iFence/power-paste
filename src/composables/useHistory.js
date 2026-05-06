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
import { HISTORY_TAG_COLORS, resolveTagLabel } from "../utils/constants";

const ACTIVE_FILTER_TAB_STORAGE_KEY = "clipdesk.activeFilterTab";
const ACTIVE_TAG_FILTER_STORAGE_KEY = "clipdesk.activeTagFilter";
const SELECTED_HISTORY_ID_STORAGE_KEY = "clipdesk.selectedHistoryId";
const LATEST_HISTORY_ID_STORAGE_KEY = "clipdesk.latestHistoryId";
const HISTORY_PAGE_SIZE = 30;
let copySoundContext = null;
let copySoundPending = false;
let copySoundResumePromise = null;

function getCopySoundContext() {
  const AudioContext = window.AudioContext || window.webkitAudioContext;
  if (!AudioContext) {
    return null;
  }

  if (!copySoundContext || copySoundContext.state === "closed") {
    copySoundContext = new AudioContext();
  }

  return copySoundContext;
}

function playGeneratedCopyTone(context) {
  const startTime = context.currentTime;
  const duration = 0.06;
  const endTime = startTime + duration;
  const oscillator = context.createOscillator();
  const filter = context.createBiquadFilter();
  const gain = context.createGain();

  oscillator.type = "triangle";
  oscillator.frequency.setValueAtTime(625, startTime);
  oscillator.frequency.exponentialRampToValueAtTime(460, endTime);

  filter.type = "bandpass";
  filter.frequency.setValueAtTime(700, startTime);
  filter.Q.setValueAtTime(6, startTime);

  gain.gain.setValueAtTime(0.0001, startTime);
  gain.gain.exponentialRampToValueAtTime(0.28, startTime + 0.004);
  gain.gain.exponentialRampToValueAtTime(0.0001, endTime);

  oscillator.connect(filter);
  filter.connect(gain);
  gain.connect(context.destination);

  oscillator.start(startTime);
  oscillator.stop(endTime + 0.01);
}

function markCopySoundPending() {
  copySoundPending = true;
}

function clearPendingCopySound() {
  copySoundPending = false;
}

function tryPlayCopySound(context) {
  try {
    playGeneratedCopyTone(context);
    clearPendingCopySound();
    return true;
  } catch (error) {
    console.warn("Failed to play copy sound", error);
    return false;
  }
}

function resumeCopySoundContext(context) {
  if (copySoundResumePromise) {
    return copySoundResumePromise;
  }

  copySoundResumePromise = context
    .resume()
    .then(() => {
      copySoundResumePromise = null;
      return true;
    })
    .catch((error) => {
      copySoundResumePromise = null;
      console.warn("Failed to resume copy sound context", error);
      return false;
    });

  return copySoundResumePromise;
}

export function playCopySoundFallback() {
  const context = getCopySoundContext();
  if (!context) {
    return;
  }

  if (context.state === "suspended") {
    markCopySoundPending();
    void resumeCopySoundContext(context).then((resumed) => {
      if (resumed) {
        tryPlayCopySound(context);
      }
    });
    return;
  }

  tryPlayCopySound(context);
}

export function flushPendingCopySound() {
  if (!copySoundPending) {
    return;
  }

  const context = getCopySoundContext();
  if (!context) {
    return;
  }

  if (context.state === "running") {
    tryPlayCopySound(context);
    return;
  }

  if (context.state === "suspended") {
    void resumeCopySoundContext(context).then((resumed) => {
      if (resumed) {
        tryPlayCopySound(context);
      }
    });
  }
}

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

function getLatestHistoryItem(items) {
  return (
    [...items].sort((left, right) => {
      const createdAtCompare = (right.createdAt ?? "").localeCompare(
        left.createdAt ?? "",
      );
      if (createdAtCompare !== 0) {
        return createdAtCompare;
      }

      return String(right.id ?? "").localeCompare(String(left.id ?? ""));
    })[0] ?? null
  );
}

export function useHistory({ platformCapabilities, settings, t }) {
  const query = ref("");
  const activeFilterTab = ref(
    window.localStorage.getItem(ACTIVE_FILTER_TAB_STORAGE_KEY) || "all",
  );
  const activeTagFilter = ref(
    window.localStorage.getItem(ACTIVE_TAG_FILTER_STORAGE_KEY) || "",
  );
  const history = ref([]);
  const loading = ref(true);
  const relativeTimeVersion = ref(0);
  const selectedId = ref(
    window.localStorage.getItem(SELECTED_HISTORY_ID_STORAGE_KEY),
  );
  const historyPanelRef = ref(null);
  const showEditModal = ref(false);
  const editingItemId = ref(null);
  const editDraft = ref("");
  const actionFeedback = ref("");
  const loadingMore = ref(false);
  const hasMoreHistory = ref(true);
  const loadedHistoryOffset = ref(0);
  const totalHistoryCount = ref(0);

  const filteredHistory = computed(() =>
    history.value.filter((item) => {
      if (activeFilterTab.value === "mixed" && item.kind !== "mixed") {
        return false;
      }
      if (
        activeFilterTab.value === "text" &&
        !["text", "link"].includes(item.kind)
      ) {
        return false;
      }
      if (activeFilterTab.value === "image" && item.kind !== "image") {
        return false;
      }
      if (activeFilterTab.value === "pinned" && !item.pinned) {
        return false;
      }
      if (
        activeTagFilter.value &&
        !(Array.isArray(item.tagColors) && item.tagColors.includes(activeTagFilter.value))
      ) {
        return false;
      }

      const lower = query.value.trim().toLowerCase();
      if (!lower) {
        return true;
      }

      const haystack =
        `${item.preview}\n${item.fullText ?? ""}\n${item.sourceApp ?? ""}`.toLowerCase();
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
  const availableTagFilters = computed(() =>
    HISTORY_TAG_COLORS.filter((color) =>
      history.value.some((item) => Array.isArray(item.tagColors) && item.tagColors.includes(color)),
    ).map((color) => ({
      key: color,
      label: resolveTagLabel(color, settings.tagLabels, t),
      color,
    })),
  );

  function syncActiveFilterTab() {
    const availableTabs = new Set(historyTabs.value.map((tab) => tab.key));
    if (!availableTabs.has(activeFilterTab.value)) {
      activeFilterTab.value = "all";
      return;
    }

    window.localStorage.setItem(
      ACTIVE_FILTER_TAB_STORAGE_KEY,
      activeFilterTab.value,
    );
  }

  function syncActiveTagFilter() {
    const availableTags = new Set(availableTagFilters.value.map((tag) => tag.key));
    if (activeTagFilter.value && !availableTags.has(activeTagFilter.value)) {
      activeTagFilter.value = "";
      window.localStorage.removeItem(ACTIVE_TAG_FILTER_STORAGE_KEY);
      return;
    }

    if (activeTagFilter.value) {
      window.localStorage.setItem(ACTIVE_TAG_FILTER_STORAGE_KEY, activeTagFilter.value);
    } else {
      window.localStorage.removeItem(ACTIVE_TAG_FILTER_STORAGE_KEY);
    }
  }

  function syncPersistedHistoryState(items = history.value) {
    const latestHistoryItem = getLatestHistoryItem(items);

    if (selectedId.value) {
      window.localStorage.setItem(
        SELECTED_HISTORY_ID_STORAGE_KEY,
        selectedId.value,
      );
    } else {
      window.localStorage.removeItem(SELECTED_HISTORY_ID_STORAGE_KEY);
    }

    if (latestHistoryItem?.id) {
      window.localStorage.setItem(
        LATEST_HISTORY_ID_STORAGE_KEY,
        latestHistoryItem.id,
      );
    } else {
      window.localStorage.removeItem(LATEST_HISTORY_ID_STORAGE_KEY);
    }
  }

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
    loadedHistoryOffset.value = 0;
    hasMoreHistory.value = true;
    try {
      const limit = nextHistoryPageLimit();
      const page = await getHistory({
        query: query.value.trim() || null,
        limit,
        offset: 0,
      });
      const items = page.items;
      totalHistoryCount.value = page.totalCount;
      loadedHistoryOffset.value = items.length;
      updateHistoryPaginationState(items.length, limit);
      reorderHistory(items);
      const latestHistoryItem = getLatestHistoryItem(items);
      const previousLatestHistoryId = window.localStorage.getItem(
        LATEST_HISTORY_ID_STORAGE_KEY,
      );
      const persistedSelectedId = window.localStorage.getItem(
        SELECTED_HISTORY_ID_STORAGE_KEY,
      );
      const hasNewHistory =
        Boolean(previousLatestHistoryId) &&
        Boolean(latestHistoryItem?.id) &&
        latestHistoryItem.id !== previousLatestHistoryId;

      if (hasNewHistory) {
        activeFilterTab.value = "all";
        selectedId.value = latestHistoryItem.id;
      } else if (
        persistedSelectedId &&
        items.some((item) => item.id === persistedSelectedId)
      ) {
        selectedId.value = persistedSelectedId;
      } else if (
        !selectedId.value ||
        !items.some((item) => item.id === selectedId.value)
      ) {
        selectedId.value = latestHistoryItem?.id ?? items[0]?.id ?? null;
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
      const page = await getHistory({
        query: query.value.trim() || null,
        limit,
        offset: loadedHistoryOffset.value,
      });
      const items = page.items;
      totalHistoryCount.value = page.totalCount;
      loadedHistoryOffset.value += items.length;

      const loadedIds = new Set(history.value.map((item) => item.id));
      const nextItems = items.filter((item) => !loadedIds.has(item.id));
      if (nextItems.length) {
        reorderHistory([...history.value, ...nextItems]);
      }

      updateHistoryPaginationState(items.length, limit);
      updateSelectedAfterListChange();
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
      updateSelectedAfterListChange();
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
    totalHistoryCount.value = Math.max(0, totalHistoryCount.value - 1);
    updateSelectedAfterListChange(id);

    try {
      await deleteItem(id);
    } catch (error) {
      history.value.splice(index, 0, removedItem);
      history.value = [...history.value];
      totalHistoryCount.value += 1;
      reorderHistory();
      updateSelectedAfterListChange();
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

  async function scrollSelectedIntoView() {
    await nextTick();

    const panel = historyPanelRef.value;
    if (!panel || !selectedId.value) {
      return;
    }

    const activeItem = panel.querySelector(
      `[data-history-id="${selectedId.value}"]`,
    );
    if (!(activeItem instanceof HTMLElement)) {
      return;
    }

    const margin = 12;
    const panelRect = panel.getBoundingClientRect();
    const itemRect = activeItem.getBoundingClientRect();
    const topDelta = itemRect.top - panelRect.top;
    const bottomDelta = itemRect.bottom - panelRect.bottom;

    if (topDelta < margin) {
      panel.scrollTo({
        top: Math.max(0, panel.scrollTop + topDelta - margin),
        behavior: "smooth",
      });
      return;
    }

    if (bottomDelta > -margin) {
      panel.scrollTo({
        top: Math.max(0, panel.scrollTop + bottomDelta + margin),
        behavior: "smooth",
      });
    }
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

  watch(activeFilterTab, () => {
    syncActiveFilterTab();
  });

  watch(activeTagFilter, () => {
    syncActiveTagFilter();
  });

  watch(availableTagFilters, () => {
    syncActiveTagFilter();
  });

  watch(selectedId, () => {
    syncPersistedHistoryState();
  });

  watch(history, () => {
    syncPersistedHistoryState();
  });

  watch(filteredHistory, (items) => {
    if (!items.length && hasMoreHistory.value && !loading.value) {
      void loadMoreHistory();
    }

    void loadMoreIfPanelHasRoom();

    if (!items.some((item) => item.id === selectedId.value)) {
      selectedId.value = items[0]?.id ?? null;
    }
  });

  syncActiveFilterTab();
  syncActiveTagFilter();

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
