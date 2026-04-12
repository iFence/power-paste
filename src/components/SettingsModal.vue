<script setup>
import { computed, nextTick, onUnmounted, ref } from 'vue'
import checkIcon from '../assets/check.svg'

const props = defineProps({
  appVersion: { type: String, required: true },
  beginShortcutRecording: { type: Function, required: true },
  canToggleLaunchOnStartup: { type: Boolean, required: true },
  chooseSelectOption: { type: Function, required: true },
  clearGlobalShortcut: { type: Function, required: true },
  closeSelect: { type: Function, required: true },
  currentAccentColorOptions: { type: Array, required: true },
  currentLocale: { type: String, required: true },
  currentThemeModeOptions: { type: Array, required: true },
  debugToggleIndex: { type: Number, required: true },
  endShortcutRecording: { type: Function, required: true },
  handleShortcutKeydown: { type: Function, required: true },
  languageToggleIndex: { type: Number, required: true },
  launchToggleIndex: { type: Number, required: true },
  localeOptions: { type: Array, required: true },
  maxImageBytesMb: { type: Number, required: true },
  onUpdateMaxImageBytesMb: { type: Function, required: true },
  onCheckUpdates: { type: Function, required: true },
  onClearUpdateDebugStatus: { type: Function, required: true },
  onInstallUpdate: { type: Function, required: true },
  onSetUpdateDebugStatus: { type: Function, required: true },
  openSelectKey: { type: String, default: null },
  platformCapabilities: { type: Object, required: true },
  recordingShortcut: { type: Boolean, required: true },
  saveSettings: { type: Function, required: true },
  savingSettings: { type: Boolean, required: true },
  showUpdateAction: { type: Boolean, required: true },
  segmentedToggleStyle: { type: Function, required: true },
  selectedOptionLabel: { type: Function, required: true },
  settings: { type: Object, required: true },
  settingsSaveError: { type: String, required: true },
  showSettings: { type: Boolean, required: true },
  t: { type: Function, required: true },
  toggleSelect: { type: Function, required: true },
  updateDebugEnabled: { type: Boolean, required: true },
  updateDebugStatus: { type: String, default: null },
  updateBusy: { type: Boolean, required: true },
  updateLabel: { type: String, required: true },
  updateStatusMessage: { type: String, required: true },
  updateState: { type: Object, required: true },
})

const emit = defineEmits(['close'])
const showUpdateConfirm = ref(false)
const showUpdateFeedback = ref(false)
let updateFeedbackTimer = null

const updateNotes = computed(() => {
  const body = props.updateState?.body
  if (typeof body !== 'string' || !body.trim()) {
    return props.t('updateNotesEmpty')
  }

  return body.trim()
})

const updateDebugOptions = computed(() => [
  { value: 'available', label: props.t('updateDebugAvailable') },
  { value: 'downloading', label: props.t('updateDebugDownloading') },
  { value: 'downloaded', label: props.t('updateDebugDownloaded') },
  { value: 'up_to_date', label: props.t('updateDebugUpToDate') },
  { value: 'error', label: props.t('updateDebugError') },
])

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

async function selectUpdateDebugStatus(status) {
  await props.onSetUpdateDebugStatus(status)
}

async function clearUpdateDebugStatus() {
  await props.onClearUpdateDebugStatus()
}

onUnmounted(() => {
  if (updateFeedbackTimer) {
    clearTimeout(updateFeedbackTimer)
  }
})
</script>

<template>
  <div v-if="showSettings" class="modal-backdrop" @click="emit('close')">
    <section class="settings-modal" @click.stop>
      <header class="modal-header">
        <div class="modal-title-block">
          <div class="modal-title-row">
            <h2>{{ t("settingsTitle") }}</h2>
            <button
              v-if="showUpdateAction"
              class="modal-update-badge modal-update-badge-new"
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
              class="modal-update-badge modal-update-badge-check"
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
            <span v-if="updateHeaderMessage" class="modal-update-status">
              {{ updateHeaderMessage }}
            </span>
          </div>
          <span class="modal-version">{{ t("version") }} {{ appVersion || "--" }}</span>
        </div>
      </header>

      <div class="settings-grid">
        <section class="setting-card">
          <div class="setting-head">
            <span class="meta-label">{{ t("language") }}</span>
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
              @click="settings.locale = option.value"
            >
              {{ option.value === "zh-CN" ? "中" : "EN" }}
            </button>
          </div>
        </section>

        <section class="setting-card">
          <div class="setting-head">
            <span class="meta-label">{{ t("themeMode") }}</span>
          </div>
          <div class="custom-select" :class="{ open: openSelectKey === 'themeMode' }">
            <button
              type="button"
              class="custom-select-trigger"
              :aria-expanded="openSelectKey === 'themeMode'"
              :aria-label="t('themeMode')"
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
            <span class="meta-label">{{ t("accentColor") }}</span>
          </div>
          <div class="custom-select" :class="{ open: openSelectKey === 'accentColor' }">
            <button
              type="button"
              class="custom-select-trigger"
              :aria-expanded="openSelectKey === 'accentColor'"
              :aria-label="t('accentColor')"
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
            <span class="meta-label">{{ t("launchOnStartup") }}</span>
            <span v-if="!canToggleLaunchOnStartup" class="setting-note">
              {{ t("unsupportedLaunchOnStartup") }}
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
              :disabled="!canToggleLaunchOnStartup"
              @click="settings.launchOnStartup = true"
            >
              {{ t("toggleOn") }}
            </button>
            <button
              type="button"
              class="setting-toggle-option"
              :class="{ active: !settings.launchOnStartup }"
              :disabled="!canToggleLaunchOnStartup"
              @click="settings.launchOnStartup = false"
            >
              {{ t("toggleOff") }}
            </button>
          </div>
        </section>

        <section class="setting-card">
          <div class="setting-head">
            <span class="meta-label">{{ t("maxHistoryItems") }}</span>
          </div>
          <input v-model.number="settings.maxHistoryItems" type="number" min="50" max="2000" step="50" />
        </section>

        <section class="setting-card">
          <div class="setting-head">
            <span class="meta-label">{{ t("maxImageBytes") }} ({{ t("megabytesShort") }})</span>
            <span
              v-if="!(
                platformCapabilities.supportsTextWrite ||
                platformCapabilities.supportsHtmlWrite ||
                platformCapabilities.supportsImageWrite
              )"
              class="setting-note"
            >
              {{ t("unsupportedClipboardWrite") }}
            </span>
          </div>
          <input
            :value="maxImageBytesMb"
            type="number"
            min="1"
            step="0.5"
            @input="onUpdateMaxImageBytesMb($event.target.value)"
          />
        </section>

        <section class="setting-card wide">
          <div class="setting-head">
            <span class="meta-label">{{ t("globalShortcut") }}</span>
          </div>
          <div class="shortcut-input-wrap">
            <input
              :value="settings.globalShortcut"
              type="text"
              readonly
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
              @mousedown.prevent
              @click="clearGlobalShortcut"
            >
              <span aria-hidden="true">×</span>
            </button>
          </div>
        </section>

        <section class="setting-card">
          <div class="setting-head">
            <span class="meta-label">{{ t("debugMode") }}</span>
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
              @click="settings.debugEnabled = true"
            >
              {{ t("toggleOn") }}
            </button>
            <button
              type="button"
              class="setting-toggle-option"
              :class="{ active: !settings.debugEnabled }"
              @click="settings.debugEnabled = false"
            >
              {{ t("toggleOff") }}
            </button>
          </div>
        </section>

        <section v-if="updateDebugEnabled" class="setting-card wide">
          <div class="setting-head">
            <span class="meta-label">{{ t("updateDebugTitle") }}</span>
            <span class="setting-note">{{ t("updateDebugHint") }}</span>
          </div>
          <div class="setting-actions">
            <button
              v-for="option in updateDebugOptions"
              :key="option.value"
              type="button"
              :class="updateDebugStatus === option.value ? 'primary' : 'ghost'"
              @click="selectUpdateDebugStatus(option.value)"
            >
              {{ option.label }}
            </button>
            <button class="ghost" type="button" @click="clearUpdateDebugStatus">
              {{ t("updateDebugClear") }}
            </button>
          </div>
        </section>
      </div>

      <footer class="modal-footer">
        <span v-if="settingsSaveError" class="settings-save-feedback">{{ settingsSaveError }}</span>
        <button class="primary" type="button" :disabled="savingSettings" @click="saveSettings">
          {{ t("saveChanges") }}
        </button>
      </footer>

      <div v-if="showUpdateConfirm" class="update-confirm-backdrop" @click="closeUpdateConfirm">
        <section class="update-confirm-dialog" @click.stop>
          <header class="update-confirm-header">
            <div>
              <h3>{{ t("updateDetailsTitle") }}</h3>
              <p class="update-confirm-version">
                {{ updateState.latestVersion ? t("updateAvailableVersion", { version: updateState.latestVersion }) : t("updateAvailable") }}
              </p>
            </div>
          </header>
          <pre class="update-confirm-notes">{{ updateNotes }}</pre>
          <footer class="update-confirm-actions">
            <button class="ghost" type="button" @click="closeUpdateConfirm">
              {{ t("ignoreUpdate") }}
            </button>
            <button class="primary" type="button" :disabled="updateBusy" @click="confirmInstallUpdate">
              {{ t("installUpdateNow") }}
            </button>
          </footer>
        </section>
      </div>
    </section>
  </div>
</template>
