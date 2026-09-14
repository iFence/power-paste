<script setup>
// 会话视图：按天分组的消息流与底部输入区插槽。
import { computed, nextTick, ref, watch } from "vue";
import { formatMessageDayLabel } from "../../utils/format";
import LanMessageBubble from "./LanMessageBubble.vue";

const props = defineProps({
  busy: { type: Boolean, default: false },
  locale: { type: String, default: "zh-CN" },
  messages: { type: Array, default: () => [] },
  onLoadPreview: { type: Function, default: null },
  peer: { type: Object, required: true },
  t: { type: Function, required: true },
});

const emit = defineEmits(["back", "cancel", "open-file", "resend", "reveal-file"]);

const scrollRef = ref(null);

// 消息流按天插入分隔标题，便于回溯更早的传输。
const messageGroups = computed(() => {
  const groups = [];
  for (const message of props.messages) {
    const label = formatMessageDayLabel(message.createdAtMs, props.locale);
    const current = groups[groups.length - 1];
    if (current && current.label === label) {
      current.messages.push(message);
    } else {
      groups.push({ label, messages: [message] });
    }
  }
  return groups;
});

async function scrollToBottom() {
  await nextTick();
  const element = scrollRef.value;
  if (element) {
    element.scrollTop = element.scrollHeight;
  }
}

watch(
  () => [props.peer.fingerprint, props.messages.length],
  () => {
    void scrollToBottom();
  },
  { immediate: true },
);
</script>

<template>
  <section class="lan-conversation-panel">
    <div ref="scrollRef" class="lan-conversation-stream">
      <!-- 窄窗单栏布局下没有左侧会话列表，用悬浮返回按钮回到列表。 -->
      <button
        class="toolbar-icon-button lan-conversation-back"
        type="button"
        :aria-label="t('backAction')"
        :title="t('backAction')"
        @click="emit('back')"
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
      <template v-for="group in messageGroups" :key="group.label">
        <p class="lan-conversation-day">{{ group.label }}</p>
        <LanMessageBubble
          v-for="message in group.messages"
          :key="message.id"
          :busy="busy"
          :locale="locale"
          :message="message"
          :on-load-preview="onLoadPreview"
          :preview="message.preview"
          :t="t"
          @cancel="emit('cancel', $event)"
          @open-file="emit('open-file', $event)"
          @resend="emit('resend', $event)"
          @reveal-file="emit('reveal-file', $event)"
        />
      </template>
    </div>

    <slot name="composer"></slot>
  </section>
</template>

<style scoped>
.lan-conversation-panel {
  position: relative;
  display: grid;
  grid-template-rows: minmax(0, 1fr) auto;
  height: 100%;
  min-height: 0;
}

/* 仅在窄窗单栏布局出现：回到左侧会话列表。 */
.lan-conversation-back {
  position: absolute;
  top: 8px;
  left: 8px;
  z-index: 6;
  display: none;
}

.lan-conversation-stream {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow: auto;
  padding: 14px 16px;
}

.lan-conversation-day {
  align-self: center;
  margin: 10px 0 8px;
  padding: 2px 10px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--app-panel-strong-bg) 60%, transparent);
  color: var(--app-muted);
  font-size: 0.66rem;
}

@media (max-width: 699px) {
  .lan-conversation-back {
    display: inline-flex;
  }

  /* 给悬浮返回按钮留出空间，避免压住第一条气泡。 */
  .lan-conversation-stream {
    padding-top: 40px;
  }
}
</style>
