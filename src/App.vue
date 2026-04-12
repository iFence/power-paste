<script setup>
import { onMounted, onUnmounted, ref, watch } from "vue";
import { onHistoryUpdated, onUpdateStatus } from "./services/tauriApi";
import SearchBar from "./components/SearchBar.vue";
import FilterTabs from "./components/FilterTabs.vue";
import HistoryList from "./components/HistoryList.vue";
import SettingsModal from "./components/SettingsModal.vue";
import EditModal from "./components/EditModal.vue";
import { useSettings } from "./composables/useSettings";
import { useUpdater } from "./composables/useUpdater";
import { useHistory } from "./composables/useHistory";
import { useTheme } from "./composables/useTheme";
import { useKeyboardShortcuts } from "./composables/useKeyboardShortcuts";

const settingsState = useSettings();
const updaterState = useUpdater({ t: settingsState.t });
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

let unlistenHistory = null;
let unlistenUpdate = null;
const startupBusy = ref(false);

function cleanupListeners() {
  unlistenHistory?.();
  unlistenUpdate?.();
  unlistenHistory = null;
  unlistenUpdate = null;
}

async function initializeApp() {
  startupBusy.value = true;
  settingsState.clearStartupError();
  cleanupListeners();

  try {
    await settingsState.loadAppVersion();
    await settingsState.loadPlatformCapabilities();
    await settingsState.refreshSettings();
    await updaterState.refreshUpdateState();
    await historyState.refreshHistory();
    document.documentElement.lang = settingsState.currentLocale.value;
    unlistenHistory = await onHistoryUpdated(async (event) => {
      if (event?.payload?.id) {
        historyState.applyHistoryUpdate(event.payload);
        return;
      }
      await historyState.refreshHistory();
    });
    unlistenUpdate = await onUpdateStatus((event) => {
      if (event?.payload) {
        updaterState.applyUpdateState(event.payload);
      }
    });
  } catch (error) {
    settingsState.setStartupError(error);
  } finally {
    startupBusy.value = false;
  }
}

onMounted(async () => {
  await initializeApp();
});

onUnmounted(() => {
  cleanupListeners();
});
</script>

<template>
  <div
    class="app-shell"
    :data-density="settingsState.currentDensity.value"
    :data-platform="settingsState.platformCapabilities.value.platform"
  >
    <section class="titlebar-row">
      <div class="window-controls">
        <button
          class="traffic-light close"
          type="button"
          aria-label="Close"
          @click="handleWindowAction('close')"
        >
          <span class="traffic-light-icon" aria-hidden="true">
            <svg class="traffic-light-svg traffic-light-svg-default" viewBox="0 0 1024 1024">
              <path
                d="M512.298624 1.023829a511.488085 511.488085 0 1 0 0 1022.976171 511.488085 511.488085 0 0 0 0-1022.976171z"
                fill="#F55656"
              />
            </svg>
            <svg class="traffic-light-svg traffic-light-svg-active" viewBox="0 0 1024 1024">
              <path
                d="M512.298624 1.023829a511.488085 511.488085 0 1 0 0 1022.976171 511.488085 511.488085 0 0 0 0-1022.976171z"
                fill="#F55656"
              />
              <path
                d="M567.158814 511.914681l189.152475 188.299283c15.186802 15.35744 15.528079 39.929345 0.682552 55.628062a37.796367 37.796367 0 0 1-54.348275 0.853191L511.957348 566.774871 321.269129 756.780537a37.796367 37.796367 0 0 1-54.348275-0.767872 40.099983 40.099983 0 0 1 0.767872-55.713382L456.755881 511.914681 267.688726 323.615397a40.099983 40.099983 0 0 1-0.767872-55.542742 37.796367 37.796367 0 0 1 54.348275-0.853192L511.957348 456.969172 702.645566 267.048825a37.796367 37.796367 0 0 1 54.433595 0.767872 40.099983 40.099983 0 0 1-0.853192 55.713381L567.244133 511.914681z"
                fill="#2A2F3C"
              />
            </svg>
          </span>
        </button>
        <button
          class="traffic-light minimize"
          type="button"
          aria-label="Minimize"
          @click="handleWindowAction('minimize')"
        >
          <span class="traffic-light-icon" aria-hidden="true">
            <svg class="traffic-light-svg traffic-light-svg-default" viewBox="0 0 1024 1024">
              <path
                d="M512.298624 1.023829a511.488085 511.488085 0 1 0 0 1022.976171 511.488085 511.488085 0 0 0 0-1022.976171z"
                fill="#FFBF2F"
              />
            </svg>
            <svg class="traffic-light-svg traffic-light-svg-active" viewBox="0 0 1024 1024">
              <path
                d="M511.573333 0a511.573333 511.573333 0 1 0 0 1023.146667 511.573333 511.573333 0 0 0 0-1023.146667z"
                fill="#FFBF2F"
              />
              <path
                d="M170.666667 469.333333m46.933333 0l588.8 0q46.933333 0 46.933333 46.933334l0 0q0 46.933333-46.933333 46.933333l-588.8 0q-46.933333 0-46.933333-46.933333l0 0q0-46.933333 46.933333-46.933334Z"
                fill="#2A2F3C"
              />
            </svg>
          </span>
        </button>
        <button
          class="traffic-light maximize"
          type="button"
          aria-label="Maximize"
          @click="handleWindowAction('maximize')"
        >
          <span class="traffic-light-icon" aria-hidden="true">
            <svg class="traffic-light-svg traffic-light-svg-default" viewBox="0 0 1024 1024">
              <path
                d="M511.573333 0a511.573333 511.573333 0 1 0 0 1023.146667 511.573333 511.573333 0 0 0 0-1023.146667z"
                fill="#44C69D"
              />
            </svg>
            <svg class="traffic-light-svg traffic-light-svg-active" viewBox="0 0 1024 1024">
              <path
                d="M511.573333 0a511.573333 511.573333 0 1 0 0 1023.146667 511.573333 511.573333 0 0 0 0-1023.146667z"
                fill="#44C69D"
              />
              <path
                d="M253.184 560.384c17.237333 0 31.232 14.08 31.232 31.317333v114.602667l165.802667-165.802667a31.317333 31.317333 0 1 1 44.202666 44.202667L331.093333 748.117333h109.738667a31.317333 31.317333 0 1 1 0 62.549334h-187.733333a31.317333 31.317333 0 0 1-31.232-31.317334V591.701333c0-17.237333 13.994667-31.232 31.317333-31.232zM589.653333 213.333333h187.733334c17.237333 0 31.232 13.994667 31.232 31.317334v187.648a31.317333 31.317333 0 0 1-62.549334 0V317.610667L580.266667 483.498667a31.317333 31.317333 0 0 1-44.202667-44.202667l163.328-163.413333H589.653333a31.317333 31.317333 0 1 1 0-62.549334z"
                fill="#2A2F3C"
              />
            </svg>
          </span>
        </button>
      </div>
      <div class="titlebar-dragger" data-tauri-drag-region></div>
    </section>

    <div class="window-shell">
      <div v-if="settingsState.startupError.value" class="startup-error-panel">
        <div class="startup-error-state">
          <strong>{{ settingsState.t("startupLoadFailed") }}</strong>
          <p>{{ settingsState.startupError.value }}</p>
          <button class="primary" type="button" :disabled="startupBusy" @click="initializeApp">
            {{ settingsState.t("retryAction") }}
          </button>
        </div>
      </div>

      <template v-else>
        <SearchBar
          :action-feedback="historyState.actionFeedback.value"
          :clear-label="settingsState.t('clear')"
          :clear-search-label="settingsState.t('clearSearch')"
          :on-clear="historyState.clearHistory"
          :on-clear-query="() => {
            historyState.query.value = '';
            historyState.refreshHistory();
          }"
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

        <section class="history-region">
          <HistoryList
            :can-clipboard-write="
              settingsState.platformCapabilities.value.supportsTextWrite ||
              settingsState.platformCapabilities.value.supportsHtmlWrite ||
              settingsState.platformCapabilities.value.supportsImageWrite
            "
            :can-direct-paste="settingsState.platformCapabilities.value.supportsDirectPaste"
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
            @open-link="historyState.openExternalUrl"
            @paste="historyState.pasteItem"
            @remove="historyState.removeItem"
            @select="historyState.setSelectedId"
            @toggle-pin="historyState.togglePin"
          />
          <div class="history-count-bar">{{ historyState.historyCountLabel.value }}</div>
        </section>
      </template>
    </div>

    <SettingsModal
      v-if="!settingsState.startupError.value"
      :app-version="settingsState.appVersion.value"
      :begin-shortcut-recording="settingsState.beginShortcutRecording"
      :choose-select-option="settingsState.chooseSelectOption"
      :clear-global-shortcut="settingsState.clearGlobalShortcut"
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
      :on-check-updates="updaterState.runUpdateCheck"
      :on-clear-update-debug-status="updaterState.clearUpdateDebugStatus"
      :on-install-update="updaterState.runUpdateInstall"
      :on-set-update-debug-status="updaterState.setUpdateDebugStatus"
      :on-update-max-image-bytes-mb="settingsState.setMaxImageBytesMb"
      :open-select-key="settingsState.openSelectKey.value"
      :recording-shortcut="settingsState.recordingShortcut.value"
      :save-settings="() => settingsState.saveSettings()"
      :saving-settings="settingsState.savingSettings.value"
      :show-update-action="updaterState.canInstallUpdate.value"
      :segmented-toggle-style="settingsState.segmentedToggleStyle"
      :selected-option-label="settingsState.selectedOptionLabel"
      :settings="settingsState.settings"
      :settings-save-error="settingsState.settingsSaveError.value"
      :show-settings="settingsState.showSettings.value"
      :platform-capabilities="settingsState.platformCapabilities.value"
      :t="settingsState.t"
      :toggle-select="settingsState.toggleSelect"
      :update-debug-enabled="updaterState.updateDebugEnabled"
      :update-debug-status="updaterState.updateDebugStatus.value"
      :update-busy="updaterState.updateBusy.value"
      :update-label="settingsState.t('downloadAndInstall')"
      :update-status-message="updaterState.statusMessage.value"
      :update-state="updaterState.updateState.value"
      @close="settingsState.showSettings.value = false"
    />

    <EditModal
      v-if="!settingsState.startupError.value"
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
