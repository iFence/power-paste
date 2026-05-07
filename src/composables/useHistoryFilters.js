import { computed, ref, watch } from 'vue'
import { HISTORY_TAG_COLORS, resolveTagLabel } from '../utils/constants'

const ACTIVE_FILTER_TAB_STORAGE_KEY = 'clipdesk.activeFilterTab'
const ACTIVE_TAG_FILTER_STORAGE_KEY = 'clipdesk.activeTagFilter'

export function useHistoryFilters({ history, settings, t }) {
  const query = ref('')
  const activeFilterTab = ref(
    window.localStorage.getItem(ACTIVE_FILTER_TAB_STORAGE_KEY) || 'all',
  )
  const activeTagFilter = ref(
    window.localStorage.getItem(ACTIVE_TAG_FILTER_STORAGE_KEY) || '',
  )

  const historyTabs = computed(() => [
    { key: 'all', label: t('filterAll') },
    { key: 'pinned', label: t('filterPinned') },
    { key: 'text', label: t('filterText') },
    { key: 'image', label: t('filterImage') },
    { key: 'mixed', label: t('filterMixed') },
  ])

  const availableTagFilters = computed(() =>
    HISTORY_TAG_COLORS.filter((color) =>
      history.value.some(
        (item) => Array.isArray(item.tagColors) && item.tagColors.includes(color),
      ),
    ).map((color) => ({
      key: color,
      label: resolveTagLabel(color, settings.tagLabels, t),
      color,
    })),
  )

  function syncActiveFilterTab() {
    const availableTabs = new Set(historyTabs.value.map((tab) => tab.key))
    if (!availableTabs.has(activeFilterTab.value)) {
      activeFilterTab.value = 'all'
      return
    }

    window.localStorage.setItem(
      ACTIVE_FILTER_TAB_STORAGE_KEY,
      activeFilterTab.value,
    )
  }

  function syncActiveTagFilter() {
    const availableTags = new Set(availableTagFilters.value.map((tag) => tag.key))
    if (activeTagFilter.value && !availableTags.has(activeTagFilter.value)) {
      activeTagFilter.value = ''
      window.localStorage.removeItem(ACTIVE_TAG_FILTER_STORAGE_KEY)
      return
    }

    if (activeTagFilter.value) {
      window.localStorage.setItem(
        ACTIVE_TAG_FILTER_STORAGE_KEY,
        activeTagFilter.value,
      )
    } else {
      window.localStorage.removeItem(ACTIVE_TAG_FILTER_STORAGE_KEY)
    }
  }

  function buildHistoryQueryPayload(limit, offset = 0) {
    const payload = {
      limit,
      offset,
      query: query.value.trim() || null,
      kind: null,
      pinnedOnly: false,
      tagColor: activeTagFilter.value || null,
    }

    if (activeFilterTab.value === 'pinned') {
      payload.pinnedOnly = true
    } else if (activeFilterTab.value !== 'all') {
      payload.kind = activeFilterTab.value
    }

    return payload
  }

  watch(activeFilterTab, syncActiveFilterTab)
  watch(activeTagFilter, syncActiveTagFilter)
  watch(availableTagFilters, syncActiveTagFilter)

  syncActiveFilterTab()
  syncActiveTagFilter()

  return {
    activeFilterTab,
    activeTagFilter,
    availableTagFilters,
    buildHistoryQueryPayload,
    historyTabs,
    query,
  }
}
