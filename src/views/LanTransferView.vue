<script setup>
// 局域网互传主页：左侧设备会话列表 + 右侧会话视图，收发以聊天消息的形式呈现。
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, onMounted, onUnmounted, reactive, ref, watch } from "vue";
import { lanErrorCode, lanErrorText, lanWarningText } from "../utils/lanError";
import LanSubnetPicker from "../components/LanSubnetPicker.vue";
import LanComposer from "./lan-transfer/LanComposer.vue";
import LanConversationList from "./lan-transfer/LanConversationList.vue";
import LanConversationPanel from "./lan-transfer/LanConversationPanel.vue";
import LanIdlePanel from "./lan-transfer/LanIdlePanel.vue";
import LanLinkDialog from "./lan-transfer/LanLinkDialog.vue";

const props = defineProps({
  busy: { type: Boolean, required: true },
  error: { type: String, default: "" },
  locale: { type: String, default: "zh-CN" },
  onAddDevice: { type: Function, required: true },
  onBack: { type: Function, required: true },
  onCancelTransfer: { type: Function, required: true },
  onCancelScan: { type: Function, required: true },
  onInspectSelection: { type: Function, required: true },
  onListSubnets: { type: Function, required: true },
  onOpenFile: { type: Function, required: true },
  onOpenSettings: { type: Function, required: true },
  onReadClipboard: { type: Function, required: true },
  onReadTransferPreview: { type: Function, required: true },
  onRefreshDevices: { type: Function, required: true },
  onResendTransfer: { type: Function, required: true },
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

const activeFingerprint = ref("");
const manualOpen = ref(false);
const manualAddress = ref("");
const linkOpen = ref(false);
const pinPrompt = ref(null);
const pinDraft = ref("");
const localError = ref("");
const isFileDragOver = ref(false);
const dropTargetFingerprint = ref("");
const subnetMenuOpen = ref(false);
const subnetItems = ref([]);
const lastSubnet = ref("");
const localIps = ref([]);
// 每个对端一份未发送草稿：切换会话时不丢失已输入的内容与附件。
const drafts = reactive({});
// 图片预览按传输 ID 缓存（响应式，取回后气泡自动渲染），避免重复向后台取缩略图。
const previewCache = reactive(new Map());
let unlistenDragDrop = null;
let autoSelected = false;

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
// 刷新按钮的最短旋转时长：网络很快时后端扫描瞬间结束，
// 没有这段兜底就几乎看不到“正在刷新”的反馈。
const REFRESH_SPIN_MIN_MS = 900;
const refreshSpinning = ref(false);
let refreshSpinTimer = null;
const spinning = computed(() => scanRunning.value || refreshSpinning.value);
const webMode = computed(() => props.state.webMode || "none");
// 会话列表数据源：已知对端（含离线），在线设备置顶并按最近联系排序。
const peers = computed(() => {
  const source =
    Array.isArray(props.state.peers) && props.state.peers.length
      ? props.state.peers
      : devices.value;
  return source
    .slice()
    // 只按在线状态做一次稳定分区：组内保持后端给的顺序。
    // 不能再按 lastSeenMs 排序——刷新/扫描期间每确认到一台设备都会更新它的
    // lastSeenMs，按这个时间排会让列表里的设备跟着来回跳动。
    .sort((left, right) => Number(Boolean(right.online)) - Number(Boolean(left.online)));
});
const activePeer = computed(
  () =>
    peers.value.find((peer) => peer.fingerprint === activeFingerprint.value) ||
    null,
);
// 最近一次选中的设备快照：设备列表瞬时为空（服务重建、状态回退）时，
// 会话视图仍保持挂载，避免输入区被销毁重建而丢失焦点。
const lastActivePeer = ref(null);
const conversationPeer = computed(() => {
  if (!activeFingerprint.value) {
    return null;
  }
  return activePeer.value || lastActivePeer.value;
});
const activeDraft = computed(
  () => drafts[activeFingerprint.value] || { items: [], text: "" },
);

function transfersOf(fingerprint) {
  return transfers.value.filter((item) => item.peerFingerprint === fingerprint);
}

const conversations = computed(() =>
  peers.value.map((peer) => {
    const list = transfersOf(peer.fingerprint);
    return {
      activeCount: list.filter((item) => item.status === "active").length,
      peer,
    };
  }),
);

// 会话消息流：传输记录按时间正序排列，接收到的文件挂到对应传输气泡上。
const messages = computed(() => {
  const fingerprint = activeFingerprint.value;
  if (!fingerprint) {
    return [];
  }
  const receivedByTransfer = new Map(
    receivedFiles.value
      .filter((file) => file.fromFingerprint === fingerprint)
      .map((file) => [file.transferId, file]),
  );
  return transfersOf(fingerprint)
    .map((transfer) => ({
      createdAtMs: transfer.createdAtMs,
      direction: transfer.direction,
      doneBytes: transfer.doneBytes,
      error: transfer.error || "",
      files: Array.isArray(transfer.files) ? transfer.files : [],
      id: transfer.id,
      kind: transfer.kind,
      hasPreview: Boolean(transfer.hasPreview),
      preview: previewCache.get(transfer.id) || "",
      receivedFile: receivedByTransfer.get(transfer.id) || null,
      resendable: Boolean(transfer.resendable),
      status: transfer.status,
      text: transfer.text || "",
      totalBytes: transfer.totalBytes,
    }))
    .sort(
      (left, right) => Number(left.createdAtMs || 0) - Number(right.createdAtMs || 0),
    );
});

const pageError = computed(() => {
  // 服务启动失败时顶部状态胶囊已经给出原因，这里只显示本页动作自身的错误。
  if (failed.value) {
    return "";
  }
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
const dropHint = computed(() => {
  const peer = peers.value.find(
    (item) => item.fingerprint === dropTargetFingerprint.value,
  );
  return peer
    ? props.t("lanDropToPeer", { name: peer.alias })
    : props.t("lanDropSelectPeer");
});

function reportError(detail) {
  const text = detail?.message || String(detail || "");
  localError.value = lanErrorText(props.t, lanErrorCode(text), text);
}

// 服务启停等动作：失败只提示，不把异常抛给事件处理器。
async function runAction(action) {
  try {
    await action();
  } catch (error) {
    reportError(error);
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
    reportError(error);
  }
}

function pathKey(path) {
  const value = String(path || "");
  return props.platform === "windows" ? value.toLowerCase() : value;
}

function ensureDraft(fingerprint) {
  if (!fingerprint) {
    return { items: [], text: "" };
  }
  if (!drafts[fingerprint]) {
    drafts[fingerprint] = { items: [], text: "" };
  }
  return drafts[fingerprint];
}

function clearDraft(fingerprint) {
  const draft = ensureDraft(fingerprint);
  draft.items = [];
  draft.text = "";
}

function appendDraftText(draft, text) {
  const value = String(text || "");
  if (!value.trim()) {
    return;
  }
  draft.text = draft.text.trim() ? `${draft.text.trimEnd()}\n${value}` : value;
}

function mergeDraftItems(fingerprint, items) {
  if (!Array.isArray(items) || !items.length) {
    return false;
  }
  const draft = ensureDraft(fingerprint);
  const keys = new Set(draft.items.map((item) => pathKey(item.path)));
  for (const item of items) {
    if (item.kind === "text") {
      appendDraftText(draft, item.text);
      continue;
    }
    const key = pathKey(item.path);
    if (keys.has(key)) {
      continue;
    }
    keys.add(key);
    draft.items.push({ ...item, id: key });
  }
  return true;
}

async function pickClipboardIntoDraft(fingerprint) {
  try {
    const items = await props.onReadClipboard();
    mergeDraftItems(fingerprint, items);
  } catch (error) {
    reportError(error);
  }
}

function removeDraftItem(fingerprint, index) {
  const draft = ensureDraft(fingerprint);
  draft.items = draft.items.filter((_, itemIndex) => itemIndex !== index);
}

function updateDraftText(fingerprint, text) {
  ensureDraft(fingerprint).text = text;
}

// 发送投递：统一处理 PIN 交互与离线对端的自动重连重试。
async function deliver(peer, call, onSent, pin = "", retried = false) {
  localError.value = "";
  try {
    await call(pin || null);
    onSent?.();
    return "sent";
  } catch (error) {
    const detail = error?.message || String(error);
    const code = lanErrorCode(detail);
    if (code === "pin_required") {
      pinPrompt.value = { call, invalid: Boolean(pin), onSent, peer };
      pinDraft.value = "";
      return "pin";
    }
    if (code === "lan_transfer_device_missing" && !retried) {
      const reconnected = await reconnect(peer);
      if (reconnected) {
        return deliver(peer, call, onSent, pin, true);
      }
      localError.value = props.t("lanPeerOfflineRetry");
      return "failed";
    }
    reportError(detail);
    return "failed";
  }
}

// 离线对端：用最后一次已知地址重新发现设备，成功后才重试发送。
async function reconnect(peer) {
  if (!peer?.host) {
    return false;
  }
  try {
    await props.onAddDevice(peer.host, peer.port);
  } catch {
    return false;
  }
  return devices.value.some((device) => device.fingerprint === peer.fingerprint);
}

// 把当前草稿（输入框文字 + 待发送附件）组装成一次发送载荷。
function draftItems(peer) {
  const draft = ensureDraft(peer.fingerprint);
  const payload = [];
  const text = draft.text.trimEnd();
  if (text.trim()) {
    payload.push({ kind: "text", text });
  }
  for (const item of draft.items) {
    payload.push({
      kind: "file",
      mimeType: item.mimeType,
      name: item.name,
      path: item.path,
      size: item.size,
    });
  }
  return payload;
}

// 把新读到的选择项并入载荷：文件按路径去重，文本合并成同一条消息。
function mergeIntoItems(payload, picked) {
  const merged = payload.slice();
  const texts = [];
  for (const item of picked) {
    if (item.kind === "text") {
      texts.push(item.text);
      continue;
    }
    if (merged.some((entry) => entry.kind === "file" && entry.path === item.path)) {
      continue;
    }
    merged.push({
      kind: "file",
      mimeType: item.mimeType,
      name: item.name,
      path: item.path,
      size: item.size,
    });
  }
  if (texts.length) {
    const text = texts.join("\n");
    const index = merged.findIndex((entry) => entry.kind === "text");
    if (index >= 0) {
      merged[index] = { kind: "text", text: `${merged[index].text}\n${text}` };
    } else {
      merged.push({ kind: "text", text });
    }
  }
  return merged;
}

async function sendItems(peer, items) {
  await deliver(
    peer,
    (pin) => props.onSendItems(peer.fingerprint, items, pin),
    () => clearDraft(peer.fingerprint),
  );
}

async function submitDraft(peer) {
  if (!peer || props.busy || !running.value) {
    return;
  }
  const payload = draftItems(peer);
  if (!payload.length) {
    return;
  }
  await sendItems(peer, payload);
}

// 送入一批本地路径：与输入框里已有的文字合并成同一次传输并立即发送。
// 附件菜单选完文件、拖拽松开鼠标都走这条路径，不再要求用户再点一次发送。
async function sendPaths(peer, paths) {
  if (!peer || props.busy || !running.value) {
    return false;
  }
  const normalized = (Array.isArray(paths) ? paths : [paths]).filter(Boolean);
  if (!normalized.length) {
    return false;
  }
  let picked;
  try {
    picked = await props.onInspectSelection(normalized);
  } catch (error) {
    reportError(error);
    return false;
  }
  if (!Array.isArray(picked) || !picked.length) {
    return false;
  }
  const payload = mergeIntoItems(draftItems(peer), picked);
  if (!payload.length) {
    return false;
  }
  await sendItems(peer, payload);
  return true;
}

// 附件菜单里的「文件 / 文件夹」：选完即发送。
async function sendPickedFiles(peer, directory) {
  if (!peer || props.busy || !running.value) {
    return;
  }
  const selected = await open({ multiple: !directory, directory });
  const paths = (Array.isArray(selected) ? selected : selected ? [selected] : []).filter(
    Boolean,
  );
  await sendPaths(peer, paths);
}

async function resendMessage(transferId) {
  const transfer = transfers.value.find((item) => item.id === transferId);
  if (!transfer) {
    return;
  }
  const peer = peers.value.find(
    (item) => item.fingerprint === transfer.peerFingerprint,
  );
  if (!peer) {
    localError.value = props.t("lanTransferNoDevices");
    return;
  }
  await deliver(peer, (pin) => props.onResendTransfer(transferId, pin), null);
}

// 图片气泡按需取回缩略预览，取到后缓存；没有预览时返回空串。
async function loadPreview(transferId) {
  if (previewCache.has(transferId)) {
    return previewCache.get(transferId);
  }
  const preview = (await props.onReadTransferPreview(transferId)) || "";
  previewCache.set(transferId, preview);
  return preview;
}

async function submitPin() {
  const prompt = pinPrompt.value;
  const pin = pinDraft.value.trim();
  if (!prompt || !pin) {
    return;
  }
  const outcome = await deliver(
    prompt.peer,
    prompt.call,
    prompt.onSent,
    pin,
    true,
  );
  if (outcome === "sent") {
    closePinPrompt();
  }
}

function closePinPrompt() {
  pinPrompt.value = null;
  pinDraft.value = "";
}

async function cancelTransfer(transferId) {
  try {
    await props.onCancelTransfer(transferId);
  } catch (error) {
    reportError(error);
  }
}

async function openReceivedFile(id) {
  try {
    await props.onOpenFile(id);
  } catch (error) {
    reportError(error);
  }
}

async function revealReceivedFile(id) {
  try {
    await props.onRevealFile(id);
  } catch (error) {
    reportError(error);
  }
}

async function submitManual() {
  const address = manualAddress.value.trim();
  if (!address) {
    return;
  }
  try {
    const next = await props.onAddDevice(address);
    const device = (next?.devices || []).find(
      (entry) => entry.host === address || entry.alias === address,
    );
    if (device) {
      activeFingerprint.value = device.fingerprint;
    }
    manualAddress.value = "";
    manualOpen.value = false;
  } catch (error) {
    reportError(error);
  }
}

async function refreshDevices() {
  if (scanRunning.value) {
    try {
      await props.onCancelScan();
    } catch (error) {
      reportError(error);
    }
    return;
  }
  if (subnetMenuOpen.value) {
    subnetMenuOpen.value = false;
    return;
  }
  try {
    const payload = await props.onListSubnets();
    const items = Array.isArray(payload?.subnets) ? payload.subnets : [];
    subnetItems.value = items;
    lastSubnet.value = payload?.lastSubnet || "";
    const physicalCount = items.filter((item) => !item.virtualInterface).length;
    const countable = physicalCount || items.length;
    if (countable <= 1) {
      subnetMenuOpen.value = false;
      startRefreshSpin();
      await props.onRefreshDevices();
      return;
    }
    subnetMenuOpen.value = true;
  } catch (error) {
    reportError(error);
  }
}

// 真正发起刷新/扫描时让按钮至少转够一圈。
function startRefreshSpin() {
  refreshSpinning.value = true;
  clearTimeout(refreshSpinTimer);
  refreshSpinTimer = setTimeout(() => {
    refreshSpinning.value = false;
  }, REFRESH_SPIN_MIN_MS);
}

async function selectSubnet(cidr) {
  subnetMenuOpen.value = false;
  startRefreshSpin();
  try {
    await props.onScanSubnets([cidr]);
  } catch (error) {
    reportError(error);
  }
}

// 通过链接接收：手机扫码后可以把内容传回本机。
async function openReceiveLink() {
  localError.value = "";
  try {
    await props.onSetWebMode("receive", []);
    linkOpen.value = true;
  } catch (error) {
    reportError(error);
  }
}

async function closeLink() {
  linkOpen.value = false;
  try {
    await props.onSetWebMode("none", []);
  } catch (error) {
    reportError(error);
  }
}

function selectPeer(fingerprint) {
  activeFingerprint.value = fingerprint;
  ensureDraft(fingerprint);
}

function leaveConversation() {
  activeFingerprint.value = "";
}

// 拖拽落点判定：用物理坐标换算成 CSS 坐标后命中的会话项决定收件设备。
function peerAtPoint(position) {
  if (!position) {
    return "";
  }
  const scale = window.devicePixelRatio || 1;
  const element = document.elementFromPoint(
    position.x / scale,
    position.y / scale,
  );
  const holder = element?.closest?.("[data-peer-fingerprint]");
  return holder?.getAttribute("data-peer-fingerprint") || "";
}

function draggedPaths(payload) {
  return Array.isArray(payload?.paths) ? payload.paths.filter(Boolean) : [];
}

async function handleDrop(event) {
  const { payload } = event;
  if (payload.type === "leave") {
    isFileDragOver.value = false;
    dropTargetFingerprint.value = "";
    return;
  }
  // over 事件不带路径，不能据此判断是否携带文件，只更新当前悬停的会话。
  if (payload.type === "enter") {
    const hasPaths = draggedPaths(payload).length > 0;
    isFileDragOver.value = hasPaths;
    dropTargetFingerprint.value = hasPaths ? peerAtPoint(payload.position) : "";
    return;
  }
  if (payload.type === "over") {
    if (isFileDragOver.value) {
      dropTargetFingerprint.value = peerAtPoint(payload.position);
    }
    return;
  }
  isFileDragOver.value = false;
  dropTargetFingerprint.value = "";
  if (payload.type !== "drop") {
    return;
  }
  const paths = draggedPaths(payload);
  if (!paths.length) {
    return;
  }
  // 拖到某个会话项就发给该设备，否则落到当前打开的会话上。
  const fingerprint =
    peerAtPoint(payload.position) || activeFingerprint.value || "";
  if (!peers.value.some((peer) => peer.fingerprint === fingerprint)) {
    return;
  }
  // 拖到某个会话就发给该设备：松开鼠标立即发送，不再先进草稿等一次点击。
  const peer = peers.value.find((item) => item.fingerprint === fingerprint);
  selectPeer(fingerprint);
  await sendPaths(peer, paths);
}

watch(
  () => props.state.webMode,
  (mode) => {
    linkOpen.value = Boolean(mode && mode !== "none");
  },
);

watch(running, (value) => {
  if (value) {
    void loadLocalIps();
  }
});

watch(activePeer, (peer) => {
  if (peer) {
    lastActivePeer.value = peer;
  }
});

// 首次拿到会话列表时自动打开最近一个会话，省掉一次点击。
watch(conversations, (value) => {
  if (autoSelected || activeFingerprint.value || !value.length) {
    return;
  }
  autoSelected = true;
  selectPeer(value[0].peer.fingerprint);
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
  clearTimeout(refreshSpinTimer);
  if (webMode.value !== "none") {
    props.onSetWebMode("none", []).catch(() => {});
  }
});
</script>

<template>
  <section class="lan-transfer-page" :class="{ 'has-peer': Boolean(conversationPeer) }">
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
        <img class="lan-transfer-title-icon" src="/localsend.png" alt="" />
        <strong>{{ t("lanTransferTitle") }}</strong>
        <span class="lan-transfer-status-pill">
          <i :class="{ connected: running, disconnected: !running }"></i>
          {{ statusLabel }}
        </span>
      </div>
      <button
        class="toolbar-icon-button lan-transfer-refresh-button"
        :class="{ spinning }"
        type="button"
        :title="scanRunning ? t('lanScanCancel') : t('lanTransferRefresh')"
        :aria-label="scanRunning ? t('lanScanCancel') : t('lanTransferRefresh')"
        :disabled="!running"
        @click="refreshDevices"
      >
        <svg viewBox="0 0 1024 1024" aria-hidden="true">
          <path
            d="M958.681412 457.499032c-6.170072-50.632177-20.854483-99.563886-43.643361-145.434552-45.779694-92.144205-122.249797-166.333021-215.325711-208.898719-20.083724-9.18513-43.810309-0.349891-52.995439 19.734833-9.18413 20.082724-0.349891 43.810309 19.733833 52.996438 159.26323 72.834239 245.755201 249.640987 205.658732 420.410622-30.735395 130.876101-129.201624 233.321087-256.187941 270.333521l-0.262918-70.800875-196.843487 114.650172 197.690222 113.176632-0.275914-74.43274c75.398438-17.911403 144.809747-54.929834 202.084849-108.039237 65.597501-60.827991 111.122274-139.186504 131.651859-226.606186 12.170197-51.828803 15.10328-104.683286 8.715276-157.089909zM408.299406-0.001l0.271915 74.43374c-75.404436 17.911403-144.820744 54.931834-202.099843 108.046235-65.6005 60.83099-111.124274 139.191503-131.651859 226.616183-7.987504 34.034364-11.994252 68.507591-11.994252 103.010809 0 17.994377 1.090659 35.996751 3.271978 53.946142 6.152077 50.59119 20.803499 99.48891 43.545392 145.333583 45.678725 92.080225 122.012871 166.270041 214.936832 208.900718 20.071728 9.209122 43.810309 0.401874 53.018432-19.670852 9.210122-20.076726 0.400875-43.810309-19.671853-53.019432-158.963324-72.92821-245.278351-249.658982-205.24886-420.22368 30.732396-130.883099 129.201624-233.333083 256.195939-270.345517l0.259919 70.801874 196.850484-114.640174L408.299406-0.001z"
            fill="currentColor"
          />
        </svg>
      </button>
      <button
        class="toolbar-icon-button lan-transfer-qr-button"
        type="button"
        :title="t('lanTransferQrTitle')"
        :aria-label="t('lanTransferQrTitle')"
        :disabled="busy || !running"
        @click="openReceiveLink()"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M4.5 4.5h5v5h-5zM14.5 4.5h5v5h-5zM4.5 14.5h5v5h-5zM14.5 14.5h2v2h-2zM17.5 14.5h2v2h-2zM14.5 17.5h2v2h-2zM17.5 17.5h2v2h-2z"
            fill="none"
            stroke="currentColor"
            stroke-width="1.7"
            stroke-linejoin="round"
          />
        </svg>
      </button>
      <button
        class="toolbar-icon-button lan-transfer-service-button"
        :class="{ running, danger: !running }"
        type="button"
        :disabled="busy"
        :title="running ? t('lanTransferStopService') : t('lanTransferStartService')"
        :aria-label="running ? t('lanTransferStopService') : t('lanTransferStartService')"
        @click="runAction(running ? onStopService : onStartService)"
      >
        <svg viewBox="0 0 1024 1024" aria-hidden="true">
          <path
            d="M652 125.54l0 71.8c140 63.74 211.28 193.28 211.28 342.82 0 212.64-171.62 384.98-384.24 384.98-212.6 0-380.56-172.34-380.56-384.98 0-149.22 93.52-278.56 213.52-342.42l0-71.84c-180 68.46-278.14 228.16-278.14 414.24 0 248.52 199.42 449.94 447.92 449.94 248.48 0 447.5-201.42 447.5-449.94 0-186.38-97.28-346.32-277.28-414.6zM512 412c0 22.08-17.92 40-40 40l0 0c-22.08 0-40-17.92-40-40l0-340c0-22.08 17.92-40 40-40l0 0c22.08 0 40 17.92 40 40l0 340z"
            fill="currentColor"
          />
        </svg>
      </button>
      <button
        class="toolbar-icon-button lan-transfer-settings-button"
        type="button"
        :title="t('settingsTitle')"
        :aria-label="t('settingsTitle')"
        @click="onOpenSettings"
      >
        <svg viewBox="0 0 1024 1024" aria-hidden="true">
          <path
            d="M816.64 551.936c1.536-12.8 2.56-26.112 2.56-39.936 0-13.824-1.024-27.136-3.072-39.936l86.528-67.584a21.162667 21.162667 0 0 0 5.12-26.112l-81.92-141.824a20.821333 20.821333 0 0 0-25.088-9.216l-101.888 40.96a299.946667 299.946667 0 0 0-69.12-39.936l-15.36-108.544a20.437333 20.437333 0 0 0-20.48-17.408h-163.84a19.925333 19.925333 0 0 0-19.968 17.408l-15.36 108.544a308.010667 308.010667 0 0 0-69.12 39.936l-101.888-40.96a20.266667 20.266667 0 0 0-25.088 9.216l-81.92 141.824a19.84 19.84 0 0 0 5.12 26.112l86.528 67.584c-2.048 12.8-3.584 26.624-3.584 39.936 0 13.312 1.024 27.136 3.072 39.936L121.344 619.52a21.162667 21.162667 0 0 0-5.12 26.112l81.92 141.824c5.12 9.216 15.872 12.288 25.088 9.216l101.888-40.96a299.946667 299.946667 0 0 0 69.12 39.936l15.36 108.544c2.048 10.24 10.24 17.408 20.48 17.408h163.84c10.24 0 18.944-7.168 19.968-17.408l15.36-108.544a308.010667 308.010667 0 0 0 69.12-39.936l101.888 40.96c9.216 3.584 19.968 0 25.088-9.216l81.92-141.824a19.84 19.84 0 0 0-5.12-26.112l-85.504-67.584zM512 665.6A154.026667 154.026667 0 0 1 358.4 512c0-84.48 69.12-153.6 153.6-153.6s153.6 69.12 153.6 153.6-69.12 153.6-153.6 153.6z"
            fill="currentColor"
          />
        </svg>
      </button>
    </header>

    <div class="lan-transfer-shell">
      <LanConversationList
        :active-fingerprint="activeFingerprint"
        :conversations="conversations"
        :drop-target-fingerprint="dropTargetFingerprint"
        :running="running"
        :t="t"
        @add-device="manualOpen = true"
        @select="selectPeer"
      />

      <main class="lan-transfer-content">
        <LanConversationPanel
          v-if="conversationPeer"
          :busy="busy"
          :locale="locale"
          :messages="messages"
          :on-load-preview="loadPreview"
          :peer="conversationPeer"
          :t="t"
          @back="leaveConversation"
          @cancel="cancelTransfer"
          @open-file="openReceivedFile"
          @resend="resendMessage"
          @reveal-file="revealReceivedFile"
        >
          <template #composer>
            <LanComposer
              :busy="busy"
              :draft="activeDraft"
              :running="running"
              :t="t"
              @add-files="sendPickedFiles(conversationPeer, false)"
              @add-folder="sendPickedFiles(conversationPeer, true)"
              @pick-clipboard="pickClipboardIntoDraft(conversationPeer.fingerprint)"
              @remove-item="removeDraftItem(conversationPeer.fingerprint, $event)"
              @send="submitDraft(conversationPeer)"
              @update:text="updateDraftText(conversationPeer.fingerprint, $event)"
            />
          </template>
        </LanConversationPanel>

        <LanIdlePanel
          v-else
          :busy="busy"
          :local-ips="localIps"
          :running="running"
          :state="state"
          :t="t"
          @open-link="openReceiveLink"
          @start-service="runAction(onStartService)"
        />
      </main>
    </div>

    <p v-if="pageWarning" class="lan-transfer-message warning">{{ pageWarning }}</p>
    <p v-if="pageError" class="lan-transfer-message error">{{ pageError }}</p>

    <div v-if="subnetMenuOpen" class="lan-subnet-popover">
      <LanSubnetPicker
        :busy="busy"
        :items="subnetItems"
        :last-subnet="lastSubnet"
        :t="t"
        @close="subnetMenuOpen = false"
        @select="selectSubnet"
      />
    </div>

    <div v-if="isFileDragOver" class="lan-transfer-drop-overlay">
      <strong>{{ dropHint }}</strong>
    </div>

    <div v-if="manualOpen" class="lan-transfer-text-modal" @click.self="manualOpen = false">
      <form class="lan-transfer-text-card" @submit.prevent="submitManual">
        <strong>{{ t("lanTransferManualSend") }}</strong>
        <input
          v-model="manualAddress"
          type="text"
          :placeholder="t('lanTransferIpPlaceholder')"
          autofocus
        />
        <div class="lan-transfer-text-actions">
          <button class="ghost compact" type="button" @click="manualOpen = false">
            {{ t("cancelAction") }}
          </button>
          <button class="primary compact" type="submit" :disabled="!manualAddress.trim()">
            {{ t("addAction") }}
          </button>
        </div>
      </form>
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
  padding: 0 14px 4px;
}

/* 顶栏整体压矮：按钮比全局尺寸略矮，图标仍居中，纵向留白也收紧。 */
.lan-transfer-topbar .toolbar-icon-button {
  height: 30px;
}

.lan-transfer-title {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.lan-transfer-title strong {
  font-size: 0.9rem;
}

/* 标题前的 LocalSend 官方图标：与标题文字同高，不参与伸缩。 */
.lan-transfer-title-icon {
  width: 18px;
  height: 18px;
  flex: 0 0 auto;
  object-fit: contain;
}

.lan-transfer-status-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 2px 7px;
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
  flex: 0 0 auto;
  background: transparent;
  box-shadow: none;
  transform: none;
}

.lan-transfer-service-button.running {
  /* 服务运行时用主题强调色；停止或启动失败时由全局 danger 类标色。 */
  color: var(--accent-primary);
}

/* 扫码互传入口：顶栏二维码按钮，手机扫码即可与本机互传。 */
.lan-transfer-qr-button {
  flex: 0 0 auto;
}

/* 设置入口：跳到设置页的「传输」分类，与本页的互传配置对应。 */
.lan-transfer-settings-button {
  flex: 0 0 auto;
}

/* 刷新发现：按钮在顶栏二维码左侧，扫描期间持续旋转，再点即取消扫描。 */
.lan-transfer-refresh-button {
  flex: 0 0 auto;
}

.lan-transfer-refresh-button.spinning svg {
  animation: lan-transfer-spin 900ms linear infinite;
}

@keyframes lan-transfer-spin {
  to {
    transform: rotate(360deg);
  }
}

.lan-transfer-shell {
  position: relative;
  display: flex;
  min-height: 0;
  border-top: 1px solid var(--app-panel-border);
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

/* 网段选择悬浮在顶栏刷新按钮下方，避免选中时挤压会话区域。 */
.lan-subnet-popover {
  position: absolute;
  top: 38px;
  /* 右侧还有二维码、服务开关与设置三个按钮：每多一个按钮右移一个按钮宽 + 间距。 */
  right: 102px;
  z-index: 30;
  width: min(320px, calc(100% - 20px));
  border-radius: 10px;
  box-shadow: 0 18px 42px rgba(0, 0, 0, 0.28);
}

.lan-transfer-pin-error {
  font-size: 0.72rem;
}

/* 拖拽提示不接收指针事件，否则 elementFromPoint 无法命中下方的会话项。 */
.lan-transfer-drop-overlay {
  position: absolute;
  inset: 0;
  z-index: 40;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.32);
  pointer-events: none;
}

.lan-transfer-drop-overlay strong {
  padding: 12px 20px;
  border: 1px dashed var(--accent-primary);
  border-radius: 14px;
  background: var(--app-select-menu-bg);
  color: var(--app-select-menu-text);
  font-size: 0.86rem;
}

.lan-transfer-text-modal {
  position: absolute;
  inset: 0;
  z-index: 50;
  display: grid;
  place-items: center;
  background: rgba(0, 0, 0, 0.32);
}

.lan-transfer-text-card {
  display: grid;
  gap: 10px;
  width: min(90%, 330px);
  padding: 15px;
  border-radius: 13px;
  background: var(--app-select-menu-bg);
  color: var(--app-select-menu-text);
  box-shadow: 0 20px 48px rgba(0, 0, 0, 0.3);
}

.lan-transfer-text-card > small {
  color: var(--app-muted);
  font-size: 0.72rem;
}

.lan-transfer-text-card input {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--app-panel-border);
  border-radius: 8px;
  background: var(--app-input-bg);
  color: var(--app-text);
  font: inherit;
  font-size: 0.8rem;
}

.lan-transfer-text-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}

/* 窄窗降级为单栏：未选会话显示列表，选中会话显示对话视图。 */
@media (max-width: 699px) {
  .lan-transfer-shell {
    display: block;
  }

  .lan-transfer-page:not(.has-peer) .lan-transfer-content {
    display: none;
  }

  .lan-transfer-page.has-peer :deep(.lan-conversation-list) {
    display: none;
  }

  /* 单栏时列表独占整屏，内部滚动区域才能拿到高度。 */
  .lan-transfer-page:not(.has-peer) :deep(.lan-conversation-list) {
    height: 100%;
  }
}
</style>
