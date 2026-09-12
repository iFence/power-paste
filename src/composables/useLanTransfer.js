import { computed, onUnmounted, ref } from "vue";
import {
  addLanDevice,
  cancelLanTransfer,
  getLanTransferState,
  onLanTransferState,
  openLanReceivedFile,
  refreshLanDevices,
  removeLanTrustedDevice,
  respondLanRequest,
  revealLanReceivedFile,
  sendLanFiles,
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

  function refreshDevices() {
    return run(refreshLanDevices);
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
    cancelTransfer,
    lanDevices,
    lanIncoming,
    lanReceivedFiles,
    lanRunning,
    lanTransferBusy,
    lanTransferError,
    lanTransferState,
    lanTransfers,
    openLanTransfer,
    openReceivedFile,
    refreshDevices,
    refreshState,
    removeTrustedDevice,
    respond,
    revealReceivedFile,
    sendFiles,
    sendText,
    setWebMode,
    startLanTransferService,
    stopLanTransferService,
  };
}
