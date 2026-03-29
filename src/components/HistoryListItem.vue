<script setup>
import { formatRelativeTime } from "../utils/format";
import { looksLikeCode, previewHtml } from "../utils/codePreview";

defineProps({
  canClipboardWrite: { type: Boolean, required: true },
  canDirectPaste: { type: Boolean, required: true },
  item: { type: Object, required: true },
  locale: { type: String, required: true },
  selected: { type: Boolean, required: true },
  t: { type: Function, required: true },
  unsupportedDirectPasteMessage: { type: String, required: true },
  unsupportedClipboardWriteMessage: { type: String, required: true },
});

const emit = defineEmits(["copy", "edit", "paste", "remove", "select", "toggle-pin"]);
</script>

<template>
  <article
    :data-history-id="item.id"
    class="history-entry"
    :class="{ active: selected, 'is-paste-disabled': !canDirectPaste }"
    :title="canDirectPaste ? undefined : unsupportedDirectPasteMessage"
    :aria-label="canDirectPaste ? undefined : unsupportedDirectPasteMessage"
    @click.left="emit('select', item.id)"
    @dblclick.left.prevent="
      emit('select', item.id);
      if (canDirectPaste) emit('paste', item.id);
    "
  >
    <div class="entry-heading">
      <div class="entry-badges">
        <div
          class="source-app-icon"
          :title="item.sourceApp || t('clipboardFallback')"
          :aria-label="item.sourceApp || t('clipboardFallback')"
        >
          <img
            v-if="item.sourceIconDataUrl"
            :src="item.sourceIconDataUrl"
            alt=""
            class="source-app-icon-image"
          />
          <svg v-else viewBox="0 0 16 16" aria-hidden="true" class="source-app-icon-fallback">
            <path
              d="M2.5 3.2a1 1 0 0 1 1-1h9a1 1 0 0 1 1 1v9.6a1 1 0 0 1-1 1h-9a1 1 0 0 1-1-1V3.2Zm2 1.2v2.4h2.4V4.4H4.5Zm4.6 0v2.4h2.4V4.4H9.1ZM4.5 9.2v2.4h2.4V9.2H4.5Zm4.6 0v2.4h2.4V9.2H9.1Z"
              fill="currentColor"
            />
          </svg>
        </div>
        <span v-if="item.favorite" class="pill accent-alt">{{ t("badgeStarred") }}</span>
      </div>
      <span class="timestamp">{{ formatRelativeTime(item.createdAt, locale) }}</span>
    </div>

    <div class="entry-content">
      <img
        v-if="item.imageDataUrl"
        :src="item.imageDataUrl"
        alt=""
        class="entry-thumb"
      />
      <div class="entry-body">
        <template v-if="item.imageDataUrl && !item.fullText">
          <span class="image-meta">{{ item.imageWidth }} x {{ item.imageHeight }}</span>
        </template>
        <div v-else class="entry-text-preview">
          <pre
            v-if="item.fullText && looksLikeCode(item.fullText ?? item.preview)"
            class="code-preview"
            v-html="previewHtml(item)"
          ></pre>
          <pre v-else class="text-preview">{{ item.fullText ?? item.preview }}</pre>
        </div>
        <span v-if="item.imageDataUrl && item.fullText" class="image-meta">{{ item.imageWidth }} x {{ item.imageHeight }}</span>
      </div>
    </div>

    <footer class="entry-footer">
      <div class="entry-actions">
        <button
          class="entry-action-button icon-only"
          type="button"
          :title="canClipboardWrite ? t('copy') : unsupportedClipboardWriteMessage"
          :aria-label="canClipboardWrite ? t('copy') : unsupportedClipboardWriteMessage"
          :disabled="!canClipboardWrite"
          @mousedown.stop
          @click.stop="emit('copy', item.id)"
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path
              d="M5 3.2A1.8 1.8 0 0 1 6.8 1.4h4.1a1.8 1.8 0 0 1 1.8 1.8v4.1a1.8 1.8 0 0 1-1.8 1.8H6.8A1.8 1.8 0 0 1 5 7.3V3.2Zm1.2.1v4a.6.6 0 0 0 .6.6h4a.6.6 0 0 0 .6-.6v-4a.6.6 0 0 0-.6-.6h-4a.6.6 0 0 0-.6.6Z"
              fill="currentColor"
            />
            <path
              d="M3.3 5A1.3 1.3 0 0 0 2 6.3v5.4A1.3 1.3 0 0 0 3.3 13h5.4A1.3 1.3 0 0 0 10 11.7v-.6H8.8v.6a.1.1 0 0 1-.1.1H3.3a.1.1 0 0 1-.1-.1V6.3a.1.1 0 0 1 .1-.1h.6V5h-.6Z"
              fill="currentColor"
            />
          </svg>
        </button>
        <button
          class="entry-action-button icon-only pin-action"
          :class="{ active: item.pinned }"
          type="button"
          :title="item.pinned ? t('unpin') : t('pin')"
          :aria-label="item.pinned ? t('unpin') : t('pin')"
          @mousedown.stop
          @click.stop="emit('toggle-pin', item.id)"
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path
              d="M5.2 2.5h5.6l-.8 3 1.9 1.9v1H8.8v4.8l-.8.8-.8-.8V8.4H4.1v-1L6 5.5l-.8-3Z"
              :fill="item.pinned ? 'currentColor' : 'none'"
              stroke="currentColor"
              stroke-width="1.2"
              stroke-linejoin="round"
            />
          </svg>
        </button>
        <button
          v-if="item.kind === 'text'"
          class="entry-action-button icon-only"
          type="button"
          :title="t('editItem')"
          :aria-label="t('editItem')"
          @mousedown.stop
          @click.stop="emit('edit', item)"
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path
              d="M11.9 2.3 13.7 4a1 1 0 0 1 0 1.4l-6.8 6.8-2.8.9.9-2.8 6.8-6.8a1 1 0 0 1 1.4 0ZM4.8 10.7l.5.5 5.9-5.9-.5-.5-5.9 5.9Z"
              fill="currentColor"
            />
          </svg>
        </button>
        <button
          class="entry-action-button icon-only danger"
          type="button"
          :title="t('deleteItem')"
          :aria-label="t('deleteItem')"
          @mousedown.stop
          @click.stop="emit('remove', item.id)"
        >
          <svg viewBox="0 0 16 16" aria-hidden="true">
            <path
              d="M6.2 2.5h3.6l.5 1.3h2.2v1.1H3.5V3.8h2.2l.5-1.3Zm-1 3.1h5.6l-.5 7.2a1 1 0 0 1-1 .9H6.7a1 1 0 0 1-1-.9l-.5-7.2Zm1.8 1.3v4.8h1.1V6.9H7Zm2 0v4.8h1.1V6.9H9Z"
              fill="currentColor"
            />
          </svg>
        </button>
      </div>
    </footer>
  </article>
</template>
