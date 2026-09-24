<script setup>
// 会话输入区：多行文本 + 待发送附件 + 附件选择菜单，回车即发送。
import { computed, onBeforeUnmount, onMounted, ref } from "vue";
import { formatBytes } from "../../utils/format";

const props = defineProps({
  busy: { type: Boolean, default: false },
  draft: { type: Object, required: true },
  running: { type: Boolean, required: true },
  t: { type: Function, required: true },
});

const emit = defineEmits([
  "add-folder",
  "add-files",
  "pick-clipboard",
  "remove-item",
  "send",
  "update:text",
]);

const attachMenuOpen = ref(false);
// 附件菜单依附在“+”按钮上：点击别处或按 Esc 都会收起。
const attachRef = ref(null);
const textareaRef = ref(null);
// 模块级：极端情况下输入区被重建时，把焦点还给用户，输入不被打断。
let composerHadFocus = false;

const disabled = computed(() => props.busy || !props.running);
const items = computed(() =>
  Array.isArray(props.draft?.items) ? props.draft.items : [],
);
const canSend = computed(
  () => !disabled.value && (Boolean(props.draft?.text?.trim()) || items.value.length > 0),
);
const totalBytes = computed(() =>
  items.value.reduce((sum, item) => sum + Number(item.size || 0), 0),
);

function updateText(event) {
  emit("update:text", event.target.value);
}

function handleFocus() {
  composerHadFocus = true;
}

function handleBlur() {
  composerHadFocus = false;
}

onMounted(() => {
  if (composerHadFocus) {
    textareaRef.value?.focus();
  }
});

// 输入法组词中不触发发送，避免中文候选词被回车打断。
function handleKeydown(event) {
  if (event.isComposing) {
    return;
  }
  if (event.key === "Enter" && !event.shiftKey) {
    event.preventDefault();
    submit();
  }
}

function submit() {
  attachMenuOpen.value = false;
  if (!canSend.value) {
    return;
  }
  emit("send");
}

function runAttach(action) {
  attachMenuOpen.value = false;
  emit(action);
}

function handleDocumentPointerDown(event) {
  if (!attachMenuOpen.value) {
    return;
  }
  if (attachRef.value?.contains(event.target)) {
    return;
  }
  attachMenuOpen.value = false;
}

function handleDocumentKeydown(event) {
  if (event.key === "Escape") {
    attachMenuOpen.value = false;
  }
}

onMounted(() => {
  document.addEventListener("pointerdown", handleDocumentPointerDown, true);
  document.addEventListener("keydown", handleDocumentKeydown);
});

onBeforeUnmount(() => {
  document.removeEventListener("pointerdown", handleDocumentPointerDown, true);
  document.removeEventListener("keydown", handleDocumentKeydown);
});
</script>

<template>
  <div class="lan-composer">
    <div v-if="items.length" class="lan-composer-attachments">
      <div class="lan-composer-attachment-head">
        <span>{{ t("lanComposerAttachments") }}</span>
        <small>{{ formatBytes(totalBytes) }}</small>
      </div>
      <div class="lan-composer-chips">
        <article
          v-for="(item, index) in items"
          :key="item.id || `${item.path}-${index}`"
          class="lan-composer-chip"
        >
          <span>{{ item.name }}</span>
          <small>{{ formatBytes(item.size) }}</small>
          <button
            type="button"
            :title="t('removeAction')"
            :aria-label="t('removeAction')"
            @click="emit('remove-item', index)"
          >
            ×
          </button>
        </article>
      </div>
    </div>

    <div class="lan-composer-input">
      <textarea
        ref="textareaRef"
        :value="draft.text"
        rows="1"
        :placeholder="t('lanComposerPlaceholder')"
        @blur="handleBlur"
        @focus="handleFocus"
        @input="updateText"
        @keydown="handleKeydown"
      ></textarea>
      <div class="lan-composer-tools">
        <!-- 附件菜单挂在按钮上，弹出位置紧贴图标。 -->
        <span ref="attachRef" class="lan-composer-attach">
          <button
            class="toolbar-icon-button"
            type="button"
            :title="t('lanComposerAttach')"
            :aria-label="t('lanComposerAttach')"
            :disabled="disabled"
            @click="attachMenuOpen = !attachMenuOpen"
          >
            <svg viewBox="0 0 1024 1024" aria-hidden="true">
              <path
                d="M512 32c265.088 0 480 214.912 480 480 0 265.088-214.912 480-480 480-265.088 0-480-214.912-480-480C32 246.912 246.912 32 512 32z m0 64C282.24 96 96 282.24 96 512s186.24 416 416 416 416-186.24 416-416S741.76 96 512 96z m0 128a32 32 0 0 1 31.776 28.256L544 256v224h224a32 32 0 0 1 3.744 63.776L768 544h-224v224a32 32 0 0 1-63.776 3.744L480 768v-224H256a32 32 0 0 1-3.744-63.776L256 480h224V256a32 32 0 0 1 32-32z"
                fill="currentColor"
              />
            </svg>
          </button>

          <div v-if="attachMenuOpen" class="lan-composer-menu">
            <button type="button" @click="runAttach('add-files')">
              {{ t("lanTransferPickFile") }}
            </button>
            <button type="button" @click="runAttach('add-folder')">
              {{ t("lanTransferPickFolder") }}
            </button>
            <button type="button" @click="runAttach('pick-clipboard')">
              {{ t("lanTransferPickClipboard") }}
            </button>
          </div>
        </span>
        <button
          class="primary compact"
          type="button"
          :disabled="!canSend"
          @click="submit"
        >
          {{ t("lanTransferSend") }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.lan-composer {
  display: grid;
  gap: 6px;
  padding: 7px 12px 9px;
  background: color-mix(in srgb, var(--app-panel-bg) 88%, transparent);
}

.lan-composer-attachments {
  display: grid;
  gap: 6px;
}

.lan-composer-attachment-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  color: var(--app-muted);
  font-size: 0.68rem;
}

.lan-composer-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.lan-composer-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  max-width: 220px;
  padding: 4px 6px 4px 9px;
  border: 1px solid var(--app-panel-border);
  border-radius: 999px;
  background: color-mix(in srgb, var(--app-panel-strong-bg) 74%, transparent);
  font-size: 0.72rem;
}

.lan-composer-chip span {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lan-composer-chip small {
  flex: 0 0 auto;
  color: var(--app-muted);
  font-size: 0.66rem;
}

.lan-composer-chip button {
  width: 18px;
  height: 18px;
  flex: 0 0 auto;
  padding: 0;
  border: 0;
  border-radius: 999px;
  background: transparent;
  color: var(--app-muted);
  font: inherit;
  font-size: 0.9rem;
  line-height: 1;
  cursor: pointer;
}

.lan-composer-chip button:hover {
  background: rgba(255, 255, 255, 0.08);
  color: var(--app-text);
}

.lan-composer-input {
  position: relative;
  display: flex;
  align-items: flex-end;
  gap: 6px;
  padding: 4px 6px 4px 9px;
  border: 1px solid var(--app-panel-border);
  border-radius: 11px;
  background: var(--app-input-bg);
}

.lan-composer-input:focus-within {
  border-color: color-mix(in srgb, var(--accent-primary) 46%, var(--app-panel-border));
}

.lan-composer-input textarea {
  flex: 1;
  min-width: 0;
  /* 输入区保持原有的舒展高度，不受设置页 textarea 全局最小高度影响；
     内容更多时在框内滚动。 */
  height: 96px;
  min-height: 96px;
  max-height: 96px;
  padding: 3px 0;
  border: 0;
  background: transparent;
  color: var(--app-text);
  font: inherit;
  font-size: 0.8rem;
  line-height: 1.35;
  resize: none;
  outline: none;
}

.lan-composer-tools {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 6px;
}

.lan-composer-attach {
  position: relative;
  display: inline-flex;
}

.lan-composer-menu {
  position: absolute;
  right: 0;
  bottom: calc(100% + 4px);
  z-index: 12;
  display: grid;
  min-width: 132px;
  padding: 5px;
  border: 1px solid var(--app-panel-border);
  border-radius: 10px;
  background: var(--app-select-menu-bg);
  color: var(--app-select-menu-text);
  box-shadow: 0 16px 38px rgba(0, 0, 0, 0.28);
}

.lan-composer-menu button {
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

.lan-composer-menu button:hover {
  background: var(--accent-primary-hover-soft);
}
</style>
