import { computed, onUnmounted, ref } from "vue";
import {
  getLanReceiverState,
  onLanReceiverStatus,
  openLanTransferFile,
  revealLanTransferFile,
  sendLanTransferFile,
  sendLanTransferText,
  startLanReceiver,
  stopLanReceiver,
} from "../services/tauriApi";

function formatError(error) {
  if (typeof error === "string") {
    return error;
  }
  if (error && typeof error.message === "string") {
    return error.message;
  }
  return String(error || "");
}

function readFileBytes(file, onProgress) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onprogress = (event) => {
      if (event.lengthComputable) {
        onProgress?.(
          Math.max(
            1,
            Math.min(95, Math.round((event.loaded / event.total) * 95)),
          ),
        );
      }
    };
    reader.onerror = () =>
      reject(reader.error || new Error("file_read_failed"));
    reader.onload = () => {
      onProgress?.(96);
      resolve(Array.from(new Uint8Array(reader.result)));
    };
    reader.readAsArrayBuffer(file);
  });
}

export function useLanReceiver({ t }) {
  const showLanReceiver = ref(false);
  const lanReceiverState = ref({
    running: false,
    url: null,
    qrSvg: null,
    ip: null,
    port: null,
    token: null,
    expiresAt: null,
    lastStatus: null,
    connectedDevices: 0,
    messages: [],
  });
  const lanReceiverBusy = ref(false);
  const lanReceiverError = ref("");
  const now = ref(Date.now());
  let unlisten = null;
  let timer = null;

  const expiresInSeconds = computed(() => {
    const expiresAt = Number(lanReceiverState.value?.expiresAt || 0);
    if (!expiresAt) {
      return null;
    }
    return Math.max(0, Math.ceil((expiresAt - now.value) / 1000));
  });

  const statusLabel = computed(() => {
    const status = lanReceiverState.value?.lastStatus;
    if (!status) {
      return lanReceiverState.value.running
        ? t("lanReceiverReady")
        : t("lanReceiverStopped");
    }
    if (status.kind === "success") {
      if (status.receivedKind === "image") {
        return t("lanReceiverReceivedImage");
      }
      if (status.receivedKind === "file") {
        return t("lanReceiverReceivedFile");
      }
      return t("lanReceiverReceivedText");
    }
    if (status.kind === "processing") {
      return t("lanReceiverProcessingImage");
    }
    return status.message || t("lanReceiverFailed");
  });
  const hasActiveSession = computed(() => {
    if (!lanReceiverState.value.running) {
      return false;
    }

    const expiresAt = Number(lanReceiverState.value?.expiresAt || 0);
    return !expiresAt || expiresAt > Date.now();
  });

  function applyState(next) {
    lanReceiverState.value = {
      running: Boolean(next?.running),
      url: next?.url || null,
      qrSvg: next?.qrSvg || null,
      ip: next?.ip || null,
      port: next?.port || null,
      token: next?.token || null,
      expiresAt: next?.expiresAt || null,
      lastStatus: next?.lastStatus || null,
      connectedDevices: Number(next?.connectedDevices || 0),
      messages: Array.isArray(next?.messages) ? next.messages : [],
    };
  }

  async function setupLanReceiverListener() {
    if (unlisten) {
      return;
    }
    unlisten = await onLanReceiverStatus((event) => {
      if (event?.payload) {
        applyState(event.payload);
      }
    });
  }

  async function refreshLanReceiverState() {
    applyState(await getLanReceiverState());
  }

  async function openLanReceiver() {
    showLanReceiver.value = true;
    lanReceiverError.value = "";
    lanReceiverBusy.value = true;
    try {
      await setupLanReceiverListener();
      await refreshLanReceiverState();
      if (!hasActiveSession.value) {
        applyState(await startLanReceiver());
      }
    } catch (error) {
      lanReceiverError.value = formatError(error);
    } finally {
      lanReceiverBusy.value = false;
    }
  }

  async function closeLanReceiver() {
    lanReceiverError.value = "";
    lanReceiverBusy.value = true;
    try {
      applyState(await stopLanReceiver());
      showLanReceiver.value = false;
    } catch (error) {
      lanReceiverError.value = formatError(error);
    } finally {
      lanReceiverBusy.value = false;
    }
  }

  async function sendDesktopText(text) {
    lanReceiverError.value = "";
    lanReceiverBusy.value = true;
    try {
      applyState(await sendLanTransferText(text));
    } catch (error) {
      lanReceiverError.value = formatError(error);
      throw error;
    } finally {
      lanReceiverBusy.value = false;
    }
  }

  async function sendDesktopFile(file, onProgress) {
    lanReceiverError.value = "";
    lanReceiverBusy.value = true;
    try {
      const bytes = await readFileBytes(file, onProgress);
      onProgress?.(98);
      applyState(
        await sendLanTransferFile(
          file.name || "transfer-file",
          file.type || "application/octet-stream",
          bytes,
        ),
      );
      onProgress?.(100);
    } catch (error) {
      lanReceiverError.value = formatError(error);
      throw error;
    } finally {
      lanReceiverBusy.value = false;
    }
  }

  async function openTransferFile(id) {
    await openLanTransferFile(id);
  }

  async function revealTransferFile(id) {
    await revealLanTransferFile(id);
  }

  timer = window.setInterval(() => {
    now.value = Date.now();
    if (
      lanReceiverState.value.running &&
      expiresInSeconds.value !== null &&
      expiresInSeconds.value <= 0
    ) {
      refreshLanReceiverState();
    }
  }, 1000);

  onUnmounted(() => {
    if (timer) {
      window.clearInterval(timer);
    }
    unlisten?.();
  });

  return {
    closeLanReceiver,
    lanReceiverBusy,
    lanReceiverError,
    lanReceiverState,
    openLanReceiver,
    refreshLanReceiverState,
    openTransferFile,
    revealTransferFile,
    sendDesktopFile,
    sendDesktopText,
    showLanReceiver,
    statusLabel,
  };
}
