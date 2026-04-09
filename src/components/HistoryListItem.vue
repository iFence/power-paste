<script setup>
import { computed, ref } from 'vue'
import { formatRelativeTime } from '../utils/format'
import { looksLikeCode, previewHtml } from '../utils/codePreview'

defineProps({
  canClipboardWrite: { type: Boolean, required: true },
  canDirectPaste: { type: Boolean, required: true },
  item: { type: Object, required: true },
  locale: { type: String, required: true },
  selected: { type: Boolean, required: true },
  t: { type: Function, required: true },
  unsupportedDirectPasteMessage: { type: String, required: true },
  unsupportedClipboardWriteMessage: { type: String, required: true },
})

const emit = defineEmits(['copy', 'edit', 'open-link', 'paste', 'remove', 'select', 'toggle-pin'])
const entryRef = ref(null)
const imagePreviewStyle = ref({})
const showImagePreview = ref(false)
const imagePreviewUrl = computed(() => (showImagePreview.value ? entryRef.value?.dataset.previewUrl ?? '' : ''))

function formatImageSize(bytes) {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return ''
  }

  if (bytes < 1_000_000) {
    return `${Math.max(1, Math.round(bytes / 1024))} KB`
  }

  return `${(bytes / 1_000_000).toFixed(1)} MB`
}

function resolvePreviewFrame(target) {
  const previewWidth = Math.min(420, Math.max(280, Math.floor(window.innerWidth * 0.28)))
  const previewMaxHeight = Math.min(320, Math.max(220, Math.floor(window.innerHeight * 0.36)))
  const imageWidth = Number(target.dataset.imageWidth)
  const imageHeight = Number(target.dataset.imageHeight)

  if (!Number.isFinite(imageWidth) || !Number.isFinite(imageHeight) || imageWidth <= 0 || imageHeight <= 0) {
    return {
      previewWidth,
      previewMaxHeight,
      previewFrameHeight: previewMaxHeight + 20,
    }
  }

  const aspectRatio = imageWidth / imageHeight
  const previewAspectRatio = previewWidth / previewMaxHeight
  const renderedImageHeight =
    aspectRatio >= previewAspectRatio ? Math.max(96, previewWidth / aspectRatio) : previewMaxHeight

  return {
    previewWidth,
    previewMaxHeight,
    previewFrameHeight: renderedImageHeight + 20,
  }
}

function updateImagePreviewPosition(target) {
  if (!entryRef.value || !target) {
    return
  }

  const rect = target.getBoundingClientRect()
  const { previewWidth, previewMaxHeight, previewFrameHeight } = resolvePreviewFrame(target)
  const gap = 16
  const fitsRight = rect.right + gap + previewWidth <= window.innerWidth - 16
  const left = fitsRight
    ? rect.right + gap
    : Math.max(16, rect.left - gap - previewWidth)
  const top = Math.min(
    Math.max(16, rect.top + rect.height / 2 - previewFrameHeight / 2),
    Math.max(16, window.innerHeight - previewFrameHeight - 16),
  )

  imagePreviewStyle.value = {
    top: `${top}px`,
    left: `${left}px`,
    width: `${previewWidth}px`,
    maxHeight: `${previewFrameHeight}px`,
    '--preview-image-max-height': `${previewMaxHeight}px`,
  }
}

function handlePreviewMouseEnter(event) {
  if (!entryRef.value?.dataset.previewUrl) {
    return
  }

  updateImagePreviewPosition(event.currentTarget)
  showImagePreview.value = true
}

function handlePreviewMouseLeave() {
  showImagePreview.value = false
}
</script>

<template>
  <article
    ref="entryRef"
    :data-history-id="item.id"
    :data-preview-url="item.imageDataUrl || ''"
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

    <div class="entry-content" :class="{ 'entry-content-text-only': !item.imageDataUrl }">
      <img
        v-if="item.imageDataUrl"
        :src="item.imageDataUrl"
        alt=""
        class="entry-thumb"
        :data-image-width="item.imageWidth || ''"
        :data-image-height="item.imageHeight || ''"
        @mouseenter="handlePreviewMouseEnter"
        @mouseleave="handlePreviewMouseLeave"
      />
      <div class="entry-body">
        <div v-if="!(item.imageDataUrl && !item.fullText)" class="entry-text-preview">
          <div class="entry-text-scroll">
            <pre
              v-if="item.fullText && looksLikeCode(item.fullText ?? item.preview)"
              class="code-preview"
              v-html="previewHtml(item)"
            ></pre>
            <pre v-else class="text-preview">{{ item.fullText ?? item.preview }}</pre>
          </div>
        </div>
      </div>
    </div>

    <footer class="entry-footer">
      <span v-if="item.imageDataUrl && item.imageByteSize" class="entry-meta-note">
        {{ formatImageSize(item.imageByteSize) }}
      </span>
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
            class="pin-action-icon action-icon-balance action-icon-balance-pin"
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
          <svg viewBox="0 0 1024 1024" aria-hidden="true" class="action-icon-balance action-icon-balance-edit">
            <path
              d="M884.010667 299.989333l-77.994667 77.994667-160-160 77.994667-77.994667q11.989333-11.989333 29.994667-11.989333t29.994667 11.989333l100.010667 100.010667q11.989333 11.989333 11.989333 29.994667t-11.989333 29.994667zM128 736l472.021333-472.021333 160 160-472.021333 472.021333-160 0 0-160z"
              fill="currentColor"
            />
          </svg>
        </button>
        <button
          v-if="item.kind === 'link' && item.fullText"
          class="entry-action-button icon-only open-link-action"
          type="button"
          :title="t('openLink')"
          :aria-label="t('openLink')"
          @mousedown.stop
          @click.stop="emit('open-link', item.fullText)"
        >
          <svg viewBox="0 0 1024 1024" aria-hidden="true" class="action-icon-balance action-icon-balance-link">
            <path
              d="M593.94368 715.648a10.688 10.688 0 0 0-14.976 0L424.21568 870.4c-71.68 71.68-192.576 79.232-271.68 0-79.232-79.232-71.616-200 0-271.616l154.752-154.752a10.688 10.688 0 0 0 0-15.04l-52.992-52.992a10.688 10.688 0 0 0-15.04 0L84.50368 530.688a287.872 287.872 0 0 0 0 407.488 288 288 0 0 0 407.488 0l154.752-154.752a10.688 10.688 0 0 0 0-15.04l-52.736-52.736z m344.384-631.168a288.256 288.256 0 0 1 0 407.616l-154.752 154.752a10.688 10.688 0 0 1-15.04 0l-52.992-52.992a10.688 10.688 0 0 1 0-15.104l154.752-154.688c71.68-71.68 79.232-192.448 0-271.68-79.104-79.232-200-71.68-271.68 0L443.92768 307.2a10.688 10.688 0 0 1-15.04 0l-52.864-52.864a10.688 10.688 0 0 1 0-15.04l154.88-154.752a287.872 287.872 0 0 1 407.424 0z m-296.32 240.896l52.672 52.736a10.688 10.688 0 0 1 0 15.04l-301.504 301.44a10.688 10.688 0 0 1-15.04 0l-52.736-52.672a10.688 10.688 0 0 1 0-15.04l301.632-301.504a10.688 10.688 0 0 1 15.04 0z"
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
          <svg viewBox="0 0 1024 1024" aria-hidden="true" class="delete-action-icon action-icon-balance action-icon-balance-delete">
            <path
              d="M896 352l-73.792 556.608A96 96 0 0 1 727.04 992H296.96a96 96 0 0 1-95.168-83.392L128 352h768zM528 32A80 80 0 0 1 608 112V128h288a64 64 0 1 1 0 128H128a64 64 0 1 1 0-128h320v-16A80 80 0 0 1 528 32z"
              fill="currentColor"
            />
          </svg>
        </button>
      </div>
    </footer>

  </article>

  <Teleport to="body">
    <div
      v-if="imagePreviewUrl"
      class="image-hover-preview"
      :class="{ visible: showImagePreview }"
      :style="imagePreviewStyle"
      aria-hidden="true"
    >
      <img :src="imagePreviewUrl" alt="" class="image-hover-preview-image" />
    </div>
  </Teleport>
</template>
