<script setup>
import { onMounted, onUnmounted, watch } from "vue";
import { onHistoryUpdated } from "./services/tauriApi";
import SearchBar from "./components/SearchBar.vue";
import FilterTabs from "./components/FilterTabs.vue";
import HistoryList from "./components/HistoryList.vue";
import SettingsModal from "./components/SettingsModal.vue";
import EditModal from "./components/EditModal.vue";
import { useSettings } from "./composables/useSettings";
import { useHistory } from "./composables/useHistory";
import { useTheme } from "./composables/useTheme";
import { useKeyboardShortcuts } from "./composables/useKeyboardShortcuts";

const settingsState = useSettings();
const historyState = useHistory({
  platformCapabilities: settingsState.platformCapabilities,
  settings: settingsState.settings,
  t: settingsState.t,
});

useTheme({
  currentThemeMode: settingsState.currentThemeMode,
  currentAccentColor: settingsState.currentAccentColor,
});

const { handleWindowAction } = useKeyboardShortcuts({
  closeSelect: settingsState.closeSelect,
  copyItem: historyState.copyItem,
  filteredHistory: historyState.filteredHistory,
  openSelectKey: settingsState.openSelectKey,
  pasteItem: historyState.pasteItem,
  selectedId: historyState.selectedId,
  setSelectedId: historyState.setSelectedId,
  settings: settingsState.settings,
  showEditModal: historyState.showEditModal,
  showSettings: settingsState.showSettings,
  clearEditing: () => {
    historyState.showEditModal.value = false;
    historyState.editingItemId.value = null;
  },
});

watch(settingsState.currentLocale, (locale) => {
  document.documentElement.lang = locale;
});

let unlisten = null;

onMounted(async () => {
  await settingsState.loadAppVersion();
  await settingsState.loadPlatformCapabilities();
  await settingsState.refreshSettings();
  await historyState.refreshHistory();
  document.documentElement.lang = settingsState.currentLocale.value;
  unlisten = await onHistoryUpdated(async () => {
    await historyState.refreshHistory();
  });
});

onUnmounted(() => {
  unlisten?.();
});
</script>

<template>
  <div class="app-shell" :data-density="settingsState.currentDensity.value">
    <section class="titlebar-row">
      <div class="window-controls">
        <button
          class="traffic-light close"
          type="button"
          title="Close"
          @click="handleWindowAction('close')"
        />
        <button
          class="traffic-light minimize"
          type="button"
          title="Minimize"
          @click="handleWindowAction('minimize')"
        />
        <button
          class="traffic-light maximize"
          type="button"
          title="Maximize"
          @click="handleWindowAction('maximize')"
        />
      </div>
      <div class="titlebar-dragger" data-tauri-drag-region></div>
    </section>

    <div class="window-shell">
      <SearchBar
        :action-feedback="historyState.actionFeedback.value"
        :clear-label="settingsState.t('clear')"
        :on-clear="historyState.clearHistory"
        :on-open-settings="() => { settingsState.showSettings.value = true; }"
        :on-window-action="handleWindowAction"
        :placeholder="settingsState.t('searchPlaceholder')"
        :query="historyState.query.value"
        :settings-label="settingsState.t('settingsTitle')"
        @update:query="
          historyState.query.value = $event;
          historyState.refreshHistory();
        "
      />

      <FilterTabs
        :active-filter-tab="historyState.activeFilterTab.value"
        :aria-label="settingsState.t('searchPlaceholder')"
        :tabs="historyState.historyTabs.value"
        @select="historyState.activeFilterTab.value = $event"
      />

      <HistoryList
        :can-clipboard-write="settingsState.platformCapabilities.value.supportsClipboardWrite"
        :can-direct-paste="settingsState.platformCapabilities.value.supportsDirectPaste"
        :history-count-label="historyState.historyCountLabel.value"
        :history-panel-ref="historyState.historyPanelRef"
        :items="historyState.filteredHistory.value"
        :loading="historyState.loading.value"
        :locale="settingsState.currentLocale.value"
        :selected-id="historyState.selectedId.value"
        :t="settingsState.t"
        :unsupported-clipboard-write-message="settingsState.t('unsupportedClipboardWrite')"
        :unsupported-direct-paste-message="settingsState.t('unsupportedDirectPaste')"
        @copy="historyState.copyItem"
        @edit="historyState.openEditModal"
        @paste="historyState.pasteItem"
        @remove="historyState.removeItem"
        @select="historyState.setSelectedId"
        @toggle-pin="historyState.togglePin"
      />
    </div>

    <SettingsModal
      :app-version="settingsState.appVersion.value"
      :begin-shortcut-recording="settingsState.beginShortcutRecording"
      :choose-select-option="settingsState.chooseSelectOption"
      :close-select="settingsState.closeSelect"
      :current-accent-color-options="settingsState.currentAccentColorOptions.value"
      :current-locale="settingsState.currentLocale.value"
      :current-theme-mode-options="settingsState.currentThemeModeOptions.value"
      :can-toggle-launch-on-startup="settingsState.canToggleLaunchOnStartup.value"
      :debug-toggle-index="settingsState.debugToggleIndex.value"
      :end-shortcut-recording="settingsState.endShortcutRecording"
      :handle-shortcut-keydown="settingsState.handleShortcutKeydown"
      :language-toggle-index="settingsState.languageToggleIndex.value"
      :launch-toggle-index="settingsState.launchToggleIndex.value"
      :locale-options="settingsState.localeOptions"
      :max-image-bytes-mb="settingsState.maxImageBytesMb.value"
      :on-update-max-image-bytes-mb="settingsState.setMaxImageBytesMb"
      :open-select-key="settingsState.openSelectKey.value"
      :recording-shortcut="settingsState.recordingShortcut.value"
      :save-settings="() => settingsState.saveSettings()"
      :saving-settings="settingsState.savingSettings.value"
      :segmented-toggle-style="settingsState.segmentedToggleStyle"
      :selected-option-label="settingsState.selectedOptionLabel"
      :settings="settingsState.settings"
      :settings-save-error="settingsState.settingsSaveError.value"
      :show-settings="settingsState.showSettings.value"
      :platform-capabilities="settingsState.platformCapabilities.value"
      :t="settingsState.t"
      :toggle-select="settingsState.toggleSelect"
      @close="settingsState.showSettings.value = false"
    />

    <EditModal
      :draft="historyState.editDraft.value"
      :show="historyState.showEditModal.value"
      :t="settingsState.t"
      @close="
        historyState.showEditModal.value = false;
        historyState.editingItemId.value = null;
      "
      @save="historyState.saveEditedItem"
      @update:draft="historyState.editDraft.value = $event"
    />
  </div>
</template>
