<script setup>
// 局域网互传页面：展示服务状态、附近设备、传输进度、已接收文件与浏览器扫码入口。
import { open } from "@tauri-apps/plugin-dialog";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { lanErrorCode, lanErrorText, lanWarningText } from "../utils/lanError";

const props = defineProps({
    busy: { type: Boolean, required: true },
    error: { type: String, default: "" },
    onAddDevice: { type: Function, required: true },
    onBack: { type: Function, required: true },
    onCancelTransfer: { type: Function, required: true },
    onOpenFile: { type: Function, required: true },
    onRefreshDevices: { type: Function, required: true },
    onRevealFile: { type: Function, required: true },
    onSendFiles: { type: Function, required: true },
    onSendText: { type: Function, required: true },
    onSetWebMode: { type: Function, required: true },
    onStart: { type: Function, required: true },
    onStartService: { type: Function, required: true },
    onStopService: { type: Function, required: true },
    platform: { type: String, default: "" },
    state: { type: Object, required: true },
    t: { type: Function, required: true },
});

const manualAddress = ref("");
const localError = ref("");
const textTarget = ref(null);
const textDraft = ref("");
// 对端要求 PIN 时暂存本次发送参数，输入 PIN 后重试。
const pinPrompt = ref(null);
const pinDraft = ref("");
const isFileDragOver = ref(false);
let unlistenDragDrop = null;

const devices = computed(() =>
    Array.isArray(props.state.devices) ? props.state.devices : [],
);
const transfers = computed(() =>
    (Array.isArray(props.state.transfers) ? props.state.transfers : [])
        .slice()
        .reverse(),
);
const receivedFiles = computed(() =>
    Array.isArray(props.state.receivedFiles) ? props.state.receivedFiles : [],
);
const running = computed(() => props.state.status === "running");
const failed = computed(() => props.state.status === "error");
const webMode = computed(() => props.state.webMode || "none");
const webUrl = computed(() => props.state.webUrl || "");
const statusLabel = computed(() => {
    if (failed.value) {
        return lanErrorText(props.t, props.state.errorCode, props.state.error);
    }
    return running.value
        ? props.t("lanTransferStatusRunning")
        : props.t("lanTransferStatusStopped");
});

function transferErrorText(transfer) {
    return lanErrorText(props.t, lanErrorCode(transfer.error), transfer.error);
}

// 页面级错误/警告：后端只给错误码与原始细节，这里统一本地化。
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

function formatBytes(size) {
    const value = Number(size || 0);
    if (!value) {
        return "0 B";
    }
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
    return Math.max(0, Math.min(100, Math.round((Number(transfer.doneBytes || 0) / total) * 100)));
}

function transferStatusLabel(transfer) {
    const map = {
        active: props.t("lanTransferStatusActive"),
        done: props.t("lanTransferStatusDone"),
        failed: props.t("lanTransferStatusFailed"),
        cancelled: props.t("lanTransferStatusCancelled"),
    };
    return map[transfer.status] || transfer.status;
}

function deviceSubtitle(device) {
    const model = device.deviceModel ? `${device.deviceModel} · ` : "";
    return `${model}${device.host}:${device.port}`;
}

async function run(action) {
    localError.value = "";
    try {
        await action();
    } catch (error) {
        localError.value = error?.message || String(error);
    }
}

async function refreshDevices() {
    await run(props.onRefreshDevices);
}

async function addDevice() {
    const address = manualAddress.value.trim();
    if (!address) {
        return;
    }
    await run(async () => {
        await props.onAddDevice(address);
        manualAddress.value = "";
    });
}

async function chooseAndSend(device) {
    const selected = await open({ multiple: true, directory: false });
    const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
    if (!paths.length) {
        return;
    }
    await attemptSend("files", device, paths);
}

function openTextComposer(device) {
    textTarget.value = device;
    textDraft.value = "";
}

function closeTextComposer() {
    textTarget.value = null;
    textDraft.value = "";
}

async function submitText() {
    const device = textTarget.value;
    const text = textDraft.value.trim();
    if (!device || !text) {
        return;
    }
    const sent = await attemptSend("text", device, text);
    if (sent) {
        closeTextComposer();
    }
}

// 发送文件或文本；对端要求 PIN 时弹出输入框并保留本次参数。
async function attemptSend(kind, device, payload, pin = "") {
    localError.value = "";
    try {
        if (kind === "files") {
            await props.onSendFiles(device.fingerprint, payload, pin || null);
        } else {
            await props.onSendText(device.fingerprint, payload, pin || null);
        }
        return true;
    } catch (error) {
        const detail = error?.message || String(error);
        if (lanErrorCode(detail) === "pin_required") {
            pinPrompt.value = {
                device,
                kind,
                payload,
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
    const sent = await attemptSend(prompt.kind, prompt.device, prompt.payload, pin);
    if (sent) {
        closePinPrompt();
        if (prompt.kind === "text") {
            closeTextComposer();
        }
    }
}

function closePinPrompt() {
    pinPrompt.value = null;
    pinDraft.value = "";
}

async function toggleWebMode(mode) {
    if (webMode.value === mode) {
        await run(() => props.onSetWebMode("none", []));
        return;
    }
    // 分享模式需要先选择要分享的文件。
    if (mode === "share") {
        const selected = await open({ multiple: true, directory: false });
        const paths = Array.isArray(selected) ? selected : selected ? [selected] : [];
        if (!paths.length) {
            return;
        }
        await run(() => props.onSetWebMode("share", paths));
        return;
    }
    await run(() => props.onSetWebMode(mode, []));
}

async function copyWebUrl() {
    if (!webUrl.value) {
        return;
    }
    try {
        await navigator.clipboard.writeText(webUrl.value);
    } catch (error) {
        localError.value = error?.message || String(error);
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
    const target = devices.value[0];
    if (!target) {
        localError.value = props.t("lanTransferNoDevices");
        return;
    }
    await run(() => props.onSendFiles(target.fingerprint, payload.paths));
}

onMounted(async () => {
    unlistenDragDrop = await getCurrentWindow().onDragDropEvent((event) => {
        void handleDrop(event);
    });
    await props.onStart();
});

onUnmounted(() => {
    unlistenDragDrop?.();
});
</script>

<template>
    <section class="lan-transfer-page">
        <header class="lan-transfer-topbar">
            <button
                class="toolbar-icon-button lan-transfer-back"
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
                <h1>{{ t("lanTransferTitle") }}</h1>
            </div>
            <button
                v-if="running || state.enabled !== false"
                class="toolbar-icon-button lan-transfer-service-button"
                :class="{ running }"
                type="button"
                :disabled="busy"
                :title="
                    running
                        ? t('lanTransferStopService')
                        : t('lanTransferStartService')
                "
                :aria-label="
                    running
                        ? t('lanTransferStopService')
                        : t('lanTransferStartService')
                "
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

        <section class="lan-transfer-status">
            <span class="lan-transfer-status-pill">
                <i
                    class="lan-transfer-status-dot"
                    :class="{
                        connected: running,
                        disconnected: !running,
                    }"
                    aria-hidden="true"
                ></i>
                {{ statusLabel }}
            </span>
            <span v-if="running" class="lan-transfer-status-meta">
                {{ state.alias }} · {{ state.port }}
            </span>
        </section>

        <section class="lan-transfer-web">
            <div class="lan-transfer-tabs">
                <button
                    type="button"
                    :class="{ active: webMode === 'share' }"
                    :disabled="busy || !running"
                    @click="toggleWebMode('share')"
                >
                    {{ t("lanTransferWebShare") }}
                </button>
                <button
                    type="button"
                    :class="{ active: webMode === 'receive' }"
                    :disabled="busy || !running"
                    @click="toggleWebMode('receive')"
                >
                    {{ t("lanTransferWebReceive") }}
                </button>
            </div>
            <div v-if="webUrl" class="lan-transfer-web-body">
                <div class="lan-transfer-qr" v-html="state.webQrSvg"></div>
                <div class="lan-transfer-link-panel">
                    <div class="lan-transfer-url-row">
                        <a
                            class="lan-transfer-url"
                            :href="webUrl"
                            target="_blank"
                            rel="noreferrer"
                            :title="webUrl"
                        >
                            {{ webUrl }}
                        </a>
                        <button
                            class="toolbar-icon-button lan-transfer-copy-link"
                            type="button"
                            :title="t('copy')"
                            :aria-label="t('copy')"
                            @click="copyWebUrl"
                        >
                            <svg viewBox="0 0 24 24" aria-hidden="true">
                                <path
                                    d="M8 8h9v11H8V8Zm-3 8V5h9"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.9"
                                    stroke-linejoin="round"
                                />
                            </svg>
                        </button>
                    </div>
                    <small class="lan-transfer-web-hint">
                        {{
                            webMode === "share"
                                ? t("lanTransferWebShareHint")
                                : t("lanTransferWebReceiveHint")
                        }}
                    </small>
                    <button
                        class="ghost compact"
                        type="button"
                        :disabled="busy"
                        @click="toggleWebMode(webMode)"
                    >
                        {{ t("lanTransferWebStop") }}
                    </button>
                </div>
            </div>
            <p v-else class="lan-transfer-web-hint">
                {{
                    state.enabled === false
                        ? t("lanTransferDisabledHint")
                        : t("lanTransferWebIdleHint")
                }}
            </p>
        </section>

        <div class="lan-transfer-body">
            <section class="lan-transfer-devices">
                <div class="lan-transfer-section-head">
                    <span class="meta-label">{{ t("lanTransferDevices") }}</span>
                    <button
                        class="ghost compact lan-transfer-refresh"
                        type="button"
                        :title="t('lanTransferRefresh')"
                        :aria-label="t('lanTransferRefresh')"
                        :disabled="busy || !running"
                        @click="refreshDevices"
                    >
                        <svg viewBox="0 0 1024 1024" aria-hidden="true">
                            <path
                                d="M958.681412 457.499032c-6.170072-50.632177-20.854483-99.563886-43.643361-145.434552-45.779694-92.144205-122.249797-166.333021-215.325711-208.898719-20.083724-9.18513-43.810309-0.349891-52.995439 19.734833-9.18413 20.082724-0.349891 43.810309 19.733833 52.996438 159.26323 72.834239 245.755201 249.640987 205.658732 420.410622-30.735395 130.876101-129.201624 233.321087-256.187941 270.333521l-0.262918-70.800875-196.843487 114.650172 197.690222 113.176632-0.275914-74.43274c75.398438-17.911403 144.809747-54.929834 202.084849-108.039237 65.597501-60.827991 111.122274-139.186504 131.651859-226.606186 12.170197-51.828803 15.10328-104.683286 8.715276-157.089909zM408.299406-0.001l0.271915 74.43374c-75.404436 17.911403-144.820744 54.931834-202.099843 108.046235-65.6005 60.83099-111.124274 139.191503-131.651859 226.616183-7.987504 34.034364-11.994252 68.507591-11.994252 103.010809 0 17.994377 1.090659 35.996751 3.271978 53.946142 6.152077 50.59119 20.803499 99.48891 43.545392 145.333583 45.678725 92.080225 122.012871 166.270041 214.936832 208.900718 20.071728 9.209122 43.810309 0.401874 53.018432-19.670852 9.210122-20.076726 0.400875-43.810309-19.671853-53.019432-158.963324-72.92821-245.278351-249.658982-205.24886-420.22368 30.732396-130.883099 129.201624-233.333083 256.195939-270.345517l0.259919 70.801874 196.850484-114.640174L408.299406-0.001z"
                                fill="currentColor"
                            />
                        </svg>
                    </button>
                </div>
                <div class="lan-transfer-add">
                    <input
                        v-model="manualAddress"
                        type="text"
                        :placeholder="t('lanTransferIpPlaceholder')"
                        @keydown.enter.prevent="addDevice"
                    />
                    <button
                        class="ghost compact"
                        type="button"
                        :disabled="busy || !running || !manualAddress.trim()"
                        @click="addDevice"
                    >
                        {{ t("lanTransferAddDevice") }}
                    </button>
                </div>
                <p v-if="!devices.length" class="lan-transfer-empty">
                    {{ t("lanTransferNoDevices") }}
                </p>
                <article
                    v-for="device in devices"
                    :key="device.fingerprint"
                    class="lan-transfer-device"
                >
                    <div class="lan-transfer-device-info">
                        <strong>{{ device.alias }}</strong>
                        <small>{{ deviceSubtitle(device) }}</small>
                        <span v-if="device.trusted" class="lan-transfer-trusted">
                            {{ t("lanTransferTrusted") }}
                        </span>
                    </div>
                    <div class="lan-transfer-device-actions">
                        <button
                            class="ghost compact"
                            type="button"
                            :disabled="busy || !running"
                            @click="chooseAndSend(device)"
                        >
                            {{ t("lanTransferSendFile") }}
                        </button>
                        <button
                            class="ghost compact"
                            type="button"
                            :disabled="busy || !running"
                            @click="openTextComposer(device)"
                        >
                            {{ t("lanTransferSendText") }}
                        </button>
                    </div>
                </article>
            </section>

            <section v-if="transfers.length" class="lan-transfer-transfers">
                <div class="lan-transfer-section-head">
                    <span class="meta-label">{{ t("lanTransferTransfers") }}</span>
                </div>
                <article
                    v-for="transfer in transfers"
                    :key="transfer.id"
                    class="lan-transfer-transfer"
                >
                    <div class="lan-transfer-transfer-info">
                        <strong>{{ transfer.label }}</strong>
                        <small>
                            {{ transfer.peerAlias }} ·
                            {{ transferStatusLabel(transfer) }}
                            <template v-if="transfer.status === 'active'">
                                ({{ progressOf(transfer) }}%)
                            </template>
                        </small>
                        <progress
                            v-if="transfer.status === 'active'"
                            :value="progressOf(transfer)"
                            max="100"
                        ></progress>
                        <small v-if="transfer.error">{{
                            transferErrorText(transfer)
                        }}</small>
                    </div>
                    <button
                        v-if="transfer.status === 'active'"
                        class="ghost compact"
                        type="button"
                        @click="run(() => onCancelTransfer(transfer.id))"
                    >
                        {{ t("lanTransferCancel") }}
                    </button>
                </article>
            </section>

            <section v-if="receivedFiles.length" class="lan-transfer-received">
                <div class="lan-transfer-section-head">
                    <span class="meta-label">{{ t("lanTransferReceived") }}</span>
                </div>
                <article
                    v-for="file in receivedFiles"
                    :key="file.id"
                    class="lan-transfer-received-item"
                >
                    <div>
                        <strong>{{ file.fileName }}</strong>
                        <small>
                            {{ file.fromAlias }} · {{ formatBytes(file.size) }}
                        </small>
                    </div>
                    <div class="lan-transfer-device-actions">
                        <button
                            class="ghost compact"
                            type="button"
                            @click="run(() => onOpenFile(file.id))"
                        >
                            {{ t("openAction") }}
                        </button>
                        <button
                            class="ghost compact"
                            type="button"
                            @click="run(() => onRevealFile(file.id))"
                        >
                            {{ t("revealInExplorer") }}
                        </button>
                    </div>
                </article>
            </section>
        </div>

        <p v-if="pageWarning" class="lan-transfer-warning">
            {{ pageWarning }}
        </p>
        <p v-if="pageError" class="lan-transfer-error">
            {{ pageError }}
        </p>

        <div v-if="isFileDragOver" class="lan-transfer-drop-overlay">
            <strong>{{ t("lanTransferDropFiles") }}</strong>
        </div>

        <div v-if="textTarget" class="lan-transfer-text-modal" @click.self="closeTextComposer">
            <div class="lan-transfer-text-card">
                <strong>{{ t("lanTransferSendTextTitle", { name: textTarget.alias }) }}</strong>
                <textarea
                    v-model="textDraft"
                    rows="4"
                    :placeholder="t('lanTransferSendTextPlaceholder')"
                ></textarea>
                <div class="lan-transfer-text-actions">
                    <button class="ghost compact" type="button" @click="closeTextComposer">
                        {{ t("cancelAction") }}
                    </button>
                    <button
                        class="primary compact"
                        type="button"
                        :disabled="busy || !textDraft.trim()"
                        @click="submitText"
                    >
                        {{ t("lanTransferSend") }}
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
                <small v-if="pinPrompt.invalid" class="lan-transfer-error">
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
    </section>
</template>
