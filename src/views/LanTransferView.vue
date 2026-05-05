<script setup>
import {
    computed,
    nextTick,
    onMounted,
    ref,
    watch,
} from "vue";

const props = defineProps({
    busy: { type: Boolean, required: true },
    error: { type: String, default: "" },
    onBack: { type: Function, required: true },
    onOpenFile: { type: Function, required: true },
    onRevealFile: { type: Function, required: true },
    onSendFile: { type: Function, required: true },
    onSendText: { type: Function, required: true },
    onStart: { type: Function, required: true },
    state: { type: Object, required: true },
    statusLabel: { type: String, required: true },
    t: { type: Function, required: true },
});

const draft = ref("");
const fileInputRef = ref(null);
const messagesRef = ref(null);
const localError = ref("");
const pendingMessages = ref([]);
const contextMenu = ref({
    show: false,
    x: 0,
    y: 0,
    message: null,
});

const messages = computed(() => [
    ...(Array.isArray(props.state.messages) ? props.state.messages : []),
    ...pendingMessages.value,
]);
const transferUrl = computed(() => props.state.url || "");
const isTransferConnected = computed(
    () => props.state.running && Number(props.state.connectedDevices || 0) > 0,
);
const connectionLabel = computed(() =>
    isTransferConnected.value
        ? props.t("lanTransferConnected")
        : props.t("lanTransferDisconnected"),
);
const canSendText = computed(
    () => draft.value.trim().length > 0 && props.state.running && !props.busy,
);

function formatBytes(size) {
    const value = Number(size || 0);
    if (!value) {
        return "";
    }
    if (value < 1000) {
        return `${value} B`;
    }
    if (value < 1_000_000) {
        return `${Math.round(value / 1000)} KB`;
    }
    return `${(value / 1_000_000).toFixed(1)} MB`;
}

function avatarLabel(sender) {
    return sender === "desktop"
        ? props.t("lanTransferDesktop")
        : props.t("lanTransferPhone").slice(0, 1);
}

function transferProgress(message) {
    return Math.max(0, Math.min(100, Number(message.progress || 0)));
}

function transferStatusLabel(message) {
    if (message.status === "failed") {
        return props.t("lanTransferUploadFailed");
    }
    if (message.status === "uploading") {
        return props.t("lanTransferUploading", {
            progress: transferProgress(message),
        });
    }
    return "";
}

function upsertPendingMessage(id, patch) {
    pendingMessages.value = pendingMessages.value.map((message) =>
        message.id === id ? { ...message, ...patch } : message,
    );
}

function removePendingMessage(id) {
    pendingMessages.value = pendingMessages.value.filter(
        (message) => message.id !== id,
    );
}

async function scrollToBottom() {
    await nextTick();
    if (messagesRef.value) {
        messagesRef.value.scrollTop = messagesRef.value.scrollHeight;
    }
}

async function sendText() {
    const text = draft.value.trim();
    if (!text || props.busy) {
        return;
    }
    localError.value = "";
    draft.value = "";
    try {
        await props.onSendText(text);
        await scrollToBottom();
    } catch (error) {
        localError.value = error?.message || String(error);
    }
}

async function copyTransferUrl() {
    if (!transferUrl.value) {
        return;
    }
    localError.value = "";
    try {
        await navigator.clipboard.writeText(transferUrl.value);
    } catch (error) {
        localError.value = error?.message || String(error);
    }
}

async function handleFileChange(event) {
    const selectedFiles = Array.from(event.target.files || []);
    event.target.value = "";
    if (!selectedFiles.length || props.busy) {
        return;
    }

    const files = selectedFiles.slice(0, 9);
    localError.value =
        selectedFiles.length > 9
            ? props.t("lanTransferTooManyFiles", { max: 9 })
            : "";

    for (const [index, file] of files.entries()) {
        const id = `desktop-upload-${Date.now()}-${index}`;
        pendingMessages.value = [
            ...pendingMessages.value,
            {
                id,
                sender: "desktop",
                kind: file.type?.startsWith("image/") ? "image" : "file",
                fileName: file.name || "transfer-file",
                mimeType: file.type || "application/octet-stream",
                size: file.size,
                progress: 0,
                status: "uploading",
                hasLocalFile: false,
            },
        ];
        await scrollToBottom();

        try {
            await props.onSendFile(file, (progress) =>
                upsertPendingMessage(id, { progress }),
            );
            removePendingMessage(id);
            await scrollToBottom();
        } catch (error) {
            upsertPendingMessage(id, {
                progress: 100,
                status: "failed",
                text: error?.message || String(error),
            });
            localError.value = error?.message || String(error);
        }
    }
}

function chooseFile() {
    fileInputRef.value?.click();
}

function closeContextMenu() {
    contextMenu.value = {
        show: false,
        x: 0,
        y: 0,
        message: null,
    };
}

function openFileMenu(event, message) {
    if (event.target instanceof Element && event.target.closest("img")) {
        return;
    }
    if (!message.hasLocalFile) {
        return;
    }
    event.preventDefault();
    contextMenu.value = {
        show: true,
        x: event.clientX,
        y: event.clientY,
        message,
    };
}

async function handleOpenContextFile() {
    const message = contextMenu.value.message;
    closeContextMenu();
    if (!message) {
        return;
    }
    try {
        await props.onOpenFile(message.id);
    } catch (error) {
        localError.value = error?.message || String(error);
    }
}

async function handleRevealContextFile() {
    const message = contextMenu.value.message;
    closeContextMenu();
    if (!message) {
        return;
    }
    try {
        await props.onRevealFile(message.id);
    } catch (error) {
        localError.value = error?.message || String(error);
    }
}

async function goBack() {
    await props.onBack();
}

onMounted(async () => {
    localError.value = "";
    try {
        await props.onStart();
        await scrollToBottom();
    } catch (error) {
        localError.value = error?.message || String(error);
    }
});

watch(messages, scrollToBottom, { deep: true });
</script>

<template>
    <section
        class="lan-transfer-page"
        @click="closeContextMenu"
        @contextmenu.self.prevent="closeContextMenu"
    >
        <header class="lan-transfer-topbar">
            <button
                class="toolbar-icon-button lan-transfer-back"
                type="button"
                :aria-label="t('backAction')"
                :title="t('backAction')"
                @click="goBack"
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
            <div>
                <h1>{{ t("lanTransferTitle") }}</h1>
                <span class="lan-transfer-status-line">
                    <i
                        class="lan-transfer-status-dot"
                        :class="{
                            connected: isTransferConnected,
                            disconnected: !isTransferConnected,
                        }"
                        aria-hidden="true"
                    ></i>
                    <strong>{{ connectionLabel }}</strong>
                </span>
            </div>
        </header>

        <section class="lan-transfer-connect">
            <div class="lan-transfer-qr" v-html="state.qrSvg"></div>
            <div class="lan-transfer-link-panel">
                <div class="lan-transfer-link-row">
                    <a
                        class="lan-transfer-url"
                        :href="transferUrl || undefined"
                        target="_blank"
                        rel="noreferrer"
                        :title="transferUrl || undefined"
                    >
                        {{ transferUrl || "--" }}
                    </a>
                    <button
                        class="toolbar-icon-button lan-transfer-copy-link"
                        type="button"
                        :disabled="!transferUrl"
                        :title="t('copy')"
                        :aria-label="t('copy')"
                        @click="copyTransferUrl"
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
                <div class="lan-transfer-link-meta">
                    <span>{{ t("lanTransferConnectedDevices") }}</span>
                    <strong>{{ state.connectedDevices ?? 0 }}</strong>
                </div>
            </div>
        </section>

        <section ref="messagesRef" class="lan-transfer-messages">
            <div v-if="!messages.length" class="lan-transfer-empty">
                {{ t("lanTransferEmpty") }}
            </div>

            <article
                v-for="message in messages"
                :key="message.id"
                class="lan-transfer-message"
                :class="`from-${message.sender}`"
            >
                <div
                    class="lan-transfer-avatar"
                    :aria-label="avatarLabel(message.sender)"
                >
                    <img
                        v-if="message.sender === 'desktop'"
                        src="/app-icon.png"
                        alt=""
                    />
                    <svg v-else viewBox="0 0 24 24" aria-hidden="true">
                        <path
                            d="M8 3h8a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2Zm2 15h4"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.8"
                            stroke-linecap="round"
                        />
                    </svg>
                </div>
                <div
                    class="lan-transfer-bubble"
                    :data-app-context-menu="
                        message.hasLocalFile ? 'true' : undefined
                    "
                    @contextmenu="openFileMenu($event, message)"
                >
                    <p
                        v-if="message.text && message.kind !== 'file'"
                        class="lan-transfer-text"
                    >
                        {{ message.text }}
                    </p>
                    <img
                        v-if="message.imageDataUrl"
                        class="lan-transfer-image"
                        :src="message.imageDataUrl"
                        :alt="message.fileName || t('kindImage')"
                    />
                    <div
                        v-if="message.kind === 'file' || message.fileName"
                        class="lan-transfer-file"
                    >
                        <span class="lan-transfer-file-icon" aria-hidden="true">
                            <svg viewBox="0 0 24 24">
                                <path
                                    d="M7 3h6l4 4v14H7V3Zm6 1.5V8h3.5"
                                    fill="none"
                                    stroke="currentColor"
                                    stroke-width="1.8"
                                    stroke-linejoin="round"
                                />
                            </svg>
                        </span>
                        <div>
                            <strong>{{
                                message.fileName || t("lanTransferFile")
                            }}</strong>
                            <span>{{ formatBytes(message.size) }}</span>
                            <small
                                v-if="
                                    message.sender === 'phone' && message.text
                                "
                            >
                                {{ message.text }}
                            </small>
                            <small
                                v-if="
                                    message.status === 'failed' && message.text
                                "
                            >
                                {{ message.text }}
                            </small>
                        </div>
                    </div>
                    <div
                        v-if="
                            message.status === 'uploading' ||
                            message.status === 'failed'
                        "
                        class="lan-transfer-progress"
                    >
                        <span>{{ transferStatusLabel(message) }}</span>
                        <progress
                            v-if="message.status === 'uploading'"
                            :value="transferProgress(message)"
                            max="100"
                        ></progress>
                    </div>
                </div>
            </article>
        </section>

        <form class="lan-transfer-composer" @submit.prevent="sendText">
            <button
                class="toolbar-icon-button lan-transfer-attach"
                type="button"
                :disabled="busy || !state.running"
                :title="t('lanTransferChooseFile')"
                :aria-label="t('lanTransferChooseFile')"
                @click="chooseFile"
            >
                <svg viewBox="0 0 24 24" aria-hidden="true">
                    <path
                        d="M12 5v14M5 12h14"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2.2"
                        stroke-linecap="round"
                    />
                </svg>
            </button>
            <textarea
                v-model="draft"
                rows="1"
                :disabled="busy || !state.running"
                :placeholder="t('lanTransferInputPlaceholder')"
                @keydown.enter.exact.stop.prevent="sendText"
            ></textarea>
            <button
                class="primary lan-transfer-send"
                type="submit"
                :disabled="!canSendText"
            >
                {{ t("lanTransferSend") }}
            </button>
            <input
                ref="fileInputRef"
                type="file"
                multiple
                hidden
                @change="handleFileChange"
            />
        </form>

        <p v-if="error || localError" class="lan-transfer-error">
            {{ error || localError }}
        </p>

        <div
            v-if="contextMenu.show"
            class="lan-transfer-context-menu"
            :style="{ left: `${contextMenu.x}px`, top: `${contextMenu.y}px` }"
            @click.stop
        >
            <button type="button" @click="handleOpenContextFile">
                {{ t("openAction") }}
            </button>
            <button type="button" @click="handleRevealContextFile">
                {{ t("revealInExplorer") }}
            </button>
        </div>
    </section>
</template>
