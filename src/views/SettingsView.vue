<script setup>
import { computed, nextTick, onUnmounted, ref, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import DOMPurify from 'dompurify'
import { marked } from 'marked'
import { getInstalledAppIcon, listInstalledApps, openExternalUrl } from '../services/tauriApi'
import { normalizeShortcutKey } from '../utils/shortcut'
import { HISTORY_TAG_COLORS, resolveTagLabel } from '../utils/constants'
import checkIcon from '../assets/check.svg'

const ABOUT_INFO = {
  landingPageUrl: 'https://power-paste.hiaspirin.cc',
  repositoryUrl: 'https://github.com/iFence/power-paste',
}
const SETTINGS_ACTIVE_CATEGORY_STORAGE_KEY = 'clipdesk.settings.activeCategory'

marked.setOptions({
  breaks: true,
  gfm: true,
})

const props = defineProps({
  appVersion: { type: String, required: true },
  applySettingPatch: { type: Function, required: true },
  applyWebdavSyncPatch: { type: Function, required: true },
  beginShortcutRecording: { type: Function, required: true },
  clearWebdavPassword: { type: Function, required: true },
  canToggleLaunchOnStartup: { type: Boolean, required: true },
  closeSelect: { type: Function, required: true },
  currentAccentColorOptions: { type: Array, required: true },
  currentLocale: { type: String, required: true },
  currentThemeModeOptions: { type: Array, required: true },
  endShortcutRecording: { type: Function, required: true },
  localeOptions: { type: Array, required: true },
  onBack: { type: Function, required: true },
  onCheckUpdates: { type: Function, required: true },
  onClearUpdateDebugStatus: { type: Function, required: true },
  onInstallUpdate: { type: Function, required: true },
  onSetUpdateDebugStatusWithOverrides: { type: Function, required: true },
  openSelectKey: { type: String, default: null },
  pendingSettingKey: { type: String, default: '' },
  platformCapabilities: { type: Object, required: true },
  recordingShortcut: { type: Boolean, required: true },
  resetSettings: { type: Function, required: true },
  retryShortcutRegistration: { type: Function, required: true },
  runWebdavSyncNow: { type: Function, required: true },
  runWebdavTest: { type: Function, required: true },
  saveWebdavPassword: { type: Function, required: true },
  savingSettings: { type: Boolean, required: true },
  settings: { type: Object, required: true },
  settingsSaveError: { type: String, required: true },
  shortcutRetrying: { type: Boolean, required: true },
  shortcutStatus: { type: Object, required: true },
  showUpdateAction: { type: Boolean, required: true },
  segmentedToggleStyle: { type: Function, required: true },
  selectedOptionLabel: { type: Function, required: true },
  t: { type: Function, required: true },
  toggleSelect: { type: Function, required: true },
  updateDebugEnabled: { type: Boolean, required: true },
  updateDebugStatus: { type: String, default: null },
  updateBusy: { type: Boolean, required: true },
  updateLabel: { type: String, required: true },
  updateStatusMessage: { type: String, required: true },
  updateState: { type: Object, required: true },
  webdavPasswordDraft: { type: String, default: '' },
  webdavCredentialSaved: { type: Boolean, required: true },
  webdavSyncStatus: { type: Object, required: true },
})

const activeCategory = ref(window.localStorage.getItem(SETTINGS_ACTIVE_CATEGORY_STORAGE_KEY) || 'general')
const showUpdateConfirm = ref(false)
const showUpdateFeedback = ref(false)
const tooltipState = ref({
  visible: false,
  text: '',
  top: 0,
  left: 0,
  placement: 'top',
})
const updateDebugVersionDraft = ref('')
const updateDebugBodyDraft = ref('')
const maxHistoryItemsDraft = ref(200)
const maxHistoryDaysDraft = ref(30)
const maxImageBytesMbDraft = ref(6)
const appPickerOpen = ref(false)
const appPickerLoading = ref(false)
const appPickerError = ref('')
const appSearchQuery = ref('')
const installedApps = ref([])
let updateFeedbackTimer = null
let appIconHydrationRun = 0
const appIconLoadingKeys = new Set()
const appIconResolvedKeys = new Set()
const APP_ICON_HYDRATION_BATCH_SIZE = 16
const APP_ICON_HYDRATION_CONCURRENCY = 3

const categories = computed(() => [
  { key: 'general', label: props.t('settingsCategoryGeneral') },
  { key: 'history', label: props.t('settingsCategoryHistory') },
  { key: 'sync', label: props.t('settingsCategorySync') },
  { key: 'transfer', label: props.t('settingsCategoryTransfer') },
  { key: 'shortcuts', label: props.t('settingsCategoryShortcuts') },
  { key: 'advanced', label: props.t('settingsCategoryAdvanced') },
  { key: 'about', label: props.t('settingsCategoryAbout') },
])
const languageToggleIndex = computed(() =>
  Math.max(props.localeOptions.findIndex((option) => option.value === props.settings.locale), 0),
)
const debugToggleIndex = computed(() => (props.settings.debugEnabled ? 0 : 1))
const hardwareAccelerationToggleIndex = computed(() =>
  props.settings.hardwareAccelerationEnabled === true ? 0 : 1,
)
const soundToggleIndex = computed(() => (props.settings.soundEnabled ? 0 : 1))
const launchToggleIndex = computed(() => (props.settings.launchOnStartup ? 0 : 1))
const copyStatsToggleIndex = computed(() => (props.settings.copyStatsEnabled ? 0 : 1))
const pasteStatsToggleIndex = computed(() => (props.settings.pasteStatsEnabled ? 0 : 1))
const windowControlStyleToggleIndex = computed(() =>
  props.settings.windowControlStyle === 'windows' ? 1 : 0,
)
const webdavEnabledToggleIndex = computed(() => (props.settings.webdavSync?.enabled ? 0 : 1))
const webdavAutoSyncToggleIndex = computed(() => (props.settings.webdavSync?.autoSync ? 0 : 1))
const hasClipboardWriteSupport = computed(
  () =>
    props.platformCapabilities.supportsTextWrite ||
    props.platformCapabilities.supportsHtmlWrite ||
    props.platformCapabilities.supportsImageWrite,
)
const installedAppPickerSupported = computed(() =>
  ['windows', 'macos'].includes(props.platformCapabilities.platform),
)
const ignoredAppRules = computed(() =>
  Array.isArray(props.settings.ignoredAppRules) ? props.settings.ignoredAppRules : [],
)
const ignoredAppKeys = computed(() => new Set(ignoredAppRules.value.map(appRuleKey)))
const filteredInstalledApps = computed(() => {
  const query = appSearchQuery.value.trim().toLowerCase()
  const apps = Array.isArray(installedApps.value) ? installedApps.value : []
  if (!query) {
    return apps
  }

  return apps.filter((app) =>
    [app.displayName, app.processName, app.appPath, app.bundleId]
      .filter(Boolean)
      .some((value) => value.toLowerCase().includes(query)),
  )
})
const shortcutIssuesByKey = computed(() => {
  const issues = props.shortcutStatus?.issues || []
  return issues.reduce((acc, issue) => {
    acc[issue.key] = issue
    return acc
  }, {})
})

const updateNotes = computed(() => {
  const body = props.updateState?.body
  if (typeof body !== 'string' || !body.trim()) {
    return props.t('updateNotesEmpty')
  }

  return body.trim()
})
const updateNotesHtml = computed(() => {
  const rawHtml = marked.parse(updateNotes.value)
  return DOMPurify.sanitize(rawHtml, {
    ALLOWED_TAGS: [
      'a',
      'code',
      'em',
      'h1',
      'h2',
      'h3',
      'h4',
      'h5',
      'h6',
      'li',
      'ol',
      'p',
      'pre',
      'strong',
      'ul',
      'br',
    ],
    ALLOWED_ATTR: ['href', 'target', 'rel'],
  })
})
const updateDebugOptions = computed(() => [
  { value: 'available', label: props.t('updateDebugAvailable') },
  { value: 'downloading', label: props.t('updateDebugDownloading') },
  { value: 'downloaded', label: props.t('updateDebugDownloaded') },
  { value: 'up_to_date', label: props.t('updateDebugUpToDate') },
  { value: 'error', label: props.t('updateDebugError') },
])
const updateDebugVersionValue = computed(() => {
  const version =
    typeof props.updateState?.latestVersion === 'string'
      ? props.updateState.latestVersion.trim()
      : ''
  return version || '9.9.9-dev'
})
const updateDebugBodyValue = computed(() => {
  const body = typeof props.updateState?.body === 'string' ? props.updateState.body.trim() : ''
  return (
    body ||
    [
      '## Debug Update',
      '- Preview the update badge in development.',
      '- Validate the confirmation dialog layout and release notes.',
      '- Exercise downloading and error states without a real release.',
    ].join('\n')
  )
})
const updateHeaderMessage = computed(() => {
  if (!props.updateState) {
    return ''
  }

  if (['idle', 'up_to_date', 'downloaded'].includes(props.updateState.status)) {
    return ''
  }

  return props.updateStatusMessage
})
const updateBadgeLabel = computed(() => {
  if (props.updateState?.status === 'downloading' && updateHeaderMessage.value) {
    return updateHeaderMessage.value
  }

  return props.showUpdateAction ? props.updateLabel : props.t('checkForUpdates')
})
const tooltipStyle = computed(() => ({
  top: `${tooltipState.value.top}px`,
  left: `${tooltipState.value.left}px`,
}))

function isPending(key) {
  return props.savingSettings && (!key || props.pendingSettingKey === key)
}

function shortcutIssueText(key) {
  const issue = shortcutIssuesByKey.value[key]
  if (!issue) {
    return ''
  }

  const labelMap = {
    globalShortcut: props.t('globalShortcut'),
    quickPasteShortcut: props.t('quickPasteShortcut'),
    searchShortcut: props.t('searchShortcut'),
    filterShortcut: props.t('filterShortcut'),
  }

  return props.t('shortcutConflictMessage', {
    name: labelMap[key] || props.t('globalShortcut'),
    shortcut: issue.shortcut || props.settings[key],
  })
}

async function updateSetting(field, value, key = field) {
  if (props.settings[field] === value) {
    return
  }

  await props.applySettingPatch({ [field]: value }, key)
}

async function updateWebdavSetting(field, value, key = `webdavSync.${field}`) {
  if ((props.settings.webdavSync?.[field] ?? '') === value) {
    return
  }

  await props.applyWebdavSyncPatch({ [field]: value }, key)
}

async function chooseSelectOption(key, field, value) {
  props.closeSelect()
  await updateSetting(field, value, key)
}

async function updateCopyStatsEnabled(value) {
  await props.applySettingPatch(
    {
      copyStatsEnabled: value,
      pasteStatsEnabled: value ? false : props.settings.pasteStatsEnabled,
    },
    'copyStatsEnabled',
  )
}

async function updatePasteStatsEnabled(value) {
  await props.applySettingPatch(
    {
      pasteStatsEnabled: value,
      copyStatsEnabled: value ? false : props.settings.copyStatsEnabled,
    },
    'pasteStatsEnabled',
  )
}

async function handleWebdavPasswordChange(event) {
  const password = typeof event?.target?.value === 'string' ? event.target.value : ''
  await props.saveWebdavPassword(password)
}


function normalizeAppPathKey(value) {
  return String(value || '').trim().replace(/\\/g, '/').toLowerCase()
}

function appRuleKey(app) {
  if (!app) {
    return ''
  }
  if (app.appPath) {
    return `path:${normalizeAppPathKey(app.appPath)}`
  }
  if (app.bundleId) {
    return `bundle:${String(app.bundleId).trim().toLowerCase()}`
  }
  if (app.processName) {
    return `process:${String(app.processName).trim().toLowerCase()}`
  }
  return `display:${String(app.displayName || '').trim().toLowerCase()}`
}

function appInitial(name) {
  return String(name || '?').trim().slice(0, 1).toUpperCase() || '?'
}

function appSecondaryText(app) {
  return app?.appPath || app?.bundleId || app?.processName || ''
}

function installedAppToRule(app) {
  return {
    platform: app.platform || props.platformCapabilities.platform || '',
    displayName: app.displayName || app.processName || '',
    processName: app.processName || '',
    appPath: app.appPath || null,
    bundleId: app.bundleId || null,
    iconDataUrl: app.iconDataUrl || null,
    enabled: true,
  }
}

async function loadInstalledAppIcon(app) {
  return getInstalledAppIcon({
    displayName: app.displayName || app.processName || '',
    processName: app.processName || '',
    appPath: app.appPath || null,
    bundleId: app.bundleId || null,
  })
}

function updateInstalledAppIcon(key, iconDataUrl) {
  if (!key || !iconDataUrl) {
    return
  }

  updateInstalledAppIcons(new Map([[key, iconDataUrl]]))
}

function updateInstalledAppIcons(iconMap) {
  if (!iconMap?.size) {
    return
  }

  installedApps.value = installedApps.value.map((app) => {
    const iconDataUrl = iconMap.get(appRuleKey(app))
    return iconDataUrl ? { ...app, iconDataUrl } : app
  })
}

async function hydrateInstalledAppIcons(runId) {
  const apps = filteredInstalledApps.value
    .filter((app) => {
      const key = appRuleKey(app)
      return (
        key &&
        !app.iconDataUrl &&
        !appIconResolvedKeys.has(key) &&
        !appIconLoadingKeys.has(key)
      )
    })
    .slice(0, APP_ICON_HYDRATION_BATCH_SIZE)

  if (!apps.length || runId !== appIconHydrationRun || !appPickerOpen.value) {
    return
  }

  for (const app of apps) {
    const key = appRuleKey(app)
    if (key) {
      appIconLoadingKeys.add(key)
    }
  }

  const iconEntries = []
  let cursor = 0
  async function hydrateNextIcon() {
    while (cursor < apps.length && runId === appIconHydrationRun && appPickerOpen.value) {
      const app = apps[cursor]
      cursor += 1
      const key = appRuleKey(app)
      if (!key) {
        continue
      }

      try {
        const iconDataUrl = await loadInstalledAppIcon(app)
        if (iconDataUrl) {
          iconEntries.push([key, iconDataUrl])
        }
      } catch (error) {
        console.error('Failed to load installed app icon', error)
      } finally {
        appIconResolvedKeys.add(key)
        appIconLoadingKeys.delete(key)
      }
    }
  }

  await Promise.all(
    Array.from({ length: Math.min(APP_ICON_HYDRATION_CONCURRENCY, apps.length) }, () =>
      hydrateNextIcon(),
    ),
  )
  for (const app of apps) {
    const key = appRuleKey(app)
    if (key) {
      appIconLoadingKeys.delete(key)
    }
  }

  if (runId !== appIconHydrationRun || !appPickerOpen.value) {
    return
  }

  updateInstalledAppIcons(new Map(iconEntries))

  const hasMoreIcons = filteredInstalledApps.value.some((app) => {
    const key = appRuleKey(app)
    return key && !app.iconDataUrl && !appIconResolvedKeys.has(key)
  })
  if (runId === appIconHydrationRun && appPickerOpen.value && hasMoreIcons) {
    setTimeout(() => {
      hydrateInstalledAppIcons(runId)
    }, 80)
  }
}

function requestInstalledAppIconHydration() {
  if (!appPickerOpen.value || appPickerLoading.value || !installedApps.value.length) {
    return
  }

  const runId = ++appIconHydrationRun
  setTimeout(() => {
    hydrateInstalledAppIcons(runId)
  }, 0)
}

async function updateIgnoredAppRules(rules) {
  await props.applySettingPatch({ ignoredApps: [], ignoredAppRules: rules }, 'ignoredAppRules')
}

async function openIgnoredAppPicker() {
  appPickerOpen.value = true
  appPickerError.value = ''
  if (!installedAppPickerSupported.value || installedApps.value.length) {
    requestInstalledAppIconHydration()
    return
  }

  appPickerLoading.value = true
  try {
    installedApps.value = await listInstalledApps()
  } catch (error) {
    appPickerError.value = props.t('ignoredAppPickerLoadFailed')
    console.error('Failed to list installed apps', error)
  } finally {
    appPickerLoading.value = false
  }
  requestInstalledAppIconHydration()
}

function closeIgnoredAppPicker() {
  appPickerOpen.value = false
  appIconHydrationRun += 1
  appSearchQuery.value = ''
  appPickerError.value = ''
}

async function addIgnoredApp(app) {
  const rule = installedAppToRule(app)
  const key = appRuleKey(rule)
  if (!key || ignoredAppKeys.value.has(key)) {
    return
  }

  if (!rule.iconDataUrl) {
    try {
      rule.iconDataUrl = await loadInstalledAppIcon(rule)
      updateInstalledAppIcon(key, rule.iconDataUrl)
    } catch (error) {
      console.error('Failed to load installed app icon', error)
    }
  }

  await updateIgnoredAppRules([...ignoredAppRules.value, rule])
}

async function removeIgnoredApp(rule) {
  const key = appRuleKey(rule)
  await updateIgnoredAppRules(ignoredAppRules.value.filter((item) => appRuleKey(item) !== key))
}

async function toggleIgnoredApp(rule, enabled) {
  const key = appRuleKey(rule)
  await updateIgnoredAppRules(
    ignoredAppRules.value.map((item) =>
      appRuleKey(item) === key ? { ...item, enabled } : item,
    ),
  )
}
async function chooseLanTransferDownloadDir() {
  const selected = await open({
    directory: true,
    multiple: false,
    defaultPath: props.settings.lanTransferDownloadDir || undefined,
  })
  if (typeof selected === 'string') {
    await updateSetting('lanTransferDownloadDir', selected, 'lanTransferDownloadDir')
  }
}

async function commitMaxHistoryItems() {
  const value = Math.min(10000, Math.max(50, Number(maxHistoryItemsDraft.value) || 200))
  maxHistoryItemsDraft.value = value
  await updateSetting('maxHistoryItems', value, 'maxHistoryItems')
}

async function commitMaxHistoryDays() {
  const value = Math.min(3650, Math.max(1, Number(maxHistoryDaysDraft.value) || 30))
  maxHistoryDaysDraft.value = value
  await updateSetting('maxHistoryDays', value, 'maxHistoryDays')
}

async function commitMaxImageBytes() {
  const mb = Math.max(1, Number(maxImageBytesMbDraft.value) || 1)
  maxImageBytesMbDraft.value = Number(mb.toFixed(1))
  await updateSetting('maxImageBytes', Math.round(mb * 1_000_000), 'maxImageBytes')
}

async function updateTagLabel(color, value) {
  await props.applySettingPatch(
    {
      tagLabels: {
        ...props.settings.tagLabels,
        [color]: value,
      },
    },
    `tagLabels.${color}`,
  )
}

async function handleTagLabelChange(color, event) {
  const value =
    typeof event?.target?.value === 'string' ? event.target.value.slice(0, 5) : ''
  if (event?.target && typeof event.target.value === 'string') {
    event.target.value = value
  }
  await updateTagLabel(color, value)
}

function resolvedTagLabel(color) {
  return resolveTagLabel(color, props.settings.tagLabels, props.t)
}

function tagToneClass(color) {
  return `history-tag-${color}`
}

function findTooltipTarget(target) {
  return target instanceof Element ? target.closest('.setting-help-icon') : null
}

function showSettingTooltip(target) {
  const tooltipTarget = findTooltipTarget(target)
  const text = tooltipTarget?.getAttribute('data-tooltip')?.trim()
  if (!tooltipTarget || !text) {
    return
  }

  const rect = tooltipTarget.getBoundingClientRect()
  const tooltipWidth = Math.min(280, Math.max(160, window.innerWidth - 20))
  const left = Math.min(
    window.innerWidth - tooltipWidth / 2 - 10,
    Math.max(tooltipWidth / 2 + 10, rect.left + rect.width / 2),
  )
  const placeBelow = rect.top < 64
  tooltipState.value = {
    visible: true,
    text,
    top: placeBelow ? rect.bottom + 10 : rect.top - 10,
    left,
    placement: placeBelow ? 'bottom' : 'top',
  }
}

function hideSettingTooltip(target) {
  if (findTooltipTarget(target)) {
    tooltipState.value.visible = false
  }
}

async function clearShortcut(field) {
  props.endShortcutRecording()
  await updateSetting(field, '', field)
}

async function handleShortcutKeydown(event, field) {
  event.preventDefault()
  event.stopPropagation()

  if ((event.key === 'Tab' && !event.ctrlKey && !event.metaKey && !event.altKey) || event.key === 'Escape') {
    props.endShortcutRecording()
    return
  }

  if (event.key === 'Backspace' || event.key === 'Delete') {
    await clearShortcut(field)
    return
  }

  const parts = []
  if (event.ctrlKey) {
    parts.push('Ctrl')
  }
  if (event.altKey) {
    parts.push('Alt')
  }
  if (event.shiftKey) {
    parts.push('Shift')
  }
  if (event.metaKey) {
    parts.push(props.platformCapabilities.platform === 'macos' ? 'Command' : 'Super')
  }

  const mainKey =
    event.code === 'Backquote'
      ? '`'
      : normalizeShortcutKey(event.key, props.platformCapabilities.platform)
  if (!mainKey || ['Ctrl', 'Alt', 'Shift', 'Command', 'Super'].includes(mainKey)) {
    return
  }

  props.endShortcutRecording()
  await updateSetting(field, [...parts, mainKey].join('+'), field)
}

async function resetSettings() {
  await props.resetSettings()
}

function closeUpdateConfirm() {
  showUpdateConfirm.value = false
}

async function showLatestVersionFeedback() {
  if (updateFeedbackTimer) {
    clearTimeout(updateFeedbackTimer)
    updateFeedbackTimer = null
  }

  if (showUpdateFeedback.value) {
    showUpdateFeedback.value = false
    await nextTick()
  }

  showUpdateFeedback.value = true
  updateFeedbackTimer = window.setTimeout(() => {
    showUpdateFeedback.value = false
    updateFeedbackTimer = null
  }, 2600)
}

async function confirmInstallUpdate() {
  showUpdateConfirm.value = false
  await props.onInstallUpdate()
}

async function handleUpdateAction() {
  if (props.showUpdateAction) {
    showUpdateConfirm.value = true
    return
  }

  const nextState = await props.onCheckUpdates()

  if (nextState?.status === 'available') {
    showUpdateConfirm.value = true
    return
  }

  if (nextState?.status === 'up_to_date') {
    await showLatestVersionFeedback()
  }
}

async function clearUpdateDebugStatus() {
  await props.onClearUpdateDebugStatus()
}

async function applyUpdateDebugStatus(status) {
  await props.onSetUpdateDebugStatusWithOverrides(status, {
    latestVersion: updateDebugVersionDraft.value.trim() || undefined,
    body: updateDebugBodyDraft.value.trim() || undefined,
  })
}

async function openRepositoryUrl() {
  await openExternalUrl(ABOUT_INFO.repositoryUrl)
}

async function openLandingPageUrl() {
  await openExternalUrl(ABOUT_INFO.landingPageUrl)
}

async function handleUpdateNotesClick(event) {
  const target = event.target instanceof Element ? event.target : null
  const link = target?.closest('a')
  if (!link) {
    return
  }

  const href = link.getAttribute('href')
  if (!href) {
    return
  }

  event.preventDefault()
  await openExternalUrl(href)
}

watch(
  () => props.settings.maxHistoryItems,
  (value) => {
    maxHistoryItemsDraft.value = Number(value) || 200
  },
  { immediate: true },
)

watch(
  () => props.settings.maxHistoryDays,
  (value) => {
    maxHistoryDaysDraft.value = Number(value) || 30
  },
  { immediate: true },
)

watch(
  () => props.settings.maxImageBytes,
  (value) => {
    maxImageBytesMbDraft.value = Number(((Number(value) || 0) / 1_000_000).toFixed(1))
  },
  { immediate: true },
)

watch(
  () => [props.updateDebugStatus, updateDebugVersionValue.value, updateDebugBodyValue.value],
  ([, version, body]) => {
    updateDebugVersionDraft.value = version
    updateDebugBodyDraft.value = body
  },
  { immediate: true },
)

watch(appSearchQuery, () => {
  requestInstalledAppIconHydration()
})

onUnmounted(() => {
  appIconHydrationRun += 1
  if (updateFeedbackTimer) {
    clearTimeout(updateFeedbackTimer)
  }
})

watch(
  [categories, activeCategory],
  ([nextCategories, nextActiveCategory]) => {
    const availableKeys = new Set(nextCategories.map((category) => category.key))
    if (!availableKeys.has(nextActiveCategory)) {
      activeCategory.value = 'general'
      return
    }

    window.localStorage.setItem(SETTINGS_ACTIVE_CATEGORY_STORAGE_KEY, nextActiveCategory)
  },
  { immediate: true },
)
</script>

<template>
  <section
    class="settings-page"
    @mouseover="showSettingTooltip($event.target)"
    @focusin="showSettingTooltip($event.target)"
    @mouseout="hideSettingTooltip($event.target)"
    @focusout="hideSettingTooltip($event.target)"
  >
    <header class="settings-page-topbar">
      <button
        class="toolbar-icon-button settings-page-back"
        type="button"
        :aria-label="t('backAction')"
        :title="t('backAction')"
        @click="onBack"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M15.5 5 8.5 12l7 7"
            fill="none"
            stroke="currentColor"
            stroke-width="2.4"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
      </button>
      <div class="settings-page-title-block">
        <div class="settings-title-row">
          <h1>{{ t('settingsTitle') }}</h1>
          <button
            v-if="showUpdateAction"
            class="modal-update-badge modal-update-badge-new settings-title-update-badge"
            type="button"
            :disabled="updateBusy"
            :title="updateBadgeLabel"
            :aria-label="updateBadgeLabel"
            @click="handleUpdateAction"
          >
            <span class="modal-update-badge-mark">new</span>
          </button>
          <button
            v-else
            class="modal-update-badge modal-update-badge-check settings-title-update-badge"
            type="button"
            :disabled="updateBusy"
            :title="updateBadgeLabel"
            :aria-label="updateBadgeLabel"
            @click="handleUpdateAction"
          >
            <img :src="checkIcon" alt="" class="modal-update-badge-icon" />
          </button>
          <Transition name="update-feedback">
            <span v-if="showUpdateFeedback" class="modal-update-feedback">
              {{ t('upToDate') }}
            </span>
          </Transition>
        </div>
        <span v-if="updateHeaderMessage" class="modal-update-status settings-update-status">
          {{ updateHeaderMessage }}
        </span>
      </div>
      <div class="settings-page-actions">
        <span v-if="settingsSaveError" class="settings-save-feedback">
          {{ settingsSaveError }}
        </span>
      </div>
    </header>

    <div class="settings-layout">
      <nav class="settings-sidebar" :aria-label="t('settingsTitle')">
        <button
          v-for="category in categories"
          :key="category.key"
          type="button"
          class="settings-category-button"
          :class="{ active: activeCategory === category.key }"
          @click="activeCategory = category.key"
        >
          {{ category.label }}
        </button>
      </nav>

      <section class="settings-content">
        <div v-if="activeCategory === 'general'" class="settings-grid settings-section-grid">
          <section class="setting-card wide">
            <div class="setting-head">
              <span class="meta-label">{{ t('language') }}</span>
            </div>
            <div
              class="setting-toggle"
              role="group"
              :aria-label="t('language')"
              :style="segmentedToggleStyle(languageToggleIndex, localeOptions.length)"
            >
              <button
                v-for="option in localeOptions"
                :key="option.value"
                type="button"
                class="setting-toggle-option"
                :class="{ active: settings.locale === option.value }"
                :disabled="isPending('locale')"
                @click="updateSetting('locale', option.value, 'locale')"
              >
                {{ option.value === 'zh-CN' ? '中' : 'EN' }}
              </button>
            </div>
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="meta-label">{{ t('themeMode') }}</span>
            </div>
            <div class="custom-select" :class="{ open: openSelectKey === 'themeMode' }">
              <button
                type="button"
                class="custom-select-trigger"
                :aria-expanded="openSelectKey === 'themeMode'"
                :aria-label="t('themeMode')"
                :disabled="isPending('themeMode')"
                @click.stop="toggleSelect('themeMode')"
              >
                <span class="custom-select-value">
                  {{ selectedOptionLabel(currentThemeModeOptions, settings.themeMode) }}
                </span>
                <span class="custom-select-chevron" aria-hidden="true"></span>
              </button>
              <div v-if="openSelectKey === 'themeMode'" class="custom-select-menu" @click.stop>
                <button
                  v-for="option in currentThemeModeOptions"
                  :key="option.value"
                  type="button"
                  class="custom-select-option"
                  :class="{ active: settings.themeMode === option.value }"
                  @click="chooseSelectOption('themeMode', 'themeMode', option.value)"
                >
                  <span>{{ option.label }}</span>
                </button>
              </div>
            </div>
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="meta-label">{{ t('accentColor') }}</span>
            </div>
            <div class="custom-select" :class="{ open: openSelectKey === 'accentColor' }">
              <button
                type="button"
                class="custom-select-trigger"
                :aria-expanded="openSelectKey === 'accentColor'"
                :aria-label="t('accentColor')"
                :disabled="isPending('accentColor')"
                @click.stop="toggleSelect('accentColor')"
              >
                <span class="custom-select-value">
                  {{ selectedOptionLabel(currentAccentColorOptions, settings.accentColor) }}
                </span>
                <span class="custom-select-chevron" aria-hidden="true"></span>
              </button>
              <div v-if="openSelectKey === 'accentColor'" class="custom-select-menu" @click.stop>
                <button
                  v-for="option in currentAccentColorOptions"
                  :key="option.value"
                  type="button"
                  class="custom-select-option"
                  :class="{ active: settings.accentColor === option.value }"
                  @click="chooseSelectOption('accentColor', 'accentColor', option.value)"
                >
                  <span>{{ option.label }}</span>
                </button>
              </div>
            </div>
          </section>

          <section v-if="platformCapabilities.platform === 'windows'" class="setting-card wide window-control-style-setting">
            <div class="setting-head">
              <span class="meta-label">{{ t('windowControlStyle') }}</span>
              <span class="setting-note window-control-style-note">{{ t('windowControlStyleTip') }}</span>
            </div>
            <div
              class="setting-toggle"
              role="group"
              :aria-label="t('windowControlStyle')"
              :style="segmentedToggleStyle(windowControlStyleToggleIndex, 2)"
            >
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: settings.windowControlStyle !== 'windows' }"
                :disabled="isPending('windowControlStyle')"
                @click="updateSetting('windowControlStyle', 'traffic-lights', 'windowControlStyle')"
              >
                {{ t('windowControlTrafficLights') }}
              </button>
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: settings.windowControlStyle === 'windows' }"
                :disabled="isPending('windowControlStyle')"
                @click="updateSetting('windowControlStyle', 'windows', 'windowControlStyle')"
              >
                {{ t('windowControlWindows') }}
              </button>
            </div>
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('launchOnStartup') }}</span>
              </span>
              <span v-if="!canToggleLaunchOnStartup" class="setting-note">
                {{ t('unsupportedLaunchOnStartup') }}
              </span>
            </div>
            <div
              class="setting-toggle"
              :class="{ disabled: !canToggleLaunchOnStartup }"
              role="group"
              :aria-label="t('launchOnStartup')"
              :style="segmentedToggleStyle(launchToggleIndex, 2)"
            >
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: settings.launchOnStartup }"
                :disabled="!canToggleLaunchOnStartup || isPending('launchOnStartup')"
                @click="updateSetting('launchOnStartup', true, 'launchOnStartup')"
              >
                {{ t('toggleOn') }}
              </button>
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: !settings.launchOnStartup }"
                :disabled="!canToggleLaunchOnStartup || isPending('launchOnStartup')"
                @click="updateSetting('launchOnStartup', false, 'launchOnStartup')"
              >
                {{ t('toggleOff') }}
              </button>
            </div>
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="meta-label">{{ t('copySound') }}</span>
            </div>
            <div
              class="setting-toggle"
              role="group"
              :aria-label="t('copySound')"
              :style="segmentedToggleStyle(soundToggleIndex, 2)"
            >
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: settings.soundEnabled }"
                :disabled="isPending('soundEnabled')"
                @click="updateSetting('soundEnabled', true, 'soundEnabled')"
              >
                {{ t('toggleOn') }}
              </button>
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: !settings.soundEnabled }"
                :disabled="isPending('soundEnabled')"
                @click="updateSetting('soundEnabled', false, 'soundEnabled')"
              >
                {{ t('toggleOff') }}
              </button>
            </div>
          </section>

          <section class="setting-card wide ignored-apps-card">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('ignoredApps') }}</span>
                <span class="setting-help-icon" :data-tooltip="t('ignoredAppsTip')" :aria-label="t('ignoredAppsTip')" tabindex="0">
                  <svg viewBox="0 0 1024 1024" aria-hidden="true">
                    <path d="M512 96a416 416 0 1 0 0 832 416 416 0 0 0 0-832z m0 768a352 352 0 1 1 0-704 352 352 0 0 1 0 704z m64-160a32 32 0 0 1-32 32 64 64 0 0 1-64-64V512a32 32 0 0 1 0-64 64 64 0 0 1 64 64v160a32 32 0 0 1 32 32z m-128-368.042667a47.957333 47.957333 0 1 1 96 0 47.957333 47.957333 0 0 1-96 0z" />
                  </svg>
                </span>
              </span>
              <span v-if="!ignoredAppRules.length" class="setting-note ignored-app-empty">
                {{ t('ignoredAppsEmpty') }}
              </span>
              <span v-if="!installedAppPickerSupported" class="setting-note">
                {{ t('ignoredAppsLinuxUnsupported') }}
              </span>
            </div>
            <div class="ignored-apps-panel">
              <div v-if="ignoredAppRules.length" class="ignored-app-scroll" aria-live="polite">
                <div class="ignored-app-list">
                  <span
                    v-for="rule in ignoredAppRules"
                    :key="appRuleKey(rule)"
                    class="ignored-app-icon-wrap"
                    :class="{ disabled: !rule.enabled }"
                  >
                    <button
                      class="ignored-app-icon-button"
                      type="button"
                      :title="appSecondaryText(rule) || rule.displayName || rule.processName"
                      :aria-label="`${rule.displayName || rule.processName} ${rule.enabled ? t('toggleOff') : t('toggleOn')}`"
                      :disabled="isPending('ignoredAppRules')"
                      @click="toggleIgnoredApp(rule, !rule.enabled)"
                    >
                      <span class="app-icon-box" aria-hidden="true">
                        <img
                          v-if="rule.iconDataUrl"
                          :src="rule.iconDataUrl"
                          alt=""
                          loading="lazy"
                          decoding="async"
                        />
                        <span v-else>{{ appInitial(rule.displayName || rule.processName) }}</span>
                      </span>
                    </button>
                    <button
                      class="ignored-app-remove"
                      type="button"
                      :aria-label="`${t('removeAction')} ${rule.displayName || rule.processName}`"
                      :disabled="isPending('ignoredAppRules')"
                      @click="removeIgnoredApp(rule)"
                    >
                      <span aria-hidden="true">×</span>
                    </button>
                  </span>
                </div>
              </div>

              <div class="ignored-app-actions">
                <button
                  class="ignored-app-add-button"
                  type="button"
                  :aria-label="t('addIgnoredApp')"
                  :title="t('addIgnoredApp')"
                  :disabled="isPending('ignoredAppRules') || !installedAppPickerSupported"
                  @click="openIgnoredAppPicker"
                >
                  <svg
                    class="ignored-app-add-icon"
                    viewBox="0 0 1024 1024"
                    version="1.1"
                    xmlns="http://www.w3.org/2000/svg"
                    aria-hidden="true"
                    focusable="false"
                  >
                    <path d="M480 208h64v608h-64z" fill="currentColor"></path>
                    <path d="M208 544v-64h608v64z" fill="currentColor"></path>
                  </svg>
                </button>
              </div>
            </div>
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('resetSettings') }}</span>
              </span>
            </div>
            <button
              class="ghost settings-reset-button"
              type="button"
              :disabled="savingSettings"
              @click="resetSettings"
            >
              {{ t('resetSettings') }}
            </button>
          </section>

          <section class="setting-card wide webdav-field-card">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('tagNames') }}</span>
              </span>
            </div>
            <div class="tag-settings-list">
              <label
                v-for="color in HISTORY_TAG_COLORS"
                :key="color"
                class="tag-settings-row"
              >
                <span class="tag-settings-label">
                  <span class="history-tag-dot" :class="tagToneClass(color)"></span>
                  <span>{{ t(`tagDefaultName${color[0].toUpperCase()}${color.slice(1)}`) }}</span>
                </span>
                <input
                  :value="props.settings.tagLabels?.[color] ?? ''"
                  type="text"
                  class="tag-settings-input"
                  maxlength="5"
                  :placeholder="resolvedTagLabel(color)"
                  :disabled="isPending(`tagLabels.${color}`)"
                  @change="handleTagLabelChange(color, $event)"
                  @keydown.enter.prevent="handleTagLabelChange(color, $event)"
                />
              </label>
            </div>
          </section>

        </div>

        <div v-if="activeCategory === 'history'" class="settings-grid settings-section-grid">
          <section class="setting-card">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('copyStatsEnabled') }}</span>
                <span class="setting-help-icon" :data-tooltip="t('copyStatsEnabledTip')" :aria-label="t('copyStatsEnabledTip')" tabindex="0">
                  <svg viewBox="0 0 1024 1024" aria-hidden="true">
                    <path d="M512 96a416 416 0 1 0 0 832 416 416 0 0 0 0-832z m0 768a352 352 0 1 1 0-704 352 352 0 0 1 0 704z m64-160a32 32 0 0 1-32 32 64 64 0 0 1-64-64V512a32 32 0 0 1 0-64 64 64 0 0 1 64 64v160a32 32 0 0 1 32 32z m-128-368.042667a47.957333 47.957333 0 1 1 96 0 47.957333 47.957333 0 0 1-96 0z" />
                  </svg>
                </span>
              </span>
            </div>
            <div
              class="setting-toggle"
              role="group"
              :aria-label="t('copyStatsEnabled')"
              :style="segmentedToggleStyle(copyStatsToggleIndex, 2)"
            >
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: settings.copyStatsEnabled }"
                :disabled="isPending('copyStatsEnabled')"
                @click="updateCopyStatsEnabled(true)"
              >
                {{ t('toggleOn') }}
              </button>
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: !settings.copyStatsEnabled }"
                :disabled="isPending('copyStatsEnabled')"
                @click="updateCopyStatsEnabled(false)"
              >
                {{ t('toggleOff') }}
              </button>
            </div>
          </section>

          <section class="setting-card">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('pasteStatsEnabled') }}</span>
                <span class="setting-help-icon" :data-tooltip="t('pasteStatsEnabledTip')" :aria-label="t('pasteStatsEnabledTip')" tabindex="0">
                  <svg viewBox="0 0 1024 1024" aria-hidden="true">
                    <path d="M512 96a416 416 0 1 0 0 832 416 416 0 0 0 0-832z m0 768a352 352 0 1 1 0-704 352 352 0 0 1 0 704z m64-160a32 32 0 0 1-32 32 64 64 0 0 1-64-64V512a32 32 0 0 1 0-64 64 64 0 0 1 64 64v160a32 32 0 0 1 32 32z m-128-368.042667a47.957333 47.957333 0 1 1 96 0 47.957333 47.957333 0 0 1-96 0z" />
                  </svg>
                </span>
              </span>
            </div>
            <div
              class="setting-toggle"
              role="group"
              :aria-label="t('pasteStatsEnabled')"
              :style="segmentedToggleStyle(pasteStatsToggleIndex, 2)"
            >
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: settings.pasteStatsEnabled }"
                :disabled="isPending('pasteStatsEnabled')"
                @click="updatePasteStatsEnabled(true)"
              >
                {{ t('toggleOn') }}
              </button>
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: !settings.pasteStatsEnabled }"
                :disabled="isPending('pasteStatsEnabled')"
                @click="updatePasteStatsEnabled(false)"
              >
                {{ t('toggleOff') }}
              </button>
            </div>
          </section>

          <section class="setting-card webdav-field-card">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('maxHistoryItems') }}</span>
              </span>
            </div>
            <input
              v-model.number="maxHistoryItemsDraft"
              type="number"
              min="50"
              max="10000"
              step="50"
              :disabled="isPending('maxHistoryItems')"
              @change="commitMaxHistoryItems"
              @keydown.enter.prevent="commitMaxHistoryItems"
            />
          </section>

          <section class="setting-card webdav-field-card">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('maxHistoryDays') }}</span>
              </span>
            </div>
            <input
              v-model.number="maxHistoryDaysDraft"
              type="number"
              min="1"
              max="3650"
              step="1"
              :disabled="isPending('maxHistoryDays')"
              @change="commitMaxHistoryDays"
              @keydown.enter.prevent="commitMaxHistoryDays"
            />
          </section>

          <section class="setting-card webdav-field-card">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('maxImageBytes') }} ({{ t('megabytesShort') }})</span>
                <span class="setting-help-icon" :data-tooltip="t('maxImageBytesTip')" :aria-label="t('maxImageBytesTip')" tabindex="0">
                  <svg viewBox="0 0 1024 1024" aria-hidden="true">
                    <path d="M512 96a416 416 0 1 0 0 832 416 416 0 0 0 0-832z m0 768a352 352 0 1 1 0-704 352 352 0 0 1 0 704z m64-160a32 32 0 0 1-32 32 64 64 0 0 1-64-64V512a32 32 0 0 1 0-64 64 64 0 0 1 64 64v160a32 32 0 0 1 32 32z m-128-368.042667a47.957333 47.957333 0 1 1 96 0 47.957333 47.957333 0 0 1-96 0z" />
                  </svg>
                </span>
              </span>
              <span v-if="!hasClipboardWriteSupport" class="setting-note">
                {{ t('unsupportedClipboardWrite') }}
              </span>
            </div>
            <input
              v-model.number="maxImageBytesMbDraft"
              type="number"
              min="1"
              step="0.5"
              :disabled="isPending('maxImageBytes')"
              @change="commitMaxImageBytes"
              @keydown.enter.prevent="commitMaxImageBytes"
            />
          </section>


        </div>

        <div v-if="activeCategory === 'sync'" class="settings-grid settings-section-grid">
          <section class="setting-card">
            <div class="setting-head">
              <span class="meta-label">{{ t('webdavSyncEnabled') }}</span>
            </div>
            <div
              class="setting-toggle"
              role="group"
              :aria-label="t('webdavSyncEnabled')"
              :style="segmentedToggleStyle(webdavEnabledToggleIndex, 2)"
            >
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: settings.webdavSync?.enabled }"
                :disabled="isPending('webdavSync.enabled')"
                @click="updateWebdavSetting('enabled', true)"
              >
                {{ t('toggleOn') }}
              </button>
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: !settings.webdavSync?.enabled }"
                :disabled="isPending('webdavSync.enabled')"
                @click="updateWebdavSetting('enabled', false)"
              >
                {{ t('toggleOff') }}
              </button>
            </div>
          </section>

          <section class="setting-card">
            <div class="setting-head">
              <span class="meta-label">{{ t('webdavAutoSync') }}</span>
            </div>
            <div
              class="setting-toggle"
              role="group"
              :aria-label="t('webdavAutoSync')"
              :style="segmentedToggleStyle(webdavAutoSyncToggleIndex, 2)"
            >
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: settings.webdavSync?.autoSync }"
                :disabled="isPending('webdavSync.autoSync')"
                @click="updateWebdavSetting('autoSync', true)"
              >
                {{ t('toggleOn') }}
              </button>
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: !settings.webdavSync?.autoSync }"
                :disabled="isPending('webdavSync.autoSync')"
                @click="updateWebdavSetting('autoSync', false)"
              >
                {{ t('toggleOff') }}
              </button>
            </div>
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('webdavServerUrl') }}</span>
                <span class="setting-help-icon" :data-tooltip="t('webdavServerUrlTip')" :aria-label="t('webdavServerUrlTip')" tabindex="0">
                  <svg viewBox="0 0 1024 1024" aria-hidden="true">
                    <path d="M512 96a416 416 0 1 0 0 832 416 416 0 0 0 0-832z m0 768a352 352 0 1 1 0-704 352 352 0 0 1 0 704z m64-160a32 32 0 0 1-32 32 64 64 0 0 1-64-64V512a32 32 0 0 1 0-64 64 64 0 0 1 64 64v160a32 32 0 0 1 32 32z m-128-368.042667a47.957333 47.957333 0 1 1 96 0 47.957333 47.957333 0 0 1-96 0z" />
                  </svg>
                </span>
              </span>
            </div>
            <input
              :value="settings.webdavSync?.serverUrl"
              type="url"
              placeholder="https://example.com/dav"
              :disabled="isPending('webdavSync.serverUrl')"
              @change="updateWebdavSetting('serverUrl', $event.target.value.trim())"
              @keydown.enter.prevent="updateWebdavSetting('serverUrl', $event.target.value.trim())"
            />
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="meta-label">{{ t('webdavUsername') }}</span>
            </div>
            <input
              :value="settings.webdavSync?.username"
              type="text"
              autocomplete="username"
              :disabled="isPending('webdavSync.username')"
              @change="updateWebdavSetting('username', $event.target.value.trim())"
              @keydown.enter.prevent="updateWebdavSetting('username', $event.target.value.trim())"
            />
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="meta-label">{{ t('webdavPassword') }}</span>
              <span v-if="webdavCredentialSaved" class="setting-note webdav-password-note">
                {{ t('webdavPasswordSaved') }}
              </span>
            </div>
            <input
              :value="webdavPasswordDraft"
              type="password"
              autocomplete="current-password"
              :placeholder="webdavCredentialSaved ? t('webdavPasswordSavedPlaceholder') : t('webdavPasswordPlaceholder')"
              :disabled="isPending('webdavSync.password')"
              @change="handleWebdavPasswordChange"
              @keydown.enter.prevent="handleWebdavPasswordChange"
            />
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="meta-label">{{ t('webdavRemoteDir') }}</span>
            </div>
            <input
              :value="settings.webdavSync?.remoteDir"
              type="text"
              :disabled="isPending('webdavSync.remoteDir')"
              @change="updateWebdavSetting('remoteDir', $event.target.value.trim())"
              @keydown.enter.prevent="updateWebdavSetting('remoteDir', $event.target.value.trim())"
            />
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('webdavSyncStatus') }}</span>
              </span>
              <span class="setting-note webdav-status-note">
                {{
                  webdavSyncStatus?.error
                    ? webdavSyncStatus.error
                    : webdavSyncStatus?.lastSyncAt
                      ? t('webdavLastSyncAt', { time: webdavSyncStatus.lastSyncAt })
                      : t('webdavNeverSynced')
                }}
              </span>
            </div>
            <div class="settings-wide-control webdav-actions">
              <button
                class="ghost"
                type="button"
                :disabled="savingSettings"
                @click="runWebdavTest"
              >
                {{ t('webdavTestConnection') }}
              </button>
              <button
                class="primary"
                type="button"
                :disabled="savingSettings || !settings.webdavSync?.enabled"
                @click="runWebdavSyncNow"
              >
                {{ webdavSyncStatus?.status === 'syncing' ? t('webdavSyncing') : t('webdavSyncNow') }}
              </button>
              <button
                class="ghost"
                type="button"
                :disabled="savingSettings"
                @click="clearWebdavPassword"
              >
                {{ t('webdavClearPassword') }}
              </button>
            </div>
          </section>
        </div>

        <div v-if="activeCategory === 'transfer'" class="settings-grid settings-section-grid">
          <section class="setting-card wide">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('lanTransferDownloadDir') }}</span>
                <span class="setting-help-icon" :data-tooltip="t('lanTransferDownloadDirTip')" :aria-label="t('lanTransferDownloadDirTip')" tabindex="0">
                  <svg viewBox="0 0 1024 1024" aria-hidden="true">
                    <path d="M512 96a416 416 0 1 0 0 832 416 416 0 0 0 0-832z m0 768a352 352 0 1 1 0-704 352 352 0 0 1 0 704z m64-160a32 32 0 0 1-32 32 64 64 0 0 1-64-64V512a32 32 0 0 1 0-64 64 64 0 0 1 64 64v160a32 32 0 0 1 32 32z m-128-368.042667a47.957333 47.957333 0 1 1 96 0 47.957333 47.957333 0 0 1-96 0z" />
                  </svg>
                </span>
              </span>
            </div>
            <div class="path-picker-wrap">
              <input
                :value="settings.lanTransferDownloadDir"
                type="text"
                readonly
                :placeholder="t('lanTransferDownloadDirPlaceholder')"
              />
              <button
                class="toolbar-icon-button path-picker-button"
                type="button"
                :disabled="isPending('lanTransferDownloadDir')"
                :title="t('chooseFolder')"
                :aria-label="t('chooseFolder')"
                @click="chooseLanTransferDownloadDir"
              >
                <svg viewBox="0 0 1025 960" aria-hidden="true">
                  <path
                    d="M86.592 153.6v716.8h853.376V288H472.64L365.696 153.6h-279.04zM1.28 64h404.288L512.64 198.4H1025.28V960H1.28V64z m85.312 281.6v535.616l853.376-1.28V480H472.64L365.696 345.6h-279.04zM33.28 256h372.352L512.64 390.4H993.28a32 32 0 0 1 32 32v458.496l-1025.216 16.192 1.152-609.152A32 32 0 0 1 33.28 256z"
                    fill="currentColor"
                  />
                </svg>
              </button>
            </div>
          </section>
        </div>

        <div v-if="activeCategory === 'shortcuts'" class="settings-grid settings-section-grid">
          <section v-if="shortcutStatus?.issues?.length" class="shortcut-settings-warning">
            <p>{{ t('shortcutRegistrationFailed') }}</p>
            <button
              class="ghost compact"
              type="button"
              :disabled="shortcutRetrying"
              @click="retryShortcutRegistration"
            >
              {{ t('retryAction') }}
            </button>
          </section>
          <section class="setting-card wide">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('globalShortcut') }}</span>
              </span>
              <span v-if="shortcutIssueText('globalShortcut')" class="setting-note shortcut-warning-note">
                {{ shortcutIssueText('globalShortcut') }}
              </span>
            </div>
            <div class="shortcut-input-wrap">
              <input
                :value="settings.globalShortcut"
                type="text"
                readonly
                :disabled="isPending('globalShortcut')"
                :placeholder="recordingShortcut ? t('shortcutRecording') : t('shortcutPlaceholder')"
                @focus="beginShortcutRecording"
                @blur="endShortcutRecording"
                @keydown="handleShortcutKeydown($event, 'globalShortcut')"
              />
              <button
                v-if="settings.globalShortcut"
                type="button"
                class="shortcut-clear-button"
                :aria-label="t('clear')"
                :disabled="isPending('globalShortcut')"
                @mousedown.prevent
                @click="clearShortcut('globalShortcut')"
              >
                <span aria-hidden="true">×</span>
              </button>
            </div>
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('quickPasteShortcut') }}</span>
                <span class="setting-help-icon" :data-tooltip="t('quickPasteShortcutTip')" :aria-label="t('quickPasteShortcutTip')" tabindex="0">
                  <svg viewBox="0 0 1024 1024" aria-hidden="true">
                    <path d="M512 96a416 416 0 1 0 0 832 416 416 0 0 0 0-832z m0 768a352 352 0 1 1 0-704 352 352 0 0 1 0 704z m64-160a32 32 0 0 1-32 32 64 64 0 0 1-64-64V512a32 32 0 0 1 0-64 64 64 0 0 1 64 64v160a32 32 0 0 1 32 32z m-128-368.042667a47.957333 47.957333 0 1 1 96 0 47.957333 47.957333 0 0 1-96 0z" />
                  </svg>
                </span>
              </span>
              <span v-if="shortcutIssueText('quickPasteShortcut')" class="setting-note shortcut-warning-note">
                {{ shortcutIssueText('quickPasteShortcut') }}
              </span>
            </div>
            <div class="shortcut-input-wrap">
              <input
                :value="settings.quickPasteShortcut"
                type="text"
                readonly
                :disabled="isPending('quickPasteShortcut')"
                :placeholder="recordingShortcut ? t('shortcutRecording') : t('shortcutPlaceholder')"
                @focus="beginShortcutRecording"
                @blur="endShortcutRecording"
                @keydown="handleShortcutKeydown($event, 'quickPasteShortcut')"
              />
              <button
                v-if="settings.quickPasteShortcut"
                type="button"
                class="shortcut-clear-button"
                :aria-label="t('clear')"
                :disabled="isPending('quickPasteShortcut')"
                @mousedown.prevent
                @click="clearShortcut('quickPasteShortcut')"
              >
                <span aria-hidden="true">×</span>
              </button>
            </div>
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('searchShortcut') }}</span>
                <span class="setting-help-icon" :data-tooltip="t('searchShortcutTip')" :aria-label="t('searchShortcutTip')" tabindex="0">
                  <svg viewBox="0 0 1024 1024" aria-hidden="true">
                    <path d="M512 96a416 416 0 1 0 0 832 416 416 0 0 0 0-832z m0 768a352 352 0 1 1 0-704 352 352 0 0 1 0 704z m64-160a32 32 0 0 1-32 32 64 64 0 0 1-64-64V512a32 32 0 0 1 0-64 64 64 0 0 1 64 64v160a32 32 0 0 1 32 32z m-128-368.042667a47.957333 47.957333 0 1 1 96 0 47.957333 47.957333 0 0 1-96 0z" />
                  </svg>
                </span>
              </span>
            </div>
            <div class="shortcut-input-wrap">
              <input
                :value="settings.searchShortcut"
                type="text"
                readonly
                :disabled="isPending('searchShortcut')"
                :placeholder="recordingShortcut ? t('shortcutRecording') : t('shortcutPlaceholder')"
                @focus="beginShortcutRecording"
                @blur="endShortcutRecording"
                @keydown="handleShortcutKeydown($event, 'searchShortcut')"
              />
              <button
                v-if="settings.searchShortcut"
                type="button"
                class="shortcut-clear-button"
                :aria-label="t('clear')"
                :disabled="isPending('searchShortcut')"
                @mousedown.prevent
                @click="clearShortcut('searchShortcut')"
              >
                <span aria-hidden="true">×</span>
              </button>
            </div>
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('filterShortcut') }}</span>
                <span class="setting-help-icon" :data-tooltip="t('filterShortcutTip')" :aria-label="t('filterShortcutTip')" tabindex="0">
                  <svg viewBox="0 0 1024 1024" aria-hidden="true">
                    <path d="M512 96a416 416 0 1 0 0 832 416 416 0 0 0 0-832z m0 768a352 352 0 1 1 0-704 352 352 0 0 1 0 704z m64-160a32 32 0 0 1-32 32 64 64 0 0 1-64-64V512a32 32 0 0 1 0-64 64 64 0 0 1 64 64v160a32 32 0 0 1 32 32z m-128-368.042667a47.957333 47.957333 0 1 1 96 0 47.957333 47.957333 0 0 1-96 0z" />
                  </svg>
                </span>
              </span>
            </div>
            <div class="shortcut-input-wrap">
              <input
                :value="settings.filterShortcut"
                type="text"
                readonly
                :disabled="isPending('filterShortcut')"
                :placeholder="recordingShortcut ? t('shortcutRecording') : t('shortcutPlaceholder')"
                @focus="beginShortcutRecording"
                @blur="endShortcutRecording"
                @keydown="handleShortcutKeydown($event, 'filterShortcut')"
              />
              <button
                v-if="settings.filterShortcut"
                type="button"
                class="shortcut-clear-button"
                :aria-label="t('clear')"
                :disabled="isPending('filterShortcut')"
                @mousedown.prevent
                @click="clearShortcut('filterShortcut')"
              >
                <span aria-hidden="true">×</span>
              </button>
            </div>
          </section>
        </div>

        <div v-if="activeCategory === 'advanced'" class="settings-grid settings-section-grid">
          <section class="setting-card">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('debugMode') }}</span>
                <span class="setting-help-icon" :data-tooltip="t('debugModeTip')" :aria-label="t('debugModeTip')" tabindex="0">
                  <svg viewBox="0 0 1024 1024" aria-hidden="true">
                    <path d="M512 96a416 416 0 1 0 0 832 416 416 0 0 0 0-832z m0 768a352 352 0 1 1 0-704 352 352 0 0 1 0 704z m64-160a32 32 0 0 1-32 32 64 64 0 0 1-64-64V512a32 32 0 0 1 0-64 64 64 0 0 1 64 64v160a32 32 0 0 1 32 32z m-128-368.042667a47.957333 47.957333 0 1 1 96 0 47.957333 47.957333 0 0 1-96 0z" />
                  </svg>
                </span>
              </span>
            </div>
            <div
              class="setting-toggle"
              role="group"
              :aria-label="t('debugMode')"
              :style="segmentedToggleStyle(debugToggleIndex, 2)"
            >
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: settings.debugEnabled }"
                :disabled="isPending('debugEnabled')"
                @click="updateSetting('debugEnabled', true, 'debugEnabled')"
              >
                {{ t('toggleOn') }}
              </button>
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: !settings.debugEnabled }"
                :disabled="isPending('debugEnabled')"
                @click="updateSetting('debugEnabled', false, 'debugEnabled')"
              >
                {{ t('toggleOff') }}
              </button>
            </div>
          </section>

          <section v-if="platformCapabilities.supportsHardwareAccelerationToggle" class="setting-card">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('hardwareAcceleration') }}</span>
                <span class="setting-help-icon" :data-tooltip="t('hardwareAccelerationTip')" :aria-label="t('hardwareAccelerationTip')" tabindex="0">
                  <svg viewBox="0 0 1024 1024" aria-hidden="true">
                    <path d="M512 96a416 416 0 1 0 0 832 416 416 0 0 0 0-832z m0 768a352 352 0 1 1 0-704 352 352 0 0 1 0 704z m64-160a32 32 0 0 1-32 32 64 64 0 0 1-64-64V512a32 32 0 0 1 0-64 64 64 0 0 1 64 64v160a32 32 0 0 1 32 32z m-128-368.042667a47.957333 47.957333 0 1 1 96 0 47.957333 47.957333 0 0 1-96 0z" />
                  </svg>
                </span>
              </span>

            </div>
            <div
              class="setting-toggle"
              role="group"
              :aria-label="t('hardwareAcceleration')"
              :style="segmentedToggleStyle(hardwareAccelerationToggleIndex, 2)"
            >
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: settings.hardwareAccelerationEnabled === true }"
                :disabled="isPending('hardwareAccelerationEnabled')"
                @click="updateSetting('hardwareAccelerationEnabled', true, 'hardwareAccelerationEnabled')"
              >
                {{ t('toggleOn') }}
              </button>
              <button
                type="button"
                class="setting-toggle-option"
                :class="{ active: settings.hardwareAccelerationEnabled !== true }"
                :disabled="isPending('hardwareAccelerationEnabled')"
                @click="updateSetting('hardwareAccelerationEnabled', false, 'hardwareAccelerationEnabled')"
              >
                {{ t('toggleOff') }}
              </button>
            </div>
          </section>

          <section v-if="updateDebugEnabled" class="setting-card wide">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('updateDebugTitle') }}</span>
                <span class="setting-help-icon" :data-tooltip="t('updateDebugHint')" :aria-label="t('updateDebugHint')" tabindex="0">
                  <svg viewBox="0 0 1024 1024" aria-hidden="true">
                    <path d="M512 96a416 416 0 1 0 0 832 416 416 0 0 0 0-832z m0 768a352 352 0 1 1 0-704 352 352 0 0 1 0 704z m64-160a32 32 0 0 1-32 32 64 64 0 0 1-64-64V512a32 32 0 0 1 0-64 64 64 0 0 1 64 64v160a32 32 0 0 1 32 32z m-128-368.042667a47.957333 47.957333 0 1 1 96 0 47.957333 47.957333 0 0 1-96 0z" />
                  </svg>
                </span>
              </span>
            </div>
            <div class="settings-wide-control">
              <div class="update-debug-fields">
                <label class="update-debug-field">
                  <span class="meta-label">{{ t('updateDebugVersionLabel') }}</span>
                  <input
                    v-model="updateDebugVersionDraft"
                    type="text"
                    :placeholder="t('updateDebugVersionPlaceholder')"
                  />
                </label>
                <label class="update-debug-field">
                  <span class="meta-label">{{ t('updateDebugBodyLabel') }}</span>
                  <textarea
                    v-model="updateDebugBodyDraft"
                    class="update-debug-textarea"
                    :placeholder="t('updateDebugBodyPlaceholder')"
                  ></textarea>
                </label>
              </div>
              <div class="setting-actions">
                <button
                  v-for="option in updateDebugOptions"
                  :key="option.value"
                  type="button"
                  :class="updateDebugStatus === option.value ? 'primary' : 'ghost'"
                  @click="applyUpdateDebugStatus(option.value)"
                >
                  {{ option.label }}
                </button>
                <button class="ghost" type="button" @click="clearUpdateDebugStatus">
                  {{ t('updateDebugClear') }}
                </button>
              </div>
            </div>
          </section>
        </div>

        <div v-if="activeCategory === 'about'" class="settings-grid settings-section-grid">
          <section class="setting-card about-card wide">
            <div class="setting-head">
              <span class="meta-label">{{ t('aboutTitle') }}</span>
            </div>
            <div class="about-content">
              <button
                class="about-link"
                type="button"
                :aria-label="t('landingPageLabel')"
                :title="t('landingPageLabel')"
                @click="openLandingPageUrl"
              >
                <svg viewBox="0 0 1024 1024" aria-hidden="true" class="about-link-site-icon">
                  <path
                    d="M906.666667 512a394.666667 394.666667 0 1 0-789.333334 0 394.666667 394.666667 0 0 0 789.333334 0z m64 0c0 253.312-205.354667 458.666667-458.666667 458.666667S53.333333 765.312 53.333333 512 258.688 53.333333 512 53.333333 970.666667 258.688 970.666667 512z"
                    fill="currentColor"
                  />
                  <path
                    d="M649.770667 512c0-114.773333-18.602667-217.301333-47.488-289.877333-14.464-36.394667-30.890667-63.658667-47.488-81.322667-16.554667-17.621333-31.018667-23.466667-42.752-23.466667s-26.197333 5.845333-42.752 23.466667c-16.64 17.664-33.024 44.928-47.488 81.322667C392.917333 294.698667 374.314667 397.184 374.314667 512c0 114.773333 18.602667 217.301333 47.488 289.877333 14.464 36.394667 30.890667 63.658667 47.488 81.322667 16.554667 17.621333 31.018667 23.466667 42.752 23.466667s26.197333-5.845333 42.752-23.466667c16.64-17.664 33.024-44.928 47.488-81.322667 28.885333-72.576 47.488-175.061333 47.488-289.877333z m64 0c0 120.832-19.413333 231.68-51.968 313.557333-16.213333 40.789333-36.394667 75.946667-60.373334 101.461334-24.064 25.6-54.272 43.648-89.386666 43.648-35.114667 0-65.322667-18.048-89.386667-43.648-23.978667-25.472-44.117333-60.672-60.373333-101.461334C329.728 743.68 310.314667 632.832 310.314667 512s19.413333-231.68 51.968-313.557333c16.213333-40.789333 36.394667-75.946667 60.373333-101.504 24.064-25.557333 54.229333-43.605333 89.386667-43.605334 35.114667 0 65.322667 18.048 89.386666 43.605334 23.978667 25.514667 44.117333 60.714667 60.373334 101.546666 32.554667 81.792 51.968 192.682667 51.968 313.514667z"
                    fill="currentColor"
                  />
                  <path
                    d="M917.205333 362.666667V426.666667H106.837333V362.666667h810.368zM917.205333 618.666667V682.666667H106.837333v-64h810.368z"
                    fill="currentColor"
                  />
                </svg>
                <span>{{ t('landingPageLabel') }}</span>
              </button>
              <button
                class="about-link about-link-icon"
                type="button"
                :aria-label="t('githubRepoLabel')"
                :title="t('githubRepoLabel')"
                @click="openRepositoryUrl"
              >
                <svg viewBox="0 0 24 24" aria-hidden="true" class="about-link-github-icon">
                  <path
                    d="M12 .5C5.65.5.5 5.66.5 12.02c0 5.09 3.29 9.41 7.86 10.94.58.11.79-.25.79-.56 0-.28-.01-1.19-.02-2.15-3.2.7-3.88-1.36-3.88-1.36-.52-1.33-1.28-1.68-1.28-1.68-1.04-.72.08-.71.08-.71 1.16.08 1.77 1.19 1.77 1.19 1.02 1.77 2.69 1.26 3.35.96.11-.75.4-1.26.73-1.55-2.56-.29-5.25-1.29-5.25-5.73 0-1.26.45-2.28 1.18-3.08-.12-.29-.51-1.46.11-3.05 0 0 .97-.31 3.17 1.18a10.9 10.9 0 0 1 5.77 0c2.2-1.5 3.17-1.18 3.17-1.18.62 1.59.23 2.76.11 3.05.73.8 1.18 1.82 1.18 3.08 0 4.45-2.69 5.44-5.26 5.73.41.36.78 1.08.78 2.19 0 1.58-.01 2.85-.01 3.24 0 .31.21.68.8.56a11.53 11.53 0 0 0 7.85-10.94C23.5 5.66 18.35.5 12 .5Z"
                    fill="currentColor"
                  />
                </svg>
              </button>
            </div>
          </section>

          <section class="setting-card wide">
            <div class="setting-head">
              <span class="meta-label">{{ t('version') }}</span>
            </div>
            <span class="settings-value-text">{{ appVersion ? `v${appVersion}` : '--' }}</span>
          </section>
        </div>
      </section>
    </div>


    <div v-if="appPickerOpen" class="app-picker-backdrop" @click="closeIgnoredAppPicker">
      <section class="app-picker-dialog" @click.stop>
        <header class="app-picker-header">
          <h3>{{ t('addIgnoredApp') }}</h3>
          <button class="toolbar-icon-button" type="button" :aria-label="t('closeAction')" @click="closeIgnoredAppPicker">
            <span aria-hidden="true">×</span>
          </button>
        </header>
        <div v-if="!installedAppPickerSupported" class="app-picker-message">
          {{ t('ignoredAppsLinuxUnsupported') }}
        </div>
        <template v-else>
          <div class="search-input-wrap app-picker-search-wrap">
            <input
              v-model="appSearchQuery"
              class="app-picker-search"
              type="search"
              :placeholder="t('searchAppsPlaceholder')"
            />
            <button
              v-if="appSearchQuery"
              class="shortcut-clear-button"
              type="button"
              :title="t('clearSearch')"
              :aria-label="t('clearSearch')"
              @click="appSearchQuery = ''"
            >
              <span aria-hidden="true">×</span>
            </button>
          </div>
          <div v-if="appPickerLoading" class="app-picker-message">
            {{ t('loadingApps') }}
          </div>
          <div v-else-if="appPickerError" class="app-picker-message error">
            {{ appPickerError }}
          </div>
          <div v-else class="app-picker-list">
            <button
              v-for="app in filteredInstalledApps"
              :key="appRuleKey(app)"
              class="app-picker-item"
              type="button"
              :class="{ selected: ignoredAppKeys.has(appRuleKey(app)) }"
              :disabled="ignoredAppKeys.has(appRuleKey(app)) || isPending('ignoredAppRules')"
              @click="addIgnoredApp(app)"
            >
              <span class="app-icon-box">
                <img
                  v-if="app.iconDataUrl"
                  :src="app.iconDataUrl"
                  alt=""
                  loading="lazy"
                  decoding="async"
                />
                <span v-else>{{ appInitial(app.displayName || app.processName) }}</span>
              </span>
              <span class="app-picker-info">
                <strong>{{ app.displayName || app.processName }}</strong>
                <small>{{ appSecondaryText(app) }}</small>
              </span>
              <span class="app-picker-state">
                {{ ignoredAppKeys.has(appRuleKey(app)) ? t('ignoredAppAdded') : t('addAction') }}
              </span>
            </button>
            <div v-if="!filteredInstalledApps.length" class="app-picker-message">
              {{ t('noAppsFound') }}
            </div>
          </div>
        </template>
      </section>
    </div>

    <div v-if="showUpdateConfirm" class="update-confirm-backdrop" @click="closeUpdateConfirm">
      <section class="update-confirm-dialog" @click.stop>
        <header class="update-confirm-header">
          <div>
            <h3>{{ t('updateDetailsTitle') }}</h3>
            <p class="update-confirm-version">
              {{
                updateState.latestVersion
                  ? t('updateAvailableVersion', { version: updateState.latestVersion })
                  : t('updateAvailable')
              }}
            </p>
          </div>
        </header>
        <div
          class="update-confirm-notes"
          @click="handleUpdateNotesClick"
          v-html="updateNotesHtml"
        ></div>
        <footer class="update-confirm-actions">
          <button class="ghost" type="button" @click="closeUpdateConfirm">
            {{ t('ignoreUpdate') }}
          </button>
          <button class="primary" type="button" :disabled="updateBusy" @click="confirmInstallUpdate">
            {{ t('installUpdateNow') }}
          </button>
        </footer>
      </section>
    </div>

    <div
      v-if="tooltipState.visible"
      class="setting-help-tooltip"
      :class="`placement-${tooltipState.placement}`"
      :style="tooltipStyle"
      role="tooltip"
    >
      {{ tooltipState.text }}
    </div>
  </section>
</template>
