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
          class="entry-action-button icon-only pin-action"
          :class="{ active: item.pinned }"
          type="button"
          :title="item.pinned ? t('unpin') : t('pin')"
          :aria-label="item.pinned ? t('unpin') : t('pin')"
          @mousedown.stop
          @click.stop="emit('toggle-pin', item.id)"
        >
          <svg
            viewBox="0 0 16 16"
            aria-hidden="true"
            class="pin-action-icon"
            :class="{ active: item.pinned }"
          >
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
          class="entry-action-button icon-only edit-action"
          type="button"
          :title="t('editItem')"
          :aria-label="t('editItem')"
          @mousedown.stop
          @click.stop="emit('edit', item)"
        >
          <svg viewBox="0 0 1024 1024" aria-hidden="true">
            <path
              d="M884.010667 299.989333l-77.994667 77.994667-160-160 77.994667-77.994667q11.989333-11.989333 29.994667-11.989333t29.994667 11.989333l100.010667 100.010667q11.989333 11.989333 11.989333 29.994667t-11.989333 29.994667zM128 736l472.021333-472.021333 160 160-472.021333 472.021333-160 0 0-160z"
              fill="currentColor"
            />
          </svg>
        </button>
        <button
          class="entry-action-button icon-only danger delete-action"
          type="button"
          :title="t('deleteItem')"
          :aria-label="t('deleteItem')"
          @mousedown.stop
          @click.stop="emit('remove', item.id)"
        >
          <svg viewBox="0 0 1024 1024" aria-hidden="true" class="delete-action-icon">
            <path
              d="M896 352l-73.792 556.608A96 96 0 0 1 727.04 992H296.96a96 96 0 0 1-95.168-83.392L128 352h768zM528 32A80 80 0 0 1 608 112V128h288a64 64 0 1 1 0 128H128a64 64 0 1 1 0-128h320v-16A80 80 0 0 1 528 32z"
              fill="currentColor"
            />
          </svg>
        </button>
      </div>
    </footer>
  </article>
</template>
