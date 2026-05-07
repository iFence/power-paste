import { nextTick, ref } from 'vue'

const SELECTED_HISTORY_ID_STORAGE_KEY = 'clipdesk.selectedHistoryId'
const LATEST_HISTORY_ID_STORAGE_KEY = 'clipdesk.latestHistoryId'

function getLatestHistoryItem(items) {
  return (
    [...items].sort((left, right) => {
      const createdAtCompare = (right.createdAt ?? '').localeCompare(
        left.createdAt ?? '',
      )
      if (createdAtCompare !== 0) {
        return createdAtCompare
      }

      return String(right.id ?? '').localeCompare(String(left.id ?? ''))
    })[0] ?? null
  )
}

export function useHistorySelection() {
  const selectedId = ref(
    window.localStorage.getItem(SELECTED_HISTORY_ID_STORAGE_KEY),
  )
  const historyPanelRef = ref(null)

  function syncPersistedHistoryState(items) {
    const latestHistoryItem = getLatestHistoryItem(items)

    if (selectedId.value) {
      window.localStorage.setItem(
        SELECTED_HISTORY_ID_STORAGE_KEY,
        selectedId.value,
      )
    } else {
      window.localStorage.removeItem(SELECTED_HISTORY_ID_STORAGE_KEY)
    }

    if (latestHistoryItem?.id) {
      window.localStorage.setItem(
        LATEST_HISTORY_ID_STORAGE_KEY,
        latestHistoryItem.id,
      )
    } else {
      window.localStorage.removeItem(LATEST_HISTORY_ID_STORAGE_KEY)
    }
  }

  function restoreSelection(items) {
    const latestHistoryItem = getLatestHistoryItem(items)
    const previousLatestHistoryId = window.localStorage.getItem(
      LATEST_HISTORY_ID_STORAGE_KEY,
    )
    const persistedSelectedId = window.localStorage.getItem(
      SELECTED_HISTORY_ID_STORAGE_KEY,
    )
    const hasNewHistory =
      Boolean(previousLatestHistoryId) &&
      Boolean(latestHistoryItem?.id) &&
      latestHistoryItem.id !== previousLatestHistoryId

    if (hasNewHistory) {
      selectedId.value = latestHistoryItem.id
      return { hasNewHistory, latestHistoryItem }
    }

    if (persistedSelectedId && items.some((item) => item.id === persistedSelectedId)) {
      selectedId.value = persistedSelectedId
    } else if (
      !selectedId.value ||
      !items.some((item) => item.id === selectedId.value)
    ) {
      selectedId.value = latestHistoryItem?.id ?? items[0]?.id ?? null
    }

    return { hasNewHistory, latestHistoryItem }
  }

  function updateSelectedAfterListChange(items, removedId = null) {
    if (!items.length) {
      selectedId.value = null
      return
    }

    if (removedId && selectedId.value === removedId) {
      selectedId.value = items[0]?.id ?? null
      return
    }

    if (!items.some((item) => item.id === selectedId.value)) {
      selectedId.value = items[0]?.id ?? null
    }
  }

  async function scrollSelectedIntoView() {
    await nextTick()

    const panel = historyPanelRef.value
    if (!panel || !selectedId.value) {
      return
    }

    const activeItem = panel.querySelector(
      `[data-history-id="${selectedId.value}"]`,
    )
    if (!(activeItem instanceof HTMLElement)) {
      return
    }

    const margin = 12
    const panelRect = panel.getBoundingClientRect()
    const itemRect = activeItem.getBoundingClientRect()
    const topDelta = itemRect.top - panelRect.top
    const bottomDelta = itemRect.bottom - panelRect.bottom

    if (topDelta < margin) {
      panel.scrollTo({
        top: Math.max(0, panel.scrollTop + topDelta - margin),
        behavior: 'smooth',
      })
      return
    }

    if (bottomDelta > -margin) {
      panel.scrollTo({
        top: Math.max(0, panel.scrollTop + bottomDelta + margin),
        behavior: 'smooth',
      })
    }
  }

  return {
    historyPanelRef,
    restoreSelection,
    scrollSelectedIntoView,
    selectedId,
    syncPersistedHistoryState,
    updateSelectedAfterListChange,
  }
}
