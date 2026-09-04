import { computed, ref, watch } from 'vue'
import { HISTORY_TAG_COLORS, resolveTagLabel } from '../utils/constants'

const ACTIVE_FILTER_TAB_STORAGE_KEY = 'clipdesk.activeFilterTab'
const ACTIVE_TAG_FILTER_STORAGE_KEY = 'clipdesk.activeTagFilter'

export function useHistoryFilters({ history, settings, t }) {
  const query = ref('')
  const activeFilterTab = ref(
    window.localStorage.getItem(ACTIVE_FILTER_TAB_STORAGE_KEY) || 'today',
  )
  const activeTagFilter = ref(
    window.localStorage.getItem(ACTIVE_TAG_FILTER_STORAGE_KEY) || '',
  )

  const historyTabs = computed(() => [
    { key: 'today', label: t('filterToday') },
    { key: 'all', label: t('filterAll') },
    { key: 'pinned', label: t('filterPinned') },
    { key: 'text', label: t('filterText') },
    { key: 'image', label: t('filterImage') },
    { key: 'mixed', label: t('filterMixed') },
  ])

  const availableTagFilters = computed(() => {
    const activeColors = new Set()

    for (const item of history.value) {
      if (!Array.isArray(item.tagColors)) {
        continue
      }

      for (const color of item.tagColors) {
        activeColors.add(color)
      }
    }

    return HISTORY_TAG_COLORS.filter((color) => activeColors.has(color)).map((color) => ({
      key: color,
      label: resolveTagLabel(color, settings.tagLabels, t),
      color,
    }))
  })

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
      createdFrom: null,
      createdBefore: null,
    }

    if (activeFilterTab.value === 'pinned') {
      payload.pinnedOnly = true
    } else if (activeFilterTab.value === 'today') {
      const startOfToday = new Date()
      startOfToday.setHours(0, 0, 0, 0)
      const startOfTomorrow = new Date(startOfToday)
      startOfTomorrow.setDate(startOfTomorrow.getDate() + 1)
      payload.createdFrom = startOfToday.toISOString()
      payload.createdBefore = startOfTomorrow.toISOString()
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
