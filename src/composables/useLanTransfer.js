import { getCurrentWindow } from "@tauri-apps/api/window";
import { computed, onUnmounted, ref, watch } from "vue";
import {
  addLanDevice,
  cancelLanScan,
  cancelLanTransfer,
  getLanTransferState,
  inspectLanSelection,
  listLanSubnets,
  onLanTransferState,
  openLanReceivedFile,
  readLanClipboard,
  refreshLanDevices,
  removeLanTrustedDevice,
  respondLanRequest,
  revealLanReceivedFile,
  scanLanSubnets,
  sendLanFiles,
  sendLanItems,
  sendLanText,
  setLanWebMode,
  startLanTransfer,
  stopLanTransfer,
} from "../services/tauriApi";

// 默认状态：尚未启动服务时的空快照。
function emptyState() {
  return {
    status: "stopped",
    error: null,
    errorCode: null,
    warning: null,
    enabled: true,
    alias: "",
    port: 53317,
    fingerprint: null,
    devices: [],
    transfers: [],
    incoming: null,
    receivedFiles: [],
    webMode: "none",
    webUrl: null,
    webQrSvg: null,
    scan: null,
  };
}

function formatError(error) {
  if (typeof error === "string") {
    return error;
  }
  if (error && typeof error.message === "string") {
    return error.message;
  }
  return String(error || "");
}

function applyState(next) {
  return {
    ...emptyState(),
    ...(next || {}),
  };
}

// 局域网互传的前端状态与动作封装。
export function useLanTransfer() {
  const lanTransferState = ref(emptyState());
  const lanTransferBusy = ref(false);
  const lanTransferError = ref("");
  let unlisten = null;

  const lanRunning = computed(() => lanTransferState.value.status === "running");
  const lanDevices = computed(() => lanTransferState.value.devices || []);
  const lanTransfers = computed(() => lanTransferState.value.transfers || []);
  const lanIncoming = computed(() => lanTransferState.value.incoming || null);
  const lanReceivedFiles = computed(() => lanTransferState.value.receivedFiles || []);
  const lanScan = computed(() => lanTransferState.value.scan || null);

  async function setupListener() {
    if (unlisten) {
      return;
    }
    unlisten = await onLanTransferState((event) => {
      if (event?.payload) {
        lanTransferState.value = applyState(event.payload);
      }
    });
  }

  async function run(action) {
    lanTransferError.value = "";
    lanTransferBusy.value = true;
    try {
      const next = await action();
      if (next) {
        lanTransferState.value = applyState(next);
      }
      return next;
    } catch (error) {
      lanTransferError.value = formatError(error);
      throw error;
    } finally {
      lanTransferBusy.value = false;
    }
  }

  // 进入互传页面时调用：注册事件监听并同步一次状态，必要时自动启动服务。
  async function openLanTransfer() {
    lanTransferError.value = "";
    try {
      await setupListener();
      lanTransferState.value = applyState(await getLanTransferState());
      if (lanTransferState.value.enabled && lanTransferState.value.status !== "running") {
        await run(() => startLanTransfer());
      }
    } catch (error) {
      lanTransferError.value = formatError(error);
    }
  }

  function refreshState() {
    return run(getLanTransferState);
  }

  // 应用启动即订阅互传状态：服务可能在后台运行，用户还没进过互传页时
  // 也要能感知待确认请求并唤起面板。这里只订阅与取状态，不启动服务。
  async function startLanStateSync() {
    try {
      await setupListener();
      lanTransferState.value = applyState(await getLanTransferState());
    } catch (error) {
      lanTransferError.value = formatError(error);
    }
  }

  // 后端只在需要用户确认时才写入 incoming，而确认窗口只有 45 秒；
  // 面板被隐藏（托盘应用中很常见）时用户看不到弹窗，请求会超时被自动拒绝，
  // 因此新请求出现时统一把面板带到前台。
  async function surfacePanelForIncomingRequest() {
    try {
      const appWindow = getCurrentWindow();
      await appWindow.show();
      // 未最小化时部分平台会报错，这里忽略：唤起失败不应影响其它步骤。
      await Promise.resolve(appWindow.unminimize()).catch(() => {});
      await appWindow.setFocus();
    } catch (error) {
      console.error("Failed to surface the panel for an incoming request", error);
    }
  }

  let surfacedRequestId = "";

  watch(
    () => lanTransferState.value.incoming?.requestId || "",
    (requestId) => {
      if (!requestId || requestId === surfacedRequestId) {
        return;
      }
      surfacedRequestId = requestId;
      void surfacePanelForIncomingRequest();
    },
    { immediate: true },
  );

  function refreshDevices() {
    return run(refreshLanDevices);
  }

  // 网段候选不进状态快照：返回的是候选列表而不是完整服务状态，不走 run。
  async function listSubnets() {
    lanTransferError.value = "";
    try {
      return await listLanSubnets();
    } catch (error) {
      lanTransferError.value = formatError(error);
      throw error;
    }
  }

  function scanSubnets(subnets, port = null) {
    return run(() => scanLanSubnets(subnets, port));
  }

  function cancelScan() {
    return run(cancelLanScan);
  }

  function addDevice(host, port) {
    return run(() => addLanDevice(host, port));
  }

  function sendFiles(fingerprint, paths, pin = null) {
    return run(() => sendLanFiles(fingerprint, paths, pin));
  }

  function sendText(fingerprint, text, pin = null) {
    return run(() => sendLanText(fingerprint, text, pin));
  }

  function sendItems(fingerprint, items, pin = null) {
    return run(() => sendLanItems(fingerprint, items, pin));
  }

  async function inspectSelection(paths) {
    lanTransferError.value = "";
    try {
      return await inspectLanSelection(paths);
    } catch (error) {
      lanTransferError.value = formatError(error);
      throw error;
    }
  }

  async function readClipboard() {
    lanTransferError.value = "";
    try {
      return await readLanClipboard();
    } catch (error) {
      lanTransferError.value = formatError(error);
      throw error;
    }
  }

  function respond(requestId, decision) {
    return run(() => respondLanRequest(requestId, decision));
  }

  function cancelTransfer(transferId) {
    return run(() => cancelLanTransfer(transferId));
  }

  function setWebMode(mode, paths = []) {
    return run(() => setLanWebMode(mode, paths));
  }

  async function openReceivedFile(id) {
    await openLanReceivedFile(id);
  }

  async function revealReceivedFile(id) {
    await revealLanReceivedFile(id);
  }

  async function removeTrustedDevice(fingerprint) {
    await removeLanTrustedDevice(fingerprint);
    await refreshState();
  }

  async function stopLanTransferService() {
    return run(stopLanTransfer);
  }

  async function startLanTransferService() {
    return run(startLanTransfer);
  }

  onUnmounted(() => {
    unlisten?.();
    unlisten = null;
  });

  return {
    addDevice,
    cancelScan,
    cancelTransfer,
    lanDevices,
    lanIncoming,
    lanReceivedFiles,
    lanRunning,
    lanScan,
    lanTransferBusy,
    lanTransferError,
    lanTransferState,
    lanTransfers,
    listSubnets,
    openLanTransfer,
    openReceivedFile,
    refreshDevices,
    refreshState,
    removeTrustedDevice,
    respond,
    revealReceivedFile,
    scanSubnets,
    inspectSelection,
    readClipboard,
    sendFiles,
    sendItems,
    sendText,
    setWebMode,
    startLanStateSync,
    startLanTransferService,
    stopLanTransferService,
  };
}
