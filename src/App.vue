<script setup>
import { cursorPosition, getCurrentWindow } from "@tauri-apps/api/window";
import { computed, defineAsyncComponent, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
    onCopySound,
    onHistoryUpdated,
    onOpenSettings,
    onPanelShown,
    onQuickPasteStarted,
    onShortcutStatusUpdated,
    onUpdateStatus,
    onWebdavSyncStatus,
} from "./services/tauriApi";
import SearchBar from "./components/SearchBar.vue";
import FilterTabs from "./components/FilterTabs.vue";
import HistoryList from "./components/HistoryList.vue";
import EditModal from "./components/EditModal.vue";
import ConfirmModal from "./components/ConfirmModal.vue";
import { useSettings } from "./composables/useSettings";
import { useUpdater } from "./composables/useUpdater";
import {
    useHistory,
} from "./composables/useHistory";
import {
    flushPendingCopySound,
    playCopySoundFallback,
} from "./composables/useCopySound";
import { useTheme } from "./composables/useTheme";
import { useKeyboardShortcuts } from "./composables/useKeyboardShortcuts";
import { useLanReceiver } from "./composables/useLanReceiver";
import { useWindowSize } from "./composables/useWindowSize";

const settingsState = useSettings();
const updaterState = useUpdater({ t: settingsState.t });
const lanReceiverState = useLanReceiver({ t: settingsState.t });
const route = useRoute();
const router = useRouter();
const historyState = useHistory({
    platformCapabilities: settingsState.platformCapabilities,
    settings: settingsState.settings,
    t: settingsState.t,
});
const LanTransferView = defineAsyncComponent(() => import("./views/LanTransferView.vue"));
const SettingsView = defineAsyncComponent(() => import("./views/SettingsView.vue"));
const quickPasteActive = ref(false);
const isWindowMaximized = ref(false);

useTheme({
    currentThemeMode: settingsState.currentThemeMode,
    currentAccentColor: settingsState.currentAccentColor,
});

// 根据路由自动调整窗口尺寸
useWindowSize(route);

const { handleWindowAction } = useKeyboardShortcuts({
    closeSelect: settingsState.closeSelect,
    copyItem: historyState.copyItem,
    activeFilterTab: historyState.activeFilterTab,
    filteredHistory: historyState.filteredHistory,
    historyTabs: historyState.historyTabs,
    openSelectKey: settingsState.openSelectKey,
    pasteItem: historyState.pasteItem,
    selectedId: historyState.selectedId,
    setSelectedId: historyState.setSelectedId,
    settings: settingsState.settings,
    showEditModal: historyState.showEditModal,
    isHomeRoute: computed(() => route.name === "home"),
    clearEditing: () => {
        historyState.showEditModal.value = false;
        historyState.editingItemId.value = null;
    },
    quickPasteActive,
    commitQuickPaste,
    cancelQuickPaste,
});

watch(settingsState.currentLocale, (locale) => {
    document.documentElement.lang = locale;
});

watch(
    () => [route.name, settingsState.platformCapabilities.value.platform],
    ([routeName, platform]) => {
        void syncTaskbarVisibilityForRoute(routeName, platform);
    },
    { immediate: true },
);

let unlistenHistory = null;
let unlistenCopySound = null;
let unlistenUpdate = null;
let unlistenWebdavSync = null;
let unlistenWindowFocus = null;
let unlistenWindowResize = null;
let unlistenQuickPaste = null;
let unlistenOpenSettings = null;
let unlistenPanelShown = null;
let unlistenShortcutStatus = null;
const startupBusy = ref(false);
const isLanTransferRoute = computed(() => route.name === "lanTransfer");
const isSettingsRoute = computed(() => route.name === "settings");
const windowControlPlatform = computed(
    () => settingsState.platformCapabilities.value.platform,
);
const confirmDialogState = ref({
    cancelLabel: "",
    confirmLabel: "",
    message: "",
    onConfirm: null,
    show: false,
    title: "",
});
const directPasteUnavailableMessage = computed(() =>
    settingsState.directPasteUnavailableMessage(
        settingsState.platformCapabilities.value,
        settingsState.t,
    ),
);

async function syncTaskbarVisibilityForRoute(routeName, platform) {
    if (platform !== "windows") {
        return;
    }

    try {
        await getCurrentWindow().setSkipTaskbar(routeName !== "settings");
    } catch (error) {
        console.error("Failed to update taskbar visibility", error);
    }
}

function cleanupListeners() {
    unlistenHistory?.();
    unlistenCopySound?.();
    unlistenUpdate?.();
    unlistenWebdavSync?.();
    unlistenWindowFocus?.();
    unlistenWindowResize?.();
    unlistenQuickPaste?.();
    unlistenOpenSettings?.();
    unlistenPanelShown?.();
    unlistenShortcutStatus?.();
    unlistenHistory = null;
    unlistenCopySound = null;
    unlistenUpdate = null;
    unlistenWebdavSync = null;
    unlistenWindowFocus = null;
    unlistenWindowResize = null;
    unlistenQuickPaste = null;
    unlistenOpenSettings = null;
    unlistenPanelShown = null;
    unlistenShortcutStatus = null;
}

function playCapturedCopySound() {
    if (!settingsState.settings.soundEnabled) {
        return;
    }

    playCopySoundFallback();
}

function flushCopySoundIfEnabled() {
    if (!settingsState.settings.soundEnabled) {
        return;
    }

    flushPendingCopySound();
}

function blurSearchIfFocused() {
    const searchInput = document.getElementById("history-search");
    if (document.activeElement === searchInput) {
        searchInput.blur();
    }
}

async function isCursorInsidePanel() {
    const window = getCurrentWindow();
    const [cursor, pos, size] = await Promise.all([
        cursorPosition(),
        window.outerPosition(),
        window.outerSize(),
    ]);
    return (
        cursor.x >= pos.x &&
        cursor.x < pos.x + size.width &&
        cursor.y >= pos.y &&
        cursor.y < pos.y + size.height
    );
}

async function hideHomePanelAfterBlur() {
    if (route.name !== 'home') {
        return
    }

    const window = getCurrentWindow();

    let cursorInside;
    try {
        cursorInside = await isCursorInsidePanel();
    } catch {
        // 查询失败时保守起见不隐藏，避免把拖动误当成点击外部。
        cursorInside = true;
    }

    // 光标在窗口内：可能是正在拖动窗口（拖拽会触发一次“失焦→重新获得
    // 焦点”，且光标始终在窗口内），此时不应隐藏。只有光标明确位于窗口外
    // （点击了其他窗口）才隐藏。
    if (cursorInside) {
        return
    }

    try {
        await window.hide()
    } catch (error) {
        console.error('Failed to hide the main panel after blur', error)
    }
}

async function syncWindowMaximized() {
    isWindowMaximized.value = await getCurrentWindow().isMaximized();
}

async function toggleWindowMaximized() {
    await handleWindowAction("maximize");
    await syncWindowMaximized();
}

// 主面板隐藏后再次打开时，恢复到初始状态：第一个 tab、清空搜索框、
// 清空标签筛选与选中项，并关闭编辑弹窗。
function resetPanelTransientState() {
    historyState.query.value = "";
    historyState.activeFilterTab.value =
        historyState.historyTabs.value[0]?.key ?? "all";
    historyState.activeTagFilter.value = "";
    historyState.setSelectedId(null);
    historyState.showEditModal.value = false;
    historyState.editingItemId.value = null;
    historyState.editDraft.value = "";
}

function handleDocumentVisibilityChange() {
    if (document.visibilityState === "visible") {
        flushCopySoundIfEnabled();
        return;
    }

    blurSearchIfFocused();
}

function handleUserInteractionForSound() {
    flushCopySoundIfEnabled();
}

function advanceQuickPasteSelection() {
    const items = historyState.filteredHistory.value;
    if (!items.length) {
        historyState.setSelectedId(null);
        return;
    }

    const currentIndex = items.findIndex(
        (item) => item.id === historyState.selectedId.value,
    );
    const nextIndex = currentIndex === -1 ? 0 : (currentIndex + 1) % items.length;
    historyState.setSelectedId(items[nextIndex].id);
}

function cancelQuickPaste() {
    quickPasteActive.value = false;
}

async function commitQuickPaste() {
    if (!quickPasteActive.value) {
        return;
    }

    quickPasteActive.value = false;
    if (historyState.selectedId.value) {
        await historyState.pasteItem(historyState.selectedId.value);
    }
}

async function startQuickPasteMode() {
    if (quickPasteActive.value) {
        return;
    }

    quickPasteActive.value = true;
    if (route.name !== "home") {
        await router.push({ name: "home" });
    }
    historyState.showEditModal.value = false;
    historyState.editingItemId.value = null;
    await nextTick();

    const items = historyState.filteredHistory.value;
    if (items.length && !items.some((item) => item.id === historyState.selectedId.value)) {
        historyState.setSelectedId(items[0].id);
    }
}

async function initializeApp() {
    startupBusy.value = true;
    settingsState.clearStartupError();
    cleanupListeners();

    try {
        await settingsState.loadAppVersion();
        await settingsState.loadPlatformCapabilities();
        await settingsState.refreshSettings();
        await settingsState.loadShortcutStatus();
        await updaterState.refreshUpdateState();
        await settingsState.refreshWebdavSyncState();
        await historyState.refreshHistory();
        document.documentElement.lang = settingsState.currentLocale.value;
        unlistenHistory = await onHistoryUpdated(async (event) => {
            if (event?.payload?.id) {
                historyState.applyHistoryUpdate(event.payload);
                return;
            }
            await historyState.refreshHistory();
        });
        unlistenCopySound = await onCopySound(() => {
            playCapturedCopySound();
        });
        unlistenUpdate = await onUpdateStatus((event) => {
            if (event?.payload) {
                updaterState.applyUpdateState(event.payload);
            }
        });
        unlistenWebdavSync = await onWebdavSyncStatus((event) => {
            if (event?.payload) {
                settingsState.applyWebdavSyncStatus(event.payload);
            }
        });
        unlistenQuickPaste = await onQuickPasteStarted(() => {
            if (quickPasteActive.value) {
                advanceQuickPasteSelection();
                return;
            }
            void startQuickPasteMode();
        });
        unlistenOpenSettings = await onOpenSettings(() => {
            void openSettingsRoute();
        });
        unlistenPanelShown = await onPanelShown(() => {
            resetPanelTransientState();
            if (route.name !== "home") {
                void router.push({ name: "home" });
            }
        });
        unlistenShortcutStatus = await onShortcutStatusUpdated((event) => {
            if (event?.payload) {
                settingsState.applyShortcutStatus(event.payload);
            }
        });
        unlistenWindowFocus = await getCurrentWindow().onFocusChanged(
            ({ payload }) => {
                if (payload) {
                    flushCopySoundIfEnabled();
                    historyState.refreshRelativeTimes();
                    return;
                }

                blurSearchIfFocused();
                void hideHomePanelAfterBlur()
            },
        );
        unlistenWindowResize = await getCurrentWindow().onResized(() => {
            void syncWindowMaximized();
        });
        await syncWindowMaximized();
    } catch (error) {
        settingsState.setStartupError(error);
    } finally {
        startupBusy.value = false;
    }
}

onMounted(async () => {
    document.addEventListener("visibilitychange", handleDocumentVisibilityChange);
    document.addEventListener("pointerdown", handleUserInteractionForSound, true);
    document.addEventListener("keydown", handleUserInteractionForSound, true);
    await initializeApp();
});

onUnmounted(() => {
    document.removeEventListener("visibilitychange", handleDocumentVisibilityChange);
    document.removeEventListener("pointerdown", handleUserInteractionForSound, true);
    document.removeEventListener("keydown", handleUserInteractionForSound, true);
    cleanupListeners();
});

async function openLanTransferRoute() {
    await router.push({ name: "lanTransfer" });
}

async function openSettingsRoute() {
    await router.push({ name: "settings" });
}

async function leaveLanTransferRoute() {
    await router.push({ name: "home" });
}

function closeConfirmDialog() {
    confirmDialogState.value = {
        cancelLabel: "",
        confirmLabel: "",
        message: "",
        onConfirm: null,
        show: false,
        title: "",
    };
}

function openConfirmDialog({
    cancelLabel,
    confirmLabel,
    message,
    onConfirm,
    title,
}) {
    confirmDialogState.value = {
        cancelLabel,
        confirmLabel,
        message,
        onConfirm,
        show: true,
        title,
    };
}

async function confirmDialogAction() {
    const action = confirmDialogState.value.onConfirm;
    closeConfirmDialog();
    if (typeof action === "function") {
        await action();
    }
}

function openClearHistoryConfirm() {
    openConfirmDialog({
        cancelLabel: settingsState.t("cancelAction"),
        confirmLabel: settingsState.t("clear"),
        message: settingsState.t("clearHistoryConfirm"),
        onConfirm: historyState.clearHistory,
        title: settingsState.t("clear"),
    });
}

function openResetSettingsConfirm() {
    openConfirmDialog({
        cancelLabel: settingsState.t("cancelAction"),
        confirmLabel: settingsState.t("resetSettings"),
        message: settingsState.t("resetSettingsConfirm"),
        onConfirm: settingsState.resetVisibleSettings,
        title: settingsState.t("resetSettings"),
    });
}
</script>

<template>
    <div
        class="app-shell"
        :data-density="settingsState.currentDensity.value"
        :data-platform="settingsState.platformCapabilities.value.platform"
    >
        <section class="titlebar-row">
            <div
                v-if="isSettingsRoute"
                class="window-controls"
                :class="windowControlPlatform"
            >
                <template v-if="windowControlPlatform === 'macos'">
                <button
                    class="traffic-light close"
                    type="button"
                    :aria-label="settingsState.t('closeAction')"
                    @click="handleWindowAction('close')"
                >
                    <span class="traffic-light-icon" aria-hidden="true">
                        <svg
                            class="traffic-light-svg traffic-light-svg-default"
                            viewBox="0 0 1024 1024"
                        >
                            <path
                                d="M512.298624 1.023829a511.488085 511.488085 0 1 0 0 1022.976171 511.488085 511.488085 0 0 0 0-1022.976171z"
                                fill="#F55656"
                            />
                        </svg>
                        <svg
                            class="traffic-light-svg traffic-light-svg-active"
                            viewBox="0 0 1024 1024"
                        >
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
                    :aria-label="settingsState.t('minimizeAction')"
                    @click="handleWindowAction('minimize')"
                >
                    <span class="traffic-light-icon" aria-hidden="true">
                        <svg
                            class="traffic-light-svg traffic-light-svg-default"
                            viewBox="0 0 1024 1024"
                        >
                            <path
                                d="M512.298624 1.023829a511.488085 511.488085 0 1 0 0 1022.976171 511.488085 511.488085 0 0 0 0-1022.976171z"
                                fill="#FFBF2F"
                            />
                        </svg>
                        <svg
                            class="traffic-light-svg traffic-light-svg-active"
                            viewBox="0 0 1024 1024"
                        >
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
                    :aria-label="settingsState.t('maximizeAction')"
                    @click="handleWindowAction('maximize')"
                >
                    <span class="traffic-light-icon" aria-hidden="true">
                        <svg
                            class="traffic-light-svg traffic-light-svg-default"
                            viewBox="0 0 1024 1024"
                        >
                            <path
                                d="M511.573333 0a511.573333 511.573333 0 1 0 0 1023.146667 511.573333 511.573333 0 0 0 0-1023.146667z"
                                fill="#44C69D"
                            />
                        </svg>
                        <svg
                            class="traffic-light-svg traffic-light-svg-active"
                            viewBox="0 0 1024 1024"
                        >
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
                </template>
                <template v-else-if="windowControlPlatform === 'windows'">
                    <button
                        class="window-control minimize"
                        type="button"
                        :aria-label="settingsState.t('minimizeAction')"
                        :title="settingsState.t('minimizeAction')"
                        @click="handleWindowAction('minimize')"
                    >
                        <svg class="window-control-icon" viewBox="0 0 12 12" aria-hidden="true">
                            <path d="M1.5 6h9" />
                        </svg>
                    </button>
                    <button
                        class="window-control maximize"
                        type="button"
                        :aria-label="settingsState.t(isWindowMaximized ? 'restoreAction' : 'maximizeAction')"
                        :title="settingsState.t(isWindowMaximized ? 'restoreAction' : 'maximizeAction')"
                        @click="toggleWindowMaximized"
                    >
                        <svg
                            v-if="isWindowMaximized"
                            class="window-control-icon"
                            viewBox="0 0 12 12"
                            aria-hidden="true"
                        >
                            <path d="M3.5 4.5h5v5h-5z M5 3h4v4" />
                        </svg>
                        <svg v-else class="window-control-icon" viewBox="0 0 12 12" aria-hidden="true">
                            <path d="M2.5 2.5h7v7h-7z" />
                        </svg>
                    </button>
                    <button
                        class="window-control close"
                        type="button"
                        :aria-label="settingsState.t('closeAction')"
                        :title="settingsState.t('closeAction')"
                        @click="handleWindowAction('close')"
                    >
                        <svg class="window-control-icon" viewBox="0 0 12 12" aria-hidden="true">
                            <path d="M2 2l8 8M10 2l-8 8" />
                        </svg>
                    </button>
                </template>
                <template v-else>
                    <button
                        class="window-control linux-control minimize"
                        type="button"
                        :aria-label="settingsState.t('minimizeAction')"
                        :title="settingsState.t('minimizeAction')"
                        @click="handleWindowAction('minimize')"
                    >
                        <svg class="window-control-icon" viewBox="0 0 12 12" aria-hidden="true">
                            <path d="M2.5 8.75h7" />
                        </svg>
                    </button>
                    <button
                        class="window-control linux-control maximize"
                        type="button"
                        :aria-label="settingsState.t(isWindowMaximized ? 'restoreAction' : 'maximizeAction')"
                        :title="settingsState.t(isWindowMaximized ? 'restoreAction' : 'maximizeAction')"
                        @click="toggleWindowMaximized"
                    >
                        <svg
                            v-if="isWindowMaximized"
                            class="window-control-icon"
                            viewBox="0 0 12 12"
                            aria-hidden="true"
                        >
                            <path d="M3.25 4.25h4.5v4.5h-4.5z M4.75 2.75h4.5v4.5" />
                        </svg>
                        <svg v-else class="window-control-icon" viewBox="0 0 12 12" aria-hidden="true">
                            <path d="M3 3h6v6H3z" />
                        </svg>
                    </button>
                    <button
                        class="window-control linux-control close"
                        type="button"
                        :aria-label="settingsState.t('closeAction')"
                        :title="settingsState.t('closeAction')"
                        @click="handleWindowAction('close')"
                    >
                        <svg class="window-control-icon" viewBox="0 0 12 12" aria-hidden="true">
                            <path d="m3.25 3.25 5.5 5.5m0-5.5-5.5 5.5" />
                        </svg>
                    </button>
                </template>
            </div>
            <div
                class="titlebar-dragger"
                data-tauri-drag-region
                @dblclick="toggleWindowMaximized"
            ></div>
        </section>

        <div class="window-shell">
            <div
                v-if="settingsState.startupError.value"
                class="startup-error-panel"
            >
                <div class="startup-error-state">
                    <strong>{{ settingsState.t("startupLoadFailed") }}</strong>
                    <p>{{ settingsState.startupError.value }}</p>
                    <button
                        class="primary"
                        type="button"
                        :disabled="startupBusy"
                        @click="initializeApp"
                    >
                        {{ settingsState.t("retryAction") }}
                    </button>
                </div>
            </div>

            <template v-else-if="isLanTransferRoute">
                <LanTransferView
                    :busy="lanReceiverState.lanReceiverBusy.value"
                    :error="lanReceiverState.lanReceiverError.value"
                    :on-back="leaveLanTransferRoute"
                    :on-start="lanReceiverState.openLanReceiver"
                    :on-send-file="lanReceiverState.sendDesktopFile"
                    :on-send-text="lanReceiverState.sendDesktopText"
                    :on-open-file="lanReceiverState.openTransferFile"
                    :on-reveal-file="lanReceiverState.revealTransferFile"
                    :state="lanReceiverState.lanReceiverState.value"
                    :status-label="lanReceiverState.statusLabel.value"
                    :t="settingsState.t"
                />
            </template>

            <template v-else-if="isSettingsRoute">
                <SettingsView
                    :app-version="settingsState.appVersion.value"
                    :apply-setting-patch="settingsState.applySettingPatch"
                    :apply-webdav-sync-patch="settingsState.applyWebdavSyncPatch"
                    :begin-shortcut-recording="settingsState.beginShortcutRecording"
                    :clear-webdav-password="settingsState.clearWebdavPassword"
                    :close-select="settingsState.closeSelect"
                    :current-accent-color-options="
                        settingsState.currentAccentColorOptions.value
                    "
                    :current-locale="settingsState.currentLocale.value"
                    :current-theme-mode-options="
                        settingsState.currentThemeModeOptions.value
                    "
                    :can-toggle-launch-on-startup="
                        settingsState.canToggleLaunchOnStartup.value
                    "
                    :end-shortcut-recording="settingsState.endShortcutRecording"
                    :locale-options="settingsState.localeOptions"
                    :on-check-updates="updaterState.runUpdateCheck"
                    :on-clear-update-debug-status="updaterState.clearUpdateDebugStatus"
                    :on-install-update="updaterState.runUpdateInstall"
                    :on-set-update-debug-status-with-overrides="
                        updaterState.setUpdateDebugStatusWithOverrides
                    "
                    :open-select-key="settingsState.openSelectKey.value"
                    :pending-setting-key="settingsState.pendingSettingKey.value"
                    :recording-shortcut="settingsState.recordingShortcut.value"
                    :reset-settings="openResetSettingsConfirm"
                    :retry-shortcut-registration="settingsState.retryShortcutRegistration"
                    :run-webdav-sync-now="settingsState.runWebdavSyncNow"
                    :run-webdav-test="settingsState.runWebdavTest"
                    :save-webdav-password="settingsState.saveWebdavPassword"
                    :saving-settings="settingsState.savingSettings.value"
                    :segmented-toggle-style="settingsState.segmentedToggleStyle"
                    :selected-option-label="settingsState.selectedOptionLabel"
                    :settings="settingsState.settings"
                    :settings-save-error="settingsState.settingsSaveError.value"
                    :shortcut-retrying="settingsState.shortcutRetrying.value"
                    :shortcut-status="settingsState.shortcutStatus.value"
                    :show-update-action="updaterState.canInstallUpdate.value"
                    :platform-capabilities="settingsState.platformCapabilities.value"
                    :t="settingsState.t"
                    :toggle-select="settingsState.toggleSelect"
                    :update-debug-enabled="updaterState.updateDebugEnabled"
                    :update-debug-status="updaterState.updateDebugStatus.value"
                    :update-busy="updaterState.updateBusy.value"
                    :update-label="settingsState.t('downloadAndInstall')"
                    :update-status-message="updaterState.statusMessage.value"
                    :update-state="updaterState.updateState.value"
                    :webdav-credential-saved="settingsState.webdavCredentialSaved.value"
                    :webdav-password-draft="settingsState.webdavPasswordDraft.value"
                    :webdav-sync-status="settingsState.webdavSyncStatus.value"
                />
            </template>

            <template v-else>
                <SearchBar
                    :action-feedback="historyState.actionFeedback.value"
                    :clear-label="settingsState.t('clear')"
                    :clear-search-label="settingsState.t('clearSearch')"
                    :on-clear="openClearHistoryConfirm"
                    :on-clear-query="
                        () => {
                            historyState.query.value = '';
                        }
                    "
                    :on-open-settings="
                        () => {
                            openSettingsRoute();
                        }
                    "
                    :on-open-lan-receiver="openLanTransferRoute"
                    :on-window-action="handleWindowAction"
                    :placeholder="settingsState.t('searchPlaceholder')"
                    :query="historyState.query.value"
                    :settings-label="settingsState.t('settingsTitle')"
                    :lan-receiver-label="settingsState.t('lanReceiverTitle')"
                    @update:query="
                        historyState.query.value = $event;
                    "
                />

                <section
                    v-if="settingsState.hasShortcutIssues.value"
                    class="shortcut-warning-banner"
                >
                    <p>{{ settingsState.shortcutWarningMessage.value }}</p>
                    <div class="shortcut-warning-actions">
                        <button
                            class="ghost compact"
                            type="button"
                            :disabled="settingsState.shortcutRetrying.value"
                            @click="settingsState.retryShortcutRegistration"
                        >
                            {{ settingsState.t("retryAction") }}
                        </button>
                        <button
                            class="primary compact"
                            type="button"
                            @click="openSettingsRoute"
                        >
                            {{ settingsState.t("settingsTitle") }}
                        </button>
                    </div>
                </section>

                <FilterTabs
                    :active-filter-tab="historyState.activeFilterTab.value"
                    :active-tag-filter="historyState.activeTagFilter.value"
                    :aria-label="settingsState.t('searchPlaceholder')"
                    :tabs="historyState.historyTabs.value"
                    :tag-filters="historyState.availableTagFilters.value"
                    :tag-label-prefix="settingsState.t('historyTags')"
                    @select="historyState.activeFilterTab.value = $event"
                    @select-tag="
                        historyState.activeTagFilter.value =
                            historyState.activeTagFilter.value === $event ? '' : $event
                    "
                />

                <section class="history-region">
                    <HistoryList
                        :can-clipboard-write="
                            settingsState.platformCapabilities.value
                                .supportsTextWrite ||
                            settingsState.platformCapabilities.value
                                .supportsHtmlWrite ||
                            settingsState.platformCapabilities.value
                                .supportsImageWrite
                        "
                        :can-direct-paste="
                            settingsState.platformCapabilities.value
                                .supportsDirectPaste
                        "
                        :copy-stats-enabled="settingsState.settings.copyStatsEnabled"
                        :paste-stats-enabled="settingsState.settings.pasteStatsEnabled"
                        :history-panel-ref="historyState.historyPanelRef"
                        :has-more="historyState.hasMoreHistory.value"
                        :items="historyState.filteredHistory.value"
                        :loading="historyState.loading.value"
                        :loading-more="historyState.loadingMore.value"
                        :locale="settingsState.currentLocale.value"
                        :relative-time-version="
                            historyState.relativeTimeVersion.value
                        "
                        :selected-id="historyState.selectedId.value"
                        :tag-label-map="settingsState.settings.tagLabels"
                        :t="settingsState.t"
                        :unsupported-clipboard-write-message="
                            settingsState.t('unsupportedClipboardWrite')
                        "
                        :unsupported-direct-paste-message="
                            directPasteUnavailableMessage
                        "
                        @copy="historyState.copyItem"
                        @edit="historyState.openEditModal"
                        @load-more="historyState.loadMoreHistory"
                        @open-link="historyState.openExternalUrl"
                        @paste="historyState.pasteItem"
                        @remove="historyState.removeItem"
                        @select="historyState.setSelectedId"
                        @toggle-pin="historyState.togglePin"
                        @update-tags="historyState.updateTags($event.id, $event.tagColors)"
                    />
                </section>

                <div class="history-count-bar">
                    {{ historyState.historyCountLabel.value }}
                </div>
            </template>
        </div>

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
        <ConfirmModal
            :cancel-label="confirmDialogState.cancelLabel"
            :confirm-label="confirmDialogState.confirmLabel"
            :message="confirmDialogState.message"
            :show="confirmDialogState.show"
            :title="confirmDialogState.title"
            @close="closeConfirmDialog"
            @confirm="confirmDialogAction"
        />
    </div>
</template>
