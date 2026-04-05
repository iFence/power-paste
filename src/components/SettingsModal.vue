<script setup>
defineProps({
  appVersion: { type: String, required: true },
  beginShortcutRecording: { type: Function, required: true },
  canToggleLaunchOnStartup: { type: Boolean, required: true },
  canInstallUpdate: { type: Boolean, required: true },
  checkForUpdates: { type: Function, required: true },
  chooseSelectOption: { type: Function, required: true },
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
  openSelectKey: { type: String, default: null },
  platformCapabilities: { type: Object, required: true },
  recordingShortcut: { type: Boolean, required: true },
  saveSettings: { type: Function, required: true },
  savingSettings: { type: Boolean, required: true },
  segmentedToggleStyle: { type: Function, required: true },
  selectedOptionLabel: { type: Function, required: true },
  settings: { type: Object, required: true },
  settingsSaveError: { type: String, required: true },
  showSettings: { type: Boolean, required: true },
  t: { type: Function, required: true },
  toggleSelect: { type: Function, required: true },
  installUpdate: { type: Function, required: true },
  updateBusy: { type: Boolean, required: true },
  updateState: { type: Object, required: true },
  updateStatusMessage: { type: String, required: true },
});

const emit = defineEmits(["close"]);
</script>

<template>
  <div v-if="showSettings" class="modal-backdrop" @click="emit('close')">
    <section class="settings-modal" @click.stop>
      <header class="modal-header">
        <div>
          <h2>{{ t("settingsTitle") }}</h2>
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
              {{ option.value === "zh-CN" ? "ZH" : "EN" }}
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
                <span
                  v-if="settings.themeMode === option.value"
                  class="custom-select-check"
                  aria-hidden="true"
                >OK</span>
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
                <span
                  v-if="settings.accentColor === option.value"
                  class="custom-select-check"
                  aria-hidden="true"
                >OK</span>
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
          <input
            :value="settings.globalShortcut"
            type="text"
            readonly
            :placeholder="recordingShortcut ? t('shortcutRecording') : t('shortcutPlaceholder')"
            @focus="beginShortcutRecording"
            @blur="endShortcutRecording"
            @keydown="handleShortcutKeydown"
          />
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

        <section class="setting-card wide update-card">
          <div class="setting-head">
            <span class="meta-label">{{ t("checkForUpdates") }}</span>
            <span class="setting-note">
              {{ t("currentVersionLabel", { version: updateState.currentVersion || appVersion || "--" }) }}
            </span>
            <span class="setting-note">{{ updateStatusMessage }}</span>
            <span v-if="updateState.latestVersion" class="setting-note">
              {{ t("latestVersionLabel", { version: updateState.latestVersion }) }}
            </span>
          </div>
          <div class="setting-actions">
            <button class="ghost" type="button" :disabled="updateBusy" @click="checkForUpdates">
              {{ t("checkForUpdates") }}
            </button>
            <button
              class="primary"
              type="button"
              :disabled="updateBusy || !canInstallUpdate"
              @click="installUpdate"
            >
              {{ t("downloadAndInstall") }}
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
    </section>
  </div>
</template>
