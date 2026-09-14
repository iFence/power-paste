<script setup>
// 发送主页面：选择待发送内容并点击附近设备发起传输。
import { computed, ref } from "vue";
import LanSubnetPicker from "../../components/LanSubnetPicker.vue";

const props = defineProps({
  busy: { type: Boolean, default: false },
  devices: { type: Array, default: () => [] },
  lastSubnet: { type: String, default: "" },
  running: { type: Boolean, required: true },
  scanRunning: { type: Boolean, default: false },
  selection: { type: Array, default: () => [] },
  subnetItems: { type: Array, default: () => [] },
  subnetMenuOpen: { type: Boolean, default: false },
  t: { type: Function, required: true },
  transfers: { type: Array, default: () => [] },
});

const emit = defineEmits([
  "add-device",
  "cancel-transfer",
  "clear-selection",
  "close-subnet-menu",
  "edit-text",
  "open-link",
  "pick-clipboard",
  "pick-files",
  "pick-folder",
  "pick-text",
  "remove-item",
  "scan",
  "select-subnet",
  "send-device",
]);

const manualOpen = ref(false);
const manualAddress = ref("");
const showAddMenu = ref(false);

const fileItems = computed(() =>
  props.selection.filter((item) => item.kind === "file"),
);
const textItem = computed(() =>
  props.selection.find((item) => item.kind === "text"),
);
const totalBytes = computed(() =>
  fileItems.value.reduce((sum, item) => sum + Number(item.size || 0), 0),
);
const activeSend = computed(() =>
  props.transfers.find(
    (transfer) => transfer.status === "active" && transfer.direction === "send",
  ),
);

function formatBytes(size) {
  const value = Number(size || 0);
  if (value < 1000) {
    return `${value} B`;
  }
  if (value < 1_000_000) {
    return `${Math.round(value / 1000)} KB`;
  }
  if (value < 1_000_000_000) {
    return `${(value / 1_000_000).toFixed(1)} MB`;
  }
  return `${(value / 1_000_000_000).toFixed(2)} GB`;
}

function progressOf(transfer) {
  const total = Number(transfer.totalBytes || 0);
  if (!total) {
    return 0;
  }
  return Math.max(
    0,
    Math.min(100, Math.round((Number(transfer.doneBytes || 0) / total) * 100)),
  );
}

function itemLabel(item) {
  if (item.kind === "text") {
    return item.text.replace(/\s+/g, " ").trim();
  }
  return item.name;
}

function itemMeta(item) {
  if (item.kind === "text") {
    return props.t("lanTransferTextMessage");
  }
  return `${formatBytes(item.size)} · ${item.mimeType || "file"}`;
}

function deviceTypeLabel(device) {
  const type = String(device.deviceType || "").toLowerCase();
  if (type.includes("phone")) {
    return "phone";
  }
  if (type.includes("tablet")) {
    return "tablet";
  }
  if (type.includes("browser") || type.includes("web")) {
    return "browser";
  }
  return "desktop";
}

function submitManual() {
  const address = manualAddress.value.trim();
  if (!address) {
    return;
  }
  emit("add-device", address);
  manualAddress.value = "";
  manualOpen.value = false;
}

function runPicker(action) {
  showAddMenu.value = false;
  emit(action);
}
</script>

<template>
  <section class="lan-send-tab">
    <div class="lan-send-scroll">
      <section class="lan-selection-section">
        <div class="lan-section-head">
          <h2>{{ t("lanTransferSelectionTitle") }}</h2>
          <button
            v-if="selection.length"
            class="ghost compact"
            type="button"
            @click="emit('clear-selection')"
          >
            {{ t("lanTransferSelectionClear") }}
          </button>
        </div>

        <div v-if="!selection.length" class="lan-picker-grid">
          <button
            class="lan-picker-button"
            type="button"
            :disabled="busy || !running"
            @click="emit('pick-files')"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M6 3.5h7l5 5V20.5H6zM13 3.5v5h5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" />
            </svg>
            <span>{{ t("lanTransferPickFile") }}</span>
          </button>
          <button
            class="lan-picker-button"
            type="button"
            :disabled="busy || !running"
            @click="emit('pick-folder')"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M3.5 7.5h6l2-2h9v13H3.5z" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" />
            </svg>
            <span>{{ t("lanTransferPickFolder") }}</span>
          </button>
          <button
            class="lan-picker-button"
            type="button"
            :disabled="busy || !running"
            @click="emit('pick-text')"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M4 5.5h16M4 10h16M4 14.5h10M4 19h7" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
            </svg>
            <span>{{ t("lanTransferPickText") }}</span>
          </button>
          <button
            class="lan-picker-button"
            type="button"
            :disabled="busy || !running"
            @click="emit('pick-clipboard')"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M8 6.5h8M9.5 4h5v3h-5zM6 6.5h1.5M6 11.5h12M6 16.5h12M6 21.5h8" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
            <span>{{ t("lanTransferPickClipboard") }}</span>
          </button>
        </div>

        <div v-else class="lan-selection-card">
          <div class="lan-selection-summary">
            <div>
              <strong>{{ t("lanTransferSelectionCount", { count: fileItems.length }) }}</strong>
              <small>{{ formatBytes(totalBytes) }}</small>
            </div>
            <div class="lan-selection-actions">
              <button
                class="ghost compact"
                type="button"
                @click="showAddMenu = !showAddMenu"
              >
                {{ t("addAction") }}
              </button>
              <div v-if="showAddMenu" class="lan-add-menu">
                <button type="button" @click="runPicker('pick-files')">
                  {{ t("lanTransferPickFile") }}
                </button>
                <button type="button" @click="runPicker('pick-folder')">
                  {{ t("lanTransferPickFolder") }}
                </button>
                <button type="button" @click="runPicker('pick-text')">
                  {{ t("lanTransferPickText") }}
                </button>
                <button type="button" @click="runPicker('pick-clipboard')">
                  {{ t("lanTransferPickClipboard") }}
                </button>
              </div>
            </div>
          </div>

          <div class="lan-selection-list">
            <article
              v-for="(item, index) in selection"
              :key="item.id || `${item.kind}-${index}`"
              class="lan-selection-chip"
            >
              <button
                class="lan-selection-chip-main"
                type="button"
                @click="item.kind === 'text' ? emit('edit-text', index) : null"
              >
                <strong>{{ itemLabel(item) }}</strong>
                <small>{{ itemMeta(item) }}</small>
              </button>
              <button
                class="lan-selection-remove"
                type="button"
                :title="t('removeAction')"
                :aria-label="t('removeAction')"
                @click="emit('remove-item', index)"
              >
                ×
              </button>
            </article>
          </div>

          <p v-if="textItem" class="lan-selection-hint">
            {{ t("lanTransferTextSelectionHint") }}
          </p>
        </div>
      </section>

      <section v-if="activeSend" class="lan-active-send">
        <div>
          <strong>{{ activeSend.label }}</strong>
          <small>{{ activeSend.peerAlias }} · {{ t("lanTransferStatusActive") }}</small>
          <progress :value="progressOf(activeSend)" max="100"></progress>
        </div>
        <span>{{ progressOf(activeSend) }}%</span>
        <button
          class="ghost compact"
          type="button"
          :disabled="busy"
          @click="emit('cancel-transfer', activeSend.id)"
        >
          {{ t("lanTransferCancel") }}
        </button>
      </section>

      <section class="lan-device-section">
        <div class="lan-section-head">
          <h2>{{ t("lanTransferDevices") }}</h2>
          <div class="lan-device-toolbar">
            <button
              class="toolbar-icon-button"
              :class="{ spinning: scanRunning }"
              type="button"
              :title="scanRunning ? t('lanScanCancel') : t('lanTransferRefresh')"
              :aria-label="t('lanTransferRefresh')"
              :disabled="busy || !running"
              @click="emit('scan')"
            >
              <svg viewBox="0 0 1024 1024" aria-hidden="true">
                <path
                  d="M958.681412 457.499032c-6.170072-50.632177-20.854483-99.563886-43.643361-145.434552-45.779694-92.144205-122.249797-166.333021-215.325711-208.898719-20.083724-9.18513-43.810309-0.349891-52.995439 19.734833-9.18413 20.082724-0.349891 43.810309 19.733833 52.996438 159.26323 72.834239 245.755201 249.640987 205.658732 420.410622-30.735395 130.876101-129.201624 233.321087-256.187941 270.333521l-0.262918-70.800875-196.843487 114.650172 197.690222 113.176632-0.275914-74.43274c75.398438-17.911403 144.809747-54.929834 202.084849-108.039237 65.597501-60.827991 111.122274-139.186504 131.651859-226.606186 12.170197-51.828803 15.10328-104.683286 8.715276-157.089909zM408.299406-0.001l0.271915 74.43374c-75.404436 17.911403-144.820744 54.931834-202.099843 108.046235-65.6005 60.83099-111.124274 139.191503-131.651859 226.616183-7.987504 34.034364-11.994252 68.507591-11.994252 103.010809 0 17.994377 1.090659 35.996751 3.271978 53.946142 6.152077 50.59119 20.803499 99.48891 43.545392 145.333583 45.678725 92.080225 122.012871 166.270041 214.936832 208.900718 20.071728 9.209122 43.810309 0.401874 53.018432-19.670852 9.210122-20.076726 0.400875-43.810309-19.671853-53.019432-158.963324-72.92821-245.278351-249.658982-205.24886-420.22368 30.732396-130.883099 129.201624-233.333083 256.195939-270.345517l0.259919 70.801874 196.850484-114.640174L408.299406-0.001z"
                  fill="currentColor"
                />
              </svg>
            </button>
            <button
              class="toolbar-icon-button"
              type="button"
              :title="t('lanTransferManualSend')"
              :aria-label="t('lanTransferManualSend')"
              :disabled="busy || !running"
              @click="manualOpen = true"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M12 5v14M5 12h14" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" />
              </svg>
            </button>
            <button
              class="toolbar-icon-button"
              type="button"
              :title="t('lanTransferWebShare')"
              :aria-label="t('lanTransferWebShare')"
              :disabled="busy || !running"
              @click="emit('open-link')"
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M10 13.8a4 4 0 0 0 5.7.1l2.6-2.6a4 4 0 0 0-5.7-5.7l-1.5 1.5M14 10.2a4 4 0 0 0-5.7-.1l-2.6 2.6a4 4 0 1 0 5.7 5.7l1.5-1.5" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
              </svg>
            </button>
          </div>
        </div>

        <LanSubnetPicker
          v-if="subnetMenuOpen"
          :busy="busy"
          :items="subnetItems"
          :last-subnet="lastSubnet"
          :t="t"
          @close="emit('close-subnet-menu')"
          @select="emit('select-subnet', $event)"
        />

        <p v-if="!devices.length" class="lan-device-empty">
          {{ t("lanTransferNoDevices") }}
        </p>
        <button
          v-for="device in devices"
          :key="device.fingerprint"
          class="lan-device-item"
          type="button"
          :disabled="busy || !running"
          @click="emit('send-device', device)"
        >
          <span class="lan-device-icon" :data-device="deviceTypeLabel(device)">
            <svg v-if="deviceTypeLabel(device) === 'phone'" viewBox="0 0 24 24" aria-hidden="true">
              <rect x="7" y="2.5" width="10" height="19" rx="2" fill="none" stroke="currentColor" stroke-width="1.7" />
              <path d="M10.3 18.5h3.4" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
            </svg>
            <svg v-else-if="deviceTypeLabel(device) === 'tablet'" viewBox="0 0 24 24" aria-hidden="true">
              <rect x="4.5" y="3" width="15" height="18" rx="2" fill="none" stroke="currentColor" stroke-width="1.7" />
            </svg>
            <svg v-else-if="deviceTypeLabel(device) === 'browser'" viewBox="0 0 24 24" aria-hidden="true">
              <circle cx="12" cy="12" r="8.5" fill="none" stroke="currentColor" stroke-width="1.7" />
              <path d="M3.8 9h16.4M3.8 15h16.4M12 3.5c2.2 2.3 3.3 5.1 3.3 8.5S14.2 18.2 12 20.5C9.8 18.2 8.7 15.4 8.7 12S9.8 5.8 12 3.5z" fill="none" stroke="currentColor" stroke-width="1.4" />
            </svg>
            <svg v-else viewBox="0 0 24 24" aria-hidden="true">
              <rect x="3.5" y="4.5" width="17" height="12" rx="1.8" fill="none" stroke="currentColor" stroke-width="1.7" />
              <path d="M8.5 20h7M12 16.5V20" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" />
            </svg>
          </span>
          <span class="lan-device-info">
            <strong>{{ device.alias }}</strong>
            <small>
              {{ device.deviceModel || t("lanTransferUnknownModel") }}
              · {{ device.protocol?.toUpperCase() || "HTTP" }}
              · {{ device.host }}:{{ device.port }}
            </small>
          </span>
          <span v-if="device.trusted" class="lan-device-trusted">
            {{ t("lanTransferTrusted") }}
          </span>
          <svg class="lan-device-chevron" viewBox="0 0 24 24" aria-hidden="true">
            <path d="m9 5 7 7-7 7" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" />
          </svg>
        </button>
      </section>

      <p class="lan-send-help">{{ t("lanTransferSendHelp") }}</p>
    </div>

    <div v-if="manualOpen" class="lan-manual-backdrop" @click.self="manualOpen = false">
      <form class="lan-manual-card" @submit.prevent="submitManual">
        <strong>{{ t("lanTransferManualSend") }}</strong>
        <input
          v-model="manualAddress"
          type="text"
          :placeholder="t('lanTransferIpPlaceholder')"
          autofocus
        />
        <div>
          <button class="ghost compact" type="button" @click="manualOpen = false">
            {{ t("cancelAction") }}
          </button>
          <button class="primary compact" type="submit" :disabled="!manualAddress.trim()">
            {{ t("addAction") }}
          </button>
        </div>
      </form>
    </div>
  </section>
</template>

<style scoped>
.lan-send-tab {
  position: relative;
  height: 100%;
  min-height: 0;
}

.lan-send-scroll {
  display: grid;
  align-content: start;
  gap: 16px;
  height: 100%;
  min-height: 0;
  overflow: auto;
  /* 底部留白只按内容需要给出；移动端底部导航的避让在窄屏媒体查询里单独设置。 */
  padding: 24px 26px 26px;
}

.lan-selection-section,
.lan-device-section {
  display: grid;
  gap: 10px;
}

.lan-section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.lan-section-head h2 {
  margin: 0;
  font-size: 0.94rem;
}

.lan-picker-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(88px, 1fr));
  gap: 10px;
}

.lan-picker-button {
  display: grid;
  align-content: center;
  justify-items: center;
  gap: 8px;
  min-height: 76px;
  padding: 10px;
  border: 1px solid var(--app-panel-border);
  border-radius: 12px;
  background: color-mix(in srgb, var(--app-panel-strong-bg) 72%, transparent);
  color: var(--app-text);
  font: inherit;
  font-size: 0.76rem;
  cursor: pointer;
  transition:
    border-color 150ms ease,
    background-color 150ms ease,
    transform 150ms ease;
}

.lan-picker-button:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--accent-primary) 42%, var(--app-panel-border));
  background: color-mix(in srgb, var(--accent-primary-soft) 30%, var(--app-panel-strong-bg));
  transform: translateY(-1px);
}

.lan-picker-button:disabled {
  cursor: default;
  opacity: 0.45;
}

.lan-picker-button svg {
  width: 24px;
  height: 24px;
}

.lan-selection-card {
  display: grid;
  gap: 10px;
  padding: 12px;
  border: 1px solid var(--app-panel-border);
  border-radius: 13px;
  background: color-mix(in srgb, var(--app-panel-bg) 82%, transparent);
}

.lan-selection-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.lan-selection-summary > div:first-child {
  display: grid;
  gap: 2px;
}

.lan-selection-summary strong {
  font-size: 0.82rem;
}

.lan-selection-summary small {
  color: var(--app-muted);
  font-size: 0.7rem;
}

.lan-selection-actions {
  position: relative;
}

.lan-add-menu {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
  z-index: 8;
  display: grid;
  min-width: 130px;
  padding: 5px;
  border: 1px solid var(--app-panel-border);
  border-radius: 10px;
  background: var(--app-select-menu-bg);
  box-shadow: 0 16px 38px rgba(0, 0, 0, 0.28);
}

.lan-add-menu button {
  padding: 7px 9px;
  border: 0;
  border-radius: 7px;
  background: transparent;
  color: inherit;
  font: inherit;
  font-size: 0.74rem;
  text-align: left;
  cursor: pointer;
}

.lan-add-menu button:hover {
  background: var(--accent-primary-hover-soft);
}

.lan-selection-list {
  display: flex;
  gap: 8px;
  overflow-x: auto;
  padding-bottom: 2px;
}

.lan-selection-chip {
  display: flex;
  flex: 0 0 188px;
  align-items: center;
  gap: 4px;
  padding: 8px;
  border: 1px solid var(--app-panel-border);
  border-radius: 10px;
  background: color-mix(in srgb, var(--app-panel-strong-bg) 74%, transparent);
}

.lan-selection-chip-main {
  display: grid;
  flex: 1;
  gap: 2px;
  min-width: 0;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
}

.lan-selection-chip-main strong,
.lan-selection-chip-main small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lan-selection-chip-main strong {
  font-size: 0.75rem;
}

.lan-selection-chip-main small {
  color: var(--app-muted);
  font-size: 0.66rem;
}

.lan-selection-remove {
  width: 22px;
  height: 22px;
  flex: 0 0 auto;
  padding: 0;
  border: 0;
  border-radius: 999px;
  background: transparent;
  color: var(--app-muted);
  font: inherit;
  font-size: 1rem;
  line-height: 1;
  cursor: pointer;
}

.lan-selection-remove:hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--app-text);
}

.lan-selection-hint,
.lan-send-help {
  margin: 0;
  color: var(--app-muted);
  font-size: 0.7rem;
}

.lan-active-send {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto auto;
  gap: 10px;
  align-items: center;
  padding: 10px 12px;
  border: 1px solid color-mix(in srgb, var(--accent-primary) 38%, var(--app-panel-border));
  border-radius: 12px;
  background: color-mix(in srgb, var(--accent-primary-soft) 28%, var(--app-panel-bg));
}

.lan-active-send > div {
  display: grid;
  gap: 3px;
  min-width: 0;
}

.lan-active-send strong,
.lan-active-send small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lan-active-send strong {
  font-size: 0.78rem;
}

.lan-active-send small,
.lan-active-send > span {
  color: var(--app-muted);
  font-size: 0.68rem;
}

.lan-active-send progress {
  width: 100%;
  height: 5px;
  overflow: hidden;
  border: 0;
  border-radius: 999px;
}

.lan-device-toolbar {
  display: flex;
  gap: 4px;
}

.lan-device-toolbar .toolbar-icon-button.spinning svg {
  animation: lan-send-spin 900ms linear infinite;
}

@keyframes lan-send-spin {
  to {
    transform: rotate(360deg);
  }
}

.lan-device-empty {
  margin: 0;
  padding: 24px 16px;
  border: 1px dashed var(--app-panel-border);
  border-radius: 12px;
  color: var(--app-muted);
  font-size: 0.76rem;
  text-align: center;
}

.lan-device-item {
  display: flex;
  align-items: center;
  gap: 11px;
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--app-panel-border);
  border-radius: 12px;
  background: color-mix(in srgb, var(--app-panel-strong-bg) 70%, transparent);
  color: var(--app-text);
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition:
    border-color 150ms ease,
    background-color 150ms ease;
}

.lan-device-item:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--accent-primary) 42%, var(--app-panel-border));
  background: color-mix(in srgb, var(--app-panel-strong-bg) 88%, transparent);
}

.lan-device-item:disabled {
  cursor: default;
  opacity: 0.55;
}

.lan-device-icon {
  display: grid;
  place-items: center;
  width: 42px;
  height: 42px;
  flex: 0 0 auto;
  border-radius: 12px;
  background: color-mix(in srgb, var(--accent-primary-soft) 48%, var(--app-panel-bg));
  color: var(--accent-primary);
}

.lan-device-icon svg {
  width: 25px;
  height: 25px;
}

.lan-device-info {
  display: grid;
  flex: 1;
  gap: 3px;
  min-width: 0;
}

.lan-device-info strong,
.lan-device-info small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lan-device-info strong {
  font-size: 0.84rem;
}

.lan-device-info small {
  color: var(--app-muted);
  font-size: 0.68rem;
}

.lan-device-trusted {
  flex: 0 0 auto;
  padding: 2px 6px;
  border-radius: 999px;
  background: var(--accent-primary-soft);
  color: var(--accent-primary);
  font-size: 0.64rem;
}

.lan-device-chevron {
  width: 16px;
  height: 16px;
  flex: 0 0 auto;
  color: var(--app-muted);
}

.lan-manual-backdrop {
  position: absolute;
  inset: 0;
  z-index: 20;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.32);
}

.lan-manual-card {
  display: grid;
  gap: 10px;
  width: min(90%, 330px);
  padding: 15px;
  border-radius: 13px;
  background: var(--app-select-menu-bg);
  color: var(--app-select-menu-text);
  box-shadow: 0 20px 48px rgba(0, 0, 0, 0.3);
}

.lan-manual-card input {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--app-panel-border);
  border-radius: 8px;
  background: var(--app-input-bg);
  color: var(--app-text);
  font: inherit;
  font-size: 0.8rem;
}

.lan-manual-card > div {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

@media (max-width: 799px) {
  .lan-picker-grid {
    grid-template-columns: repeat(2, minmax(90px, 1fr));
  }
}

@media (max-width: 699px) {
  .lan-send-scroll {
    padding: 18px 16px 86px;
  }
}
</style>
