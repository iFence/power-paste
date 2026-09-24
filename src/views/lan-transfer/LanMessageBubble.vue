<script setup>
// 会话消息气泡：文本正文、图片预览、文件清单、传输进度与失败重发都在这里渲染。
import { computed, onMounted } from "vue";
import { formatBytes, formatMessageTime } from "../../utils/format";
import { lanErrorCode, lanErrorText } from "../../utils/lanError";

// 气泡外侧进度环的半径与周长（viewBox 36×36，留出描边宽度）。
const RING_RADIUS = 15.5;
const RING_CIRCUMFERENCE = 2 * Math.PI * RING_RADIUS;

const props = defineProps({
  busy: { type: Boolean, default: false },
  locale: { type: String, default: "zh-CN" },
  message: { type: Object, required: true },
  onLoadPreview: { type: Function, default: null },
  // 图片预览 data URL：由会话视图缓存后传入，避免重复取回缩略图。
  preview: { type: String, default: "" },
  t: { type: Function, required: true },
});

const emit = defineEmits(["cancel", "open-file", "resend", "reveal-file"]);

const outgoing = computed(() => props.message.direction === "send");
const files = computed(() =>
  Array.isArray(props.message.files) ? props.message.files : [],
);
const showPreview = computed(() => Boolean(props.preview));
// 纯文本消息（含协议上的 txt 文本包）只显示正文，不显示承载用的文件名。
const textOnly = computed(
  () => Boolean(props.message.text) && props.message.kind === "text",
);
// 已经用图片预览展示的文件不再重复列出文件名，其余文件照常显示。
const listedFiles = computed(() =>
  files.value.filter(
    (file) => !(showPreview.value && String(file.mimeType || "").startsWith("image/")),
  ),
);
const active = computed(() => props.message.status === "active");
const progress = computed(() => {
  if (props.message.status === "done") {
    return 100;
  }
  const total = Number(props.message.totalBytes || 0);
  if (!total) {
    return 0;
  }
  return Math.max(
    0,
    Math.min(100, Math.round((Number(props.message.doneBytes || 0) / total) * 100)),
  );
});
const ringOffset = computed(
  () => RING_CIRCUMFERENCE * (1 - progress.value / 100),
);
const timeLabel = computed(() =>
  formatMessageTime(props.message.createdAtMs, props.locale),
);
// 失败文案优先展示后端返回的稳定错误码，未知错误保留原始细节。
const failureLabel = computed(() => {
  if (props.message.status !== "failed") {
    return "";
  }
  const detail = props.message.error || "";
  if (!detail) {
    return props.t("lanTransferStatusFailed");
  }
  return lanErrorText(props.t, lanErrorCode(detail), detail);
});
const cancelledLabel = computed(() =>
  props.message.status === "cancelled" ? props.t("lanTransferStatusCancelled") : "",
);
const canResend = computed(
  () =>
    outgoing.value &&
    Boolean(props.message.resendable) &&
    (props.message.status === "failed" || props.message.status === "cancelled"),
);
const receivedFile = computed(() => props.message.receivedFile || null);

onMounted(async () => {
  if (props.preview || !props.message.hasPreview || !props.onLoadPreview) {
    return;
  }
  try {
    await props.onLoadPreview(props.message.id);
  } catch (error) {
    // 预览属于锦上添花：取不到时回退到文件清单，不影响消息本身。
    console.error("Failed to load the transfer preview", error);
  }
});
</script>

<template>
  <article class="lan-bubble-row" :class="outgoing ? 'outgoing' : 'incoming'">
    <small class="lan-bubble-time">{{ timeLabel }}</small>
    <div class="lan-bubble-line">
      <div
        class="lan-bubble"
        :class="[message.status, { compact: !message.text, 'has-image': showPreview }]"
      >
        <p v-if="message.text" class="lan-bubble-text">{{ message.text }}</p>

        <img
          v-if="showPreview"
          class="lan-bubble-image"
          :src="preview"
          :alt="files[0]?.fileName || ''"
        />

        <div v-if="listedFiles.length && !textOnly" class="lan-bubble-files">
          <div
            v-for="(file, index) in listedFiles"
            :key="`${file.fileName}-${index}`"
            class="lan-bubble-file"
          >
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path
                d="M6 3.5h7l5 5V20.5H6zM13 3.5v5h5"
                fill="none"
                stroke="currentColor"
                stroke-width="1.6"
                stroke-linejoin="round"
              />
            </svg>
            <span class="lan-bubble-file-name">{{ file.fileName }}</span>
            <small>{{ formatBytes(file.size) }}</small>
          </div>
        </div>

        <p v-if="failureLabel" class="lan-bubble-failure">{{ failureLabel }}</p>
        <p v-else-if="cancelledLabel" class="lan-bubble-status">{{ cancelledLabel }}</p>

        <div v-if="active || canResend || receivedFile" class="lan-bubble-actions">
          <button
            v-if="active && outgoing"
            class="ghost compact"
            type="button"
            :disabled="busy"
            @click="emit('cancel', message.id)"
          >
            {{ t("lanTransferCancel") }}
          </button>
          <button
            v-if="canResend"
            class="ghost compact"
            type="button"
            :disabled="busy"
            @click="emit('resend', message.id)"
          >
            {{ t("lanResend") }}
          </button>
          <template v-if="receivedFile">
            <button
              class="ghost compact"
              type="button"
              @click="emit('open-file', receivedFile.id)"
            >
              {{ t("openAction") }}
            </button>
            <button
              class="ghost compact"
              type="button"
              @click="emit('reveal-file', receivedFile.id)"
            >
              {{ t("revealInExplorer") }}
            </button>
          </template>
        </div>
      </div>

      <!-- 传输进度放在气泡外侧，用一圈进度环 + 环内百分比表示。 -->
      <span
        v-if="active"
        class="lan-bubble-ring"
        :title="`${progress}%`"
        :aria-label="`${progress}%`"
      >
        <svg viewBox="0 0 36 36" aria-hidden="true">
          <circle class="lan-bubble-ring-track" cx="18" cy="18" :r="RING_RADIUS" />
          <circle
            class="lan-bubble-ring-value"
            cx="18"
            cy="18"
            :r="RING_RADIUS"
            :stroke-dasharray="RING_CIRCUMFERENCE"
            :stroke-dashoffset="ringOffset"
          />
        </svg>
        <span class="lan-bubble-ring-label">{{ progress }}</span>
      </span>
    </div>
  </article>
</template>

<style scoped>
.lan-bubble-row {
  display: grid;
  gap: 3px;
  padding: 3px 0;
}

.lan-bubble-row.outgoing {
  justify-items: end;
}

.lan-bubble-row.incoming {
  justify-items: start;
}

.lan-bubble-time {
  color: var(--app-muted);
  font-size: 0.66rem;
}

/* 气泡与外侧进度环同一行：自己发的在左、收到的在右，气泡的贴边方向保持不动。 */
.lan-bubble-line {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  max-width: 100%;
  min-width: 0;
}

.lan-bubble-row.outgoing .lan-bubble-line {
  flex-direction: row-reverse;
}

.lan-bubble-ring {
  position: relative;
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  flex: 0 0 auto;
  color: var(--accent-primary);
}

.lan-bubble-ring svg {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  /* 让进度从 12 点方向顺时针增长。 */
  transform: rotate(-90deg);
}

.lan-bubble-ring-track {
  fill: none;
  stroke: var(--app-panel-border);
  stroke-width: 3;
}

.lan-bubble-ring-value {
  fill: none;
  stroke: currentColor;
  stroke-width: 3;
  stroke-linecap: round;
  transition: stroke-dashoffset 200ms linear;
}

.lan-bubble-ring-label {
  color: var(--app-text);
  font-size: 0.52rem;
  font-weight: 600;
  line-height: 1;
}

.lan-bubble {
  display: grid;
  gap: 6px;
  min-width: 0;
  max-width: min(78%, 420px);
  padding: 8px 11px;
  border: 1px solid var(--app-panel-border);
  border-radius: 13px;
  background: color-mix(in srgb, var(--app-panel-strong-bg) 76%, transparent);
  color: var(--app-text);
}

.lan-bubble-row.outgoing .lan-bubble {
  /* 自己的气泡用主题色实底、不画描边；文字用与主题色配套的深色，保证对比度。 */
  border-color: transparent;
  background: var(--accent-primary-strong);
  color: var(--accent-primary-text);
}

/* 实底上的次级内容改成半透明的主题文字色，避免灰字糊在彩色背景上。 */
.lan-bubble-row.outgoing .lan-bubble-file {
  /* 自己的气泡里文件行不再画任何底色，直接落在主题色上。 */
  padding: 2px 0;
  background: transparent;
}

.lan-bubble-row.outgoing .lan-bubble-file svg,
.lan-bubble-row.outgoing .lan-bubble-file small,
.lan-bubble-row.outgoing .lan-bubble-status {
  color: color-mix(in srgb, var(--accent-primary-text) 70%, transparent);
}

.lan-bubble-row.outgoing .lan-bubble-failure {
  color: color-mix(in srgb, #b3261e 55%, var(--accent-primary-text));
}

.lan-bubble-row.outgoing .lan-bubble-actions button {
  border-color: color-mix(in srgb, var(--accent-primary-text) 32%, transparent);
  background: transparent;
  color: var(--accent-primary-text);
}

.lan-bubble-row.outgoing .lan-bubble-actions button:hover:not(:disabled) {
  background: color-mix(in srgb, var(--accent-primary-text) 12%, transparent);
}

.lan-bubble.failed {
  border-color: color-mix(in srgb, #f06d6d 46%, var(--app-panel-border));
}

/* 有图片预览时气泡不再留内边距，让图片铺满整个气泡；
   同一气泡里的文字、文件清单与按钮再各自补回内边距。 */
.lan-bubble.has-image {
  padding: 0;
  overflow: hidden;
}

.lan-bubble.has-image > :not(.lan-bubble-image) {
  margin-inline: 11px;
}

.lan-bubble.has-image > :first-child:not(.lan-bubble-image) {
  margin-top: 8px;
}

.lan-bubble.has-image > :last-child:not(.lan-bubble-image) {
  margin-bottom: 8px;
}

.lan-bubble-text {
  margin: 0;
  font-size: 0.82rem;
  line-height: 1.45;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}

.lan-bubble-files {
  display: grid;
  gap: 5px;
}

.lan-bubble-image {
  display: block;
  width: 100%;
  border-radius: 0;
  object-fit: cover;
}

.lan-bubble-file {
  display: flex;
  align-items: center;
  gap: 7px;
  min-width: 0;
  padding: 5px 7px;
  border-radius: 9px;
  background: color-mix(in srgb, var(--app-panel-bg) 70%, transparent);
}

.lan-bubble-file svg {
  width: 16px;
  height: 16px;
  flex: 0 0 auto;
  color: var(--app-muted);
}

.lan-bubble-file-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  font-size: 0.76rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lan-bubble-file small {
  flex: 0 0 auto;
  color: var(--app-muted);
  font-size: 0.68rem;
}

.lan-bubble-failure {
  margin: 0;
  color: #f06d6d;
  font-size: 0.72rem;
  overflow-wrap: anywhere;
}

.lan-bubble-status {
  color: var(--app-muted);
  font-size: 0.68rem;
}

.lan-bubble-status {
  margin: 0;
}

.lan-bubble-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

</style>
