<script setup>
// 局域网互传主页面：LocalSend 风格的接收/发送双 Tab Shell。
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { lanErrorCode, lanErrorText, lanWarningText } from "../utils/lanError";
import LanReceiveTab from "./lan-transfer/LanReceiveTab.vue";
import LanSendTab from "./lan-transfer/LanSendTab.vue";
import LanTransferHistoryPanel from "./lan-transfer/LanTransferHistoryPanel.vue";
import LanLinkDialog from "./lan-transfer/LanLinkDialog.vue";

const props = defineProps({
  busy: { type: Boolean, required: true },
  error: { type: String, default: "" },
  onAddDevice: { type: Function, required: true },
  onBack: { type: Function, required: true },
  onCancelTransfer: { type: Function, required: true },
  onCancelScan: { type: Function, required: true },
  onInspectSelection: { type: Function, required: true },
  onListSubnets: { type: Function, required: true },
  onOpenFile: { type: Function, required: true },
  onReadClipboard: { type: Function, required: true },
  onRefreshDevices: { type: Function, required: true },
  onRevealFile: { type: Function, required: true },
  onScanSubnets: { type: Function, required: true },
  onSendItems: { type: Function, required: true },
  onSetWebMode: { type: Function, required: true },
  onStart: { type: Function, required: true },
  onStartService: { type: Function, required: true },
  onStopService: { type: Function, required: true },
  platform: { type: String, default: "" },
  state: { type: Object, required: true },
  t: { type: Function, required: true },
});

const activeTab = ref("receive");
const selection = ref([]);
const historyOpen = ref(false);
const linkOpen = ref(false);
const textComposerOpen = ref(false);
const textDraft = ref("");
const editingTextIndex = ref(-1);
const pinPrompt = ref(null);
const pinDraft = ref("");
const localError = ref("");
const isFileDragOver = ref(false);
const subnetMenuOpen = ref(false);
const subnetItems = ref([]);
const lastSubnet = ref("");
const localIps = ref([]);
let unlistenDragDrop = null;

const devices = computed(() =>
  Array.isArray(props.state.devices) ? props.state.devices : [],
);
const transfers = computed(() =>
  Array.isArray(props.state.transfers) ? props.state.transfers : [],
);
const receivedFiles = computed(() =>
  Array.isArray(props.state.receivedFiles) ? props.state.receivedFiles : [],
);
const running = computed(() => props.state.status === "running");
const failed = computed(() => props.state.status === "error");
const scanRunning = computed(() => Boolean(props.state.scan?.running));
const webMode = computed(() => props.state.webMode || "none");
const selectionFiles = computed(() =>
  selection.value.filter((item) => item.kind === "file"),
);
const historyCount = computed(() => receivedFiles.value.length + transfers.value.length);
const pageError = computed(() => {
  if (props.state.error) {
    return lanErrorText(props.t, props.state.errorCode, props.state.error);
  }
  if (props.error) {
    return lanErrorText(props.t, lanErrorCode(props.error), props.error);
  }
  return localError.value;
});
const pageWarning = computed(() => {
  const warning = props.state.warning;
  if (!warning) {
    return "";
  }
  const code = lanErrorCode(warning);
  return code ? lanWarningText(props.t, code, warning, props.platform) : warning;
});
const statusLabel = computed(() => {
  if (failed.value) {
    return lanErrorText(props.t, props.state.errorCode, props.state.error);
  }
  return running.value
    ? props.t("lanTransferStatusRunning")
    : props.t("lanTransferStatusStopped");
});

async function run(action) {
  localError.value = "";
  try {
    return await action();
  } catch (error) {
    localError.value = error?.message || String(error);
    throw error;
  }
}

async function loadLocalIps() {
  try {
    const payload = await props.onListSubnets();
    const items = Array.isArray(payload?.subnets) ? payload.subnets : [];
    localIps.value = [
      ...new Set(
        items
          .slice()
          .sort(
            (left, right) =>
              Number(Boolean(left.virtualInterface)) -
              Number(Boolean(right.virtualInterface)),
          )
          .map((item) => item.address)
          .filter(Boolean),
      ),
    ];
  } catch (error) {
    const detail = error?.message || String(error);
    localError.value = lanErrorText(props.t, lanErrorCode(detail), detail);
  }
}

function pathKey(path) {
  const value = String(path || "");
  return props.platform === "windows" ? value.toLowerCase() : value;
}

function selectionPayload() {
  return selection.value.map((item) =>
    item.kind === "text"
      ? { kind: "text", text: item.text }
      : {
          kind: "file",
          path: item.path,
          name: item.name,
          size: item.size,
          mimeType: item.mimeType,
        },
  );
}

function mergeSelection(items) {
  if (!Array.isArray(items) || !items.length) {
    return;
  }
  const next = selection.value.slice();
  const fileKeys = new Set(
    next.filter((item) => item.kind === "file").map((item) => pathKey(item.path)),
  );

  for (const item of items) {
    if (item.kind === "text") {
      const textItem = {
        ...item,
        id: `text-${Date.now()}-${Math.random().toString(16).slice(2)}`,
      };
      const existing = next.findIndex((entry) => entry.kind === "text");
      if (existing >= 0) {
        next.splice(existing, 1, textItem);
      } else {
        next.push(textItem);
      }
      continue;
    }

    const key = pathKey(item.path);
    if (fileKeys.has(key)) {
      continue;
    }
    fileKeys.add(key);
    next.push({ ...item, id: key });
  }
  selection.value = next;
}

async function addSelectionPaths(paths) {
  const normalized = (Array.isArray(paths) ? paths : [paths]).filter(Boolean);
  if (!normalized.length) {
    return false;
  }
  try {
    const items = await props.onInspectSelection(normalized);
    mergeSelection(items);
    return true;
  } catch (error) {
    const detail = error?.message || String(error);
    localError.value = lanErrorText(props.t, lanErrorCode(detail), detail);
    return false;
  }
}

async function pickFiles() {
  const selected = await open({ multiple: true, directory: false });
  const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
  return addSelectionPaths(paths);
}

async function pickFolder() {
  const selected = await open({ multiple: false, directory: true });
  const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
  return addSelectionPaths(paths);
}

async function pickClipboard() {
  try {
    const items = await props.onReadClipboard();
    mergeSelection(items);
  } catch (error) {
    const detail = error?.message || String(error);
    localError.value = lanErrorText(props.t, lanErrorCode(detail), detail);
  }
}

function openTextComposer(index = -1) {
  editingTextIndex.value = index;
  textDraft.value = index >= 0 ? selection.value[index]?.text || "" : "";
  textComposerOpen.value = true;
}

function closeTextComposer() {
  textComposerOpen.value = false;
  textDraft.value = "";
  editingTextIndex.value = -1;
}

function submitText() {
  const text = textDraft.value.trimEnd();
  if (!text.trim()) {
    return;
  }
  const item = {
    kind: "text",
    text,
    id: `text-${Date.now()}-${Math.random().toString(16).slice(2)}`,
  };
  const next = selection.value.slice();
  if (editingTextIndex.value >= 0) {
    next.splice(editingTextIndex.value, 1, item);
  } else {
    const existing = next.findIndex((entry) => entry.kind === "text");
    if (existing >= 0) {
      next.splice(existing, 1, item);
    } else {
      next.push(item);
    }
  }
  selection.value = next;
  closeTextComposer();
}

function removeSelection(index) {
  selection.value = selection.value.filter((_, itemIndex) => itemIndex !== index);
}

function clearSelection() {
  selection.value = [];
}

async function ensureSelection() {
  if (selection.value.length) {
    return true;
  }
  return pickFiles();
}

async function sendToDevice(device) {
  if (!running.value || props.busy) {
    return;
  }
  const ready = await ensureSelection();
  if (!ready) {
    return;
  }
  await attemptSend(device, selectionPayload());
}

async function attemptSend(device, items, pin = "") {
  localError.value = "";
  try {
    await props.onSendItems(device.fingerprint, items, pin || null);
    return true;
  } catch (error) {
    const detail = error?.message || String(error);
    if (lanErrorCode(detail) === "pin_required") {
      pinPrompt.value = {
        device,
        items,
        invalid: Boolean(pin),
      };
      pinDraft.value = "";
      return false;
    }
    localError.value = lanErrorText(props.t, lanErrorCode(detail), detail);
    return false;
  }
}

async function submitPin() {
  const prompt = pinPrompt.value;
  const pin = pinDraft.value.trim();
  if (!prompt || !pin) {
    return;
  }
  const sent = await attemptSend(prompt.device, prompt.items, pin);
  if (sent) {
    closePinPrompt();
  }
}

function closePinPrompt() {
  pinPrompt.value = null;
  pinDraft.value = "";
}

async function addManualDevice(address) {
  try {
    const next = await props.onAddDevice(address);
    const device = (next?.devices || []).find(
      (entry) => entry.host === address || entry.alias === address,
    );
    if (device) {
      await sendToDevice(device);
    }
  } catch (error) {
    const detail = error?.message || String(error);
    localError.value = lanErrorText(props.t, lanErrorCode(detail), detail);
  }
}

async function refreshDevices() {
  if (scanRunning.value) {
    await run(props.onCancelScan);
    return;
  }
  if (subnetMenuOpen.value) {
    subnetMenuOpen.value = false;
    return;
  }
  await run(async () => {
    const payload = await props.onListSubnets();
    const items = Array.isArray(payload?.subnets) ? payload.subnets : [];
    subnetItems.value = items;
    lastSubnet.value = payload?.lastSubnet || "";
    const physicalCount = items.filter((item) => !item.virtualInterface).length;
    const countable = physicalCount || items.length;
    if (countable <= 1) {
      subnetMenuOpen.value = false;
      await props.onRefreshDevices();
      return;
    }
    subnetMenuOpen.value = true;
  });
}

async function selectSubnet(cidr) {
  subnetMenuOpen.value = false;
  await run(() => props.onScanSubnets([cidr]));
}

async function openLink(mode = "share") {
  localError.value = "";
  try {
    let paths = selectionFiles.value.map((item) => item.path);
    if (mode === "share" && !paths.length) {
      const picked = await pickFiles();
      if (!picked) {
        return;
      }
      paths = selectionFiles.value.map((item) => item.path);
    }
    await props.onSetWebMode(mode, mode === "share" ? paths : []);
    linkOpen.value = true;
  } catch (error) {
    const detail = error?.message || String(error);
    localError.value = lanErrorText(props.t, lanErrorCode(detail), detail);
  }
}

async function closeLink() {
  linkOpen.value = false;
  try {
    await props.onSetWebMode("none", []);
  } catch (error) {
    const detail = error?.message || String(error);
    localError.value = lanErrorText(props.t, lanErrorCode(detail), detail);
  }
}

async function handleDrop(event) {
  const { payload } = event;
  if (payload.type === "leave") {
    isFileDragOver.value = false;
    return;
  }
  if (payload.type === "over") {
    isFileDragOver.value = true;
    return;
  }
  isFileDragOver.value = false;
  if (payload.type !== "drop" || !payload.paths?.length) {
    return;
  }
  const added = await addSelectionPaths(payload.paths);
  if (added) {
    activeTab.value = "send";
  }
}

async function cancelTransfer(transferId) {
  await run(() => props.onCancelTransfer(transferId));
}

async function openReceivedFile(id) {
  await run(() => props.onOpenFile(id));
}

async function revealReceivedFile(id) {
  await run(() => props.onRevealFile(id));
}

watch(
  () => props.state.webMode,
  (mode) => {
    if (mode && mode !== "none") {
      linkOpen.value = true;
    } else {
      linkOpen.value = false;
    }
  },
);

watch(running, (value) => {
  if (value) {
    void loadLocalIps();
  }
});

onMounted(async () => {
  unlistenDragDrop = await getCurrentWindow().onDragDropEvent((event) => {
    void handleDrop(event);
  });
  await props.onStart();
  await loadLocalIps();
});

onUnmounted(() => {
  unlistenDragDrop?.();
  if (webMode.value !== "none") {
    props.onSetWebMode("none", []).catch(() => {});
  }
});
</script>

<template>
  <section class="lan-transfer-page" :data-active-tab="activeTab">
    <header class="lan-transfer-topbar">
      <button
        class="toolbar-icon-button"
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
      <div class="lan-transfer-title">
        <strong>{{ t("lanTransferTitle") }}</strong>
        <span class="lan-transfer-status-pill">
          <i :class="{ connected: running, disconnected: !running }"></i>
          {{ statusLabel }}
        </span>
      </div>
      <button
        class="toolbar-icon-button lan-transfer-service-button"
        :class="{ running }"
        type="button"
        :disabled="busy"
        :title="running ? t('lanTransferStopService') : t('lanTransferStartService')"
        :aria-label="running ? t('lanTransferStopService') : t('lanTransferStartService')"
        @click="running ? run(onStopService) : run(onStartService)"
      >
        <svg viewBox="0 0 1024 1024" aria-hidden="true">
          <path
            d="M652 125.54l0 71.8c140 63.74 211.28 193.28 211.28 342.82 0 212.64-171.62 384.98-384.24 384.98-212.6 0-380.56-172.34-380.56-384.98 0-149.22 93.52-278.56 213.52-342.42l0-71.84c-180 68.46-278.14 228.16-278.14 414.24 0 248.52 199.42 449.94 447.92 449.94 248.48 0 447.5-201.42 447.5-449.94 0-186.38-97.28-346.32-277.28-414.6zM512 412c0 22.08-17.92 40-40 40l0 0c-22.08 0-40-17.92-40-40l0-340c0-22.08 17.92-40 40-40l0 0c22.08 0 40 17.92 40 40l0 340z"
            fill="currentColor"
          />
        </svg>
      </button>
    </header>

    <div class="lan-transfer-shell">
      <nav class="lan-transfer-rail" :aria-label="t('lanTransferTitle')">
        <div class="lan-transfer-brand">
          <img :src="'/app-icon-512.png'" alt="" />
          <strong>{{ t("lanTransferTitle") }}</strong>
        </div>
        <button
          type="button"
          :class="{ active: activeTab === 'receive' }"
          @click="activeTab = 'receive'"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="M4 9.5a12.5 12.5 0 0 1 16 0M7 13a8 8 0 0 1 10 0M10 16.5a3.5 3.5 0 0 1 4 0M12 20h.01" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" />
          </svg>
          <span>{{ t("lanTransferReceive") }}</span>
        </button>
        <button
          type="button"
          :class="{ active: activeTab === 'send' }"
          @click="activeTab = 'send'"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="m3.5 5 17 7-17 7 3.3-7zM6.8 12h6.4" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" stroke-linecap="round" />
          </svg>
          <span>{{ t("lanTransferSend") }}</span>
        </button>
      </nav>

      <main class="lan-transfer-content">
        <LanReceiveTab
          v-if="activeTab === 'receive'"
          :busy="busy"
          :history-count="historyCount"
          :local-ips="localIps"
          :running="running"
          :state="state"
          :t="t"
          @open-history="historyOpen = true"
          @open-link="openLink('receive')"
          @start-service="run(onStartService)"
        />
        <LanSendTab
          v-else
          :busy="busy"
          :devices="devices"
          :last-subnet="lastSubnet"
          :running="running"
          :scan-running="scanRunning"
          :selection="selection"
          :subnet-items="subnetItems"
          :subnet-menu-open="subnetMenuOpen"
          :t="t"
          :transfers="transfers"
          @add-device="addManualDevice"
          @cancel-transfer="cancelTransfer"
          @clear-selection="clearSelection"
          @close-subnet-menu="subnetMenuOpen = false"
          @edit-text="openTextComposer"
          @open-link="openLink('share')"
          @pick-clipboard="pickClipboard"
          @pick-files="pickFiles"
          @pick-folder="pickFolder"
          @pick-text="openTextComposer(-1)"
          @remove-item="removeSelection"
          @scan="refreshDevices"
          @select-subnet="selectSubnet"
          @send-device="sendToDevice"
        />
      </main>

      <LanTransferHistoryPanel
        v-if="historyOpen"
        :busy="busy"
        :received-files="receivedFiles"
        :t="t"
        :transfers="transfers"
        :on-cancel-transfer="cancelTransfer"
        :on-open-file="openReceivedFile"
        :on-reveal-file="revealReceivedFile"
        @close="historyOpen = false"
      />
    </div>

    <p v-if="pageWarning" class="lan-transfer-message warning">{{ pageWarning }}</p>
    <p v-if="pageError" class="lan-transfer-message error">{{ pageError }}</p>

    <div v-if="isFileDragOver" class="lan-transfer-drop-overlay">
      <strong>{{ t("lanTransferDropFiles") }}</strong>
    </div>

    <div v-if="textComposerOpen" class="lan-transfer-text-modal" @click.self="closeTextComposer">
      <div class="lan-transfer-text-card">
        <strong>{{ t("lanTransferPickText") }}</strong>
        <textarea
          v-model="textDraft"
          rows="5"
          :placeholder="t('lanTransferSendTextPlaceholder')"
        ></textarea>
        <div class="lan-transfer-text-actions">
          <button class="ghost compact" type="button" @click="closeTextComposer">
            {{ t("cancelAction") }}
          </button>
          <button class="primary compact" type="button" :disabled="!textDraft.trim()" @click="submitText">
            {{ t("confirmAction") }}
          </button>
        </div>
      </div>
    </div>

    <div v-if="pinPrompt" class="lan-transfer-text-modal" @click.self="closePinPrompt">
      <div class="lan-transfer-text-card">
        <strong>{{ t("lanPinTitle") }}</strong>
        <small>{{ t("lanPinHint") }}</small>
        <input
          v-model="pinDraft"
          type="password"
          inputmode="numeric"
          maxlength="6"
          :placeholder="t('lanPinPlaceholder')"
          @keydown.enter.prevent="submitPin"
        />
        <small v-if="pinPrompt.invalid" class="lan-transfer-pin-error">
          {{ t("lanPinInvalid") }}
        </small>
        <div class="lan-transfer-text-actions">
          <button class="ghost compact" type="button" @click="closePinPrompt">
            {{ t("cancelAction") }}
          </button>
          <button
            class="primary compact"
            type="button"
            :disabled="busy || !pinDraft.trim()"
            @click="submitPin"
          >
            {{ t("lanTransferSend") }}
          </button>
        </div>
      </div>
    </div>

    <LanLinkDialog
      v-if="linkOpen"
      :busy="busy"
      :mode="webMode === 'share' ? 'share' : 'receive'"
      :state="state"
      :t="t"
      @close="closeLink"
    />
  </section>
</template>

<style scoped>
.lan-transfer-page {
  position: relative;
  display: grid;
  grid-template-rows: auto minmax(0, 1fr) auto auto;
  height: 100%;
  min-height: 0;
  color: var(--app-text);
}

.lan-transfer-topbar {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px 8px;
}

.lan-transfer-title {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.lan-transfer-title strong {
  font-size: 0.94rem;
}

.lan-transfer-status-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 3px 8px;
  border: 1px solid var(--app-panel-border);
  border-radius: 999px;
  color: var(--app-muted);
  font-size: 0.68rem;
}

.lan-transfer-status-pill i {
  width: 7px;
  height: 7px;
  border-radius: 999px;
}

.lan-transfer-status-pill i.connected {
  background: #44d17f;
  box-shadow: 0 0 8px rgba(68, 209, 127, 0.7);
}

.lan-transfer-status-pill i.disconnected {
  background: #f55656;
}

.lan-transfer-service-button,
.lan-transfer-service-button:hover {
  background: transparent;
  box-shadow: none;
  transform: none;
}

.lan-transfer-service-button.running {
  color: var(--accent-primary);
}

.lan-transfer-shell {
  position: relative;
  display: flex;
  min-height: 0;
  border-top: 1px solid var(--app-panel-border);
}

.lan-transfer-rail {
  display: flex;
  width: 172px;
  flex: 0 0 auto;
  flex-direction: column;
  gap: 8px;
  padding: 16px 12px;
  border-right: 1px solid var(--app-panel-border);
  background: color-mix(in srgb, var(--app-panel-bg) 72%, transparent);
}

.lan-transfer-brand {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 14px;
  padding: 0 8px;
}

.lan-transfer-brand img {
  width: 28px;
  height: 28px;
  object-fit: contain;
}

.lan-transfer-brand strong {
  font-size: 1rem;
}

.lan-transfer-rail > button {
  display: flex;
  align-items: center;
  gap: 12px;
  width: 100%;
  padding: 10px 12px;
  border: 0;
  border-radius: 11px;
  background: transparent;
  color: var(--app-muted);
  font: inherit;
  font-size: 0.82rem;
  text-align: left;
  cursor: pointer;
  transition:
    background-color 150ms ease,
    color 150ms ease;
}

.lan-transfer-rail > button:hover {
  color: var(--app-text);
}

.lan-transfer-rail > button.active {
  background: var(--accent-primary-soft);
  color: var(--accent-primary);
  font-weight: 650;
}

.lan-transfer-rail svg {
  width: 21px;
  height: 21px;
  flex: 0 0 auto;
}

.lan-transfer-content {
  min-width: 0;
  min-height: 0;
  flex: 1;
  overflow: hidden;
}

.lan-transfer-message {
  margin: 0 14px 8px;
  font-size: 0.74rem;
}

.lan-transfer-message.warning {
  color: #d7a441;
}

.lan-transfer-message.error,
.lan-transfer-pin-error {
  color: #f06d6d;
}

.lan-transfer-pin-error {
  font-size: 0.72rem;
}

@media (max-width: 799px) {
  .lan-transfer-rail {
    width: 72px;
    align-items: center;
    padding: 14px 8px;
  }

  .lan-transfer-brand {
    justify-content: center;
    padding: 0;
  }

  .lan-transfer-brand strong,
  .lan-transfer-rail span {
    display: none;
  }

  .lan-transfer-rail > button {
    justify-content: center;
    padding: 11px;
  }
}

@media (max-width: 699px) {
  .lan-transfer-shell {
    display: grid;
    grid-template-rows: minmax(0, 1fr) auto;
  }

  .lan-transfer-rail {
    grid-row: 2;
    flex-direction: row;
    width: 100%;
    height: 56px;
    justify-content: center;
    gap: 10px;
    padding: 5px 10px;
    border-top: 1px solid var(--app-panel-border);
    border-right: 0;
    background: var(--app-select-menu-bg);
  }

  .lan-transfer-brand {
    display: none;
  }

  .lan-transfer-rail > button {
    width: auto;
    min-width: 110px;
    justify-content: center;
    padding: 9px 14px;
  }

  .lan-transfer-rail span {
    display: inline;
  }

  .lan-transfer-content {
    grid-row: 1;
  }
}
</style>
