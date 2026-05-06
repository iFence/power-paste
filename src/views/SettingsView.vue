<script setup>
import { computed, nextTick, onUnmounted, ref, watch } from 'vue'
import { open } from '@tauri-apps/plugin-dialog'
import DOMPurify from 'dompurify'
import { marked } from 'marked'
import { openExternalUrl } from '../services/tauriApi'
import { normalizeShortcutKey } from '../utils/shortcut'
import { HISTORY_TAG_COLORS, resolveTagLabel } from '../utils/constants'
import checkIcon from '../assets/check.svg'

const ABOUT_INFO = {
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
  beginShortcutRecording: { type: Function, required: true },
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
  savingSettings: { type: Boolean, required: true },
  settings: { type: Object, required: true },
  settingsSaveError: { type: String, required: true },
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
})

const activeCategory = ref(window.localStorage.getItem(SETTINGS_ACTIVE_CATEGORY_STORAGE_KEY) || 'general')
const showUpdateConfirm = ref(false)
const showUpdateFeedback = ref(false)
const updateDebugVersionDraft = ref('')
const updateDebugBodyDraft = ref('')
const maxHistoryItemsDraft = ref(200)
const maxHistoryDaysDraft = ref(30)
const maxImageBytesMbDraft = ref(6)
let updateFeedbackTimer = null

const categories = computed(() => [
  { key: 'general', label: props.t('settingsCategoryGeneral') },
  { key: 'history', label: props.t('settingsCategoryHistory') },
  { key: 'transfer', label: props.t('settingsCategoryTransfer') },
  { key: 'shortcuts', label: props.t('settingsCategoryShortcuts') },
  { key: 'advanced', label: props.t('settingsCategoryAdvanced') },
  { key: 'about', label: props.t('settingsCategoryAbout') },
])
const languageToggleIndex = computed(() =>
  Math.max(props.localeOptions.findIndex((option) => option.value === props.settings.locale), 0),
)
const debugToggleIndex = computed(() => (props.settings.debugEnabled ? 0 : 1))
const soundToggleIndex = computed(() => (props.settings.soundEnabled ? 0 : 1))
const launchToggleIndex = computed(() => (props.settings.launchOnStartup ? 0 : 1))
const hasClipboardWriteSupport = computed(
  () =>
    props.platformCapabilities.supportsTextWrite ||
    props.platformCapabilities.supportsHtmlWrite ||
    props.platformCapabilities.supportsImageWrite,
)

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
  if (!props.updateState || props.updateState.status !== 'downloading') {
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

function isPending(key) {
  return props.savingSettings && (!key || props.pendingSettingKey === key)
}

async function updateSetting(field, value, key = field) {
  if (props.settings[field] === value) {
    return
  }

  await props.applySettingPatch({ [field]: value }, key)
}

async function chooseSelectOption(key, field, value) {
  props.closeSelect()
  await updateSetting(field, value, key)
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

async function clearGlobalShortcut() {
  props.endShortcutRecording()
  await updateSetting('globalShortcut', '', 'globalShortcut')
}

async function handleShortcutKeydown(event) {
  event.preventDefault()
  event.stopPropagation()

  if (event.key === 'Tab' || event.key === 'Escape') {
    props.endShortcutRecording()
    return
  }

  if (event.key === 'Backspace' || event.key === 'Delete') {
    await clearGlobalShortcut()
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

  const mainKey = normalizeShortcutKey(event.key, props.platformCapabilities.platform)
  if (!mainKey || ['Ctrl', 'Alt', 'Shift', 'Command', 'Super'].includes(mainKey)) {
    return
  }

  props.endShortcutRecording()
  await updateSetting('globalShortcut', [...parts, mainKey].join('+'), 'globalShortcut')
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

  await props.onCheckUpdates()

  if (props.updateState?.status === 'up_to_date') {
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

onUnmounted(() => {
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
  <section class="settings-page">
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
        <span v-if="updateHeaderMessage">{{ updateHeaderMessage }}</span>
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
          <section class="setting-card">
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

          <section class="setting-card">
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

          <section class="setting-card">
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

          <section class="setting-card">
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

          <section class="setting-card">
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

          <section class="setting-card">
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

          <section class="setting-card wide">
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

          <section class="setting-card">
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

          <section class="setting-card">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('maxImageBytes') }} ({{ t('megabytesShort') }})</span>
                <span class="setting-help-icon" :title="t('maxImageBytesTip')" tabindex="0">?</span>
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

        <div v-if="activeCategory === 'transfer'" class="settings-grid settings-section-grid">
          <section class="setting-card wide">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('lanTransferDownloadDir') }}</span>
                <span class="setting-help-icon" :title="t('lanTransferDownloadDirTip')" tabindex="0">?</span>
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
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path
                    d="M3.5 7.5h6l1.7 2h9.3v8a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2v-10Zm0 2h17"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="1.8"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                  />
                </svg>
              </button>
            </div>
          </section>
        </div>

        <div v-if="activeCategory === 'shortcuts'" class="settings-grid settings-section-grid">
          <section class="setting-card wide">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('globalShortcut') }}</span>
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
                @keydown="handleShortcutKeydown"
              />
              <button
                v-if="settings.globalShortcut"
                type="button"
                class="shortcut-clear-button"
                :aria-label="t('clear')"
                :disabled="isPending('globalShortcut')"
                @mousedown.prevent
                @click="clearGlobalShortcut"
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
                <span class="setting-help-icon" :title="t('debugModeTip')" tabindex="0">?</span>
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

          <section v-if="updateDebugEnabled" class="setting-card wide">
            <div class="setting-head">
              <span class="setting-label-row">
                <span class="meta-label">{{ t('updateDebugTitle') }}</span>
                <span class="setting-help-icon" :title="t('updateDebugHint')" tabindex="0">?</span>
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
            <span class="settings-value-text">{{ appVersion || '--' }}</span>
          </section>
        </div>
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
  </section>
</template>
