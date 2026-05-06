<script setup>
import { computed, nextTick, onBeforeUnmount, onMounted, ref } from 'vue'
import { formatRelativeTime } from '../utils/format'
import { looksLikeCode, previewHtml } from '../utils/codePreview'
import { HISTORY_TAG_COLORS, resolveTagLabel } from '../utils/constants'

const props = defineProps({
  canClipboardWrite: { type: Boolean, required: true },
  canDirectPaste: { type: Boolean, required: true },
  item: { type: Object, required: true },
  locale: { type: String, required: true },
  relativeTimeVersion: { type: Number, required: true },
  selected: { type: Boolean, required: true },
  tagLabelMap: { type: Object, required: true },
  t: { type: Function, required: true },
  unsupportedDirectPasteMessage: { type: String, required: true },
  unsupportedClipboardWriteMessage: { type: String, required: true },
})

const emit = defineEmits(['copy', 'edit', 'open-link', 'paste', 'remove', 'select', 'toggle-pin', 'update-tags'])
const entryRef = ref(null)
const tagTriggerRef = ref(null)
const tagPickerRef = ref(null)
const tagPickerStyle = ref({})
const imagePreviewStyle = ref({})
const showImagePreview = ref(false)
const showTagPicker = ref(false)
const imagePreviewUrl = computed(() => (showImagePreview.value ? entryRef.value?.dataset.previewUrl ?? '' : ''))
const tagColors = computed(() => Array.isArray(props.item?.tagColors) ? props.item.tagColors : [])
const tagColorOptions = HISTORY_TAG_COLORS
const canAddMoreTags = computed(() => tagColors.value.length < 3)
const hasTextPreview = computed(() => {
  if (props.item?.kind === 'image') {
    return false
  }
  const text = typeof props.item?.fullText === 'string' ? props.item.fullText : ''
  const preview = typeof props.item?.preview === 'string' ? props.item.preview : ''
  return Boolean(text.trim() || preview.trim())
})
const hasMixedPreview = computed(
  () => props.item?.kind === 'mixed' && Boolean(props.item?.imageDataUrl) && hasTextPreview.value,
)
const isMobileSource = computed(() => props.item?.sourceApp === 'Mobile')
const sourceAppInitials = computed(() => {
  const sourceApp = typeof props.item?.sourceApp === 'string' ? props.item.sourceApp.trim() : ''
  if (!sourceApp) {
    return ''
  }

  const segments = sourceApp
    .split(/[\s._-]+/)
    .map((segment) => segment.trim())
    .filter(Boolean)
  if (segments.length >= 2) {
    return segments
      .slice(0, 2)
      .map((segment) => segment[0]?.toUpperCase() ?? '')
      .join('')
  }

  return sourceApp.slice(0, 2).toUpperCase()
})
const relativeTimeLabel = computed(() => {
  const version = props.relativeTimeVersion
  if (version < 0) {
    return ''
  }
  return formatRelativeTime(props.item.createdAt, props.locale)
})

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

async function updateTagPickerPosition() {
  if (!showTagPicker.value || !tagTriggerRef.value || !tagPickerRef.value) {
    return
  }

  const triggerRect = tagTriggerRef.value.getBoundingClientRect()
  const pickerRect = tagPickerRef.value.getBoundingClientRect()
  const gap = 8
  const viewportPadding = 12
  const pickerWidth = pickerRect.width || 156
  const pickerHeight = pickerRect.height || 320

  const left = Math.min(
    Math.max(viewportPadding, triggerRect.right - pickerWidth),
    Math.max(viewportPadding, window.innerWidth - pickerWidth - viewportPadding),
  )

  const preferredTop = triggerRect.bottom + gap
  const fallbackTop = triggerRect.top - pickerHeight - gap
  const top =
    preferredTop + pickerHeight <= window.innerHeight - viewportPadding
      ? preferredTop
      : Math.max(viewportPadding, fallbackTop)

  tagPickerStyle.value = {
    top: `${top}px`,
    left: `${left}px`,
  }
}

async function openTagPicker() {
  showTagPicker.value = true
  await nextTick()
  await updateTagPickerPosition()
}

function closeTagPicker() {
  showTagPicker.value = false
}

async function toggleTagPicker() {
  if (showTagPicker.value) {
    closeTagPicker()
    return
  }

  await openTagPicker()
}

function isTagSelected(color) {
  return tagColors.value.includes(color)
}

function tagToneClass(color) {
  return `history-tag-${color}`
}

function tagDisplayName(color) {
  return resolveTagLabel(color, props.tagLabelMap, props.t)
}

function removeTagColor(color) {
  emit('update-tags', {
    id: props.item.id,
    tagColors: tagColors.value.filter((item) => item !== color),
  })
}

function toggleTagColor(color) {
  const current = [...tagColors.value]
  if (current.includes(color)) {
    emit('update-tags', {
      id: props.item.id,
      tagColors: current.filter((item) => item !== color),
    })
    return
  }

  if (current.length >= 3) {
    return
  }

  emit('update-tags', {
    id: props.item.id,
    tagColors: [...current, color],
  })
}

function handleDocumentPointerDown(event) {
  if (!showTagPicker.value) {
    return
  }

  const target = event.target
  if (tagPickerRef.value?.contains(target) || tagTriggerRef.value?.contains(target) || entryRef.value?.contains(target)) {
    return
  }

  closeTagPicker()
}

function handleDocumentKeydown(event) {
  if (event.key === 'Escape') {
    closeTagPicker()
  }
}

function handleDocumentScroll() {
  if (!showTagPicker.value) {
    return
  }

  void updateTagPickerPosition()
}

function handleWindowResize() {
  if (!showTagPicker.value) {
    return
  }

  void updateTagPickerPosition()
}

onMounted(() => {
  document.addEventListener('pointerdown', handleDocumentPointerDown)
  document.addEventListener('keydown', handleDocumentKeydown)
  document.addEventListener('scroll', handleDocumentScroll, true)
  window.addEventListener('resize', handleWindowResize)
})

onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', handleDocumentPointerDown)
  document.removeEventListener('keydown', handleDocumentKeydown)
  document.removeEventListener('scroll', handleDocumentScroll, true)
  window.removeEventListener('resize', handleWindowResize)
})
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
          <svg
            v-if="isMobileSource"
            viewBox="0 0 24 24"
            aria-hidden="true"
            class="source-app-icon-phone"
          >
            <path
              d="M8 3h8a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2Zm2 15h4"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              stroke-linecap="round"
            />
          </svg>
          <img
            v-else-if="item.sourceIconDataUrl"
            :src="item.sourceIconDataUrl"
            alt=""
            class="source-app-icon-image"
          />
          <span v-else-if="sourceAppInitials" aria-hidden="true" class="source-app-icon-monogram">
            {{ sourceAppInitials }}
          </span>
          <svg v-else viewBox="0 0 16 16" aria-hidden="true" class="source-app-icon-fallback">
            <path
              d="M2.5 3.2a1 1 0 0 1 1-1h9a1 1 0 0 1 1 1v9.6a1 1 0 0 1-1 1h-9a1 1 0 0 1-1-1V3.2Zm2 1.2v2.4h2.4V4.4H4.5Zm4.6 0v2.4h2.4V4.4H9.1ZM4.5 9.2v2.4h2.4V9.2H4.5Zm4.6 0v2.4h2.4V9.2H9.1Z"
              fill="currentColor"
            />
          </svg>
        </div>
        <span v-if="item.favorite" class="pill accent-alt">{{ t("badgeStarred") }}</span>
        <div v-if="tagColors.length" class="history-tag-list" :aria-label="t('historyTags')">
          <span
            v-for="color in tagColors"
            :key="`${item.id}-${color}`"
            class="history-tag-chip"
            :class="tagToneClass(color)"
          >
            <span class="history-tag-dot" :class="tagToneClass(color)"></span>
            <span class="history-tag-chip-label">{{ tagDisplayName(color) }}</span>
            <button
              class="shortcut-clear-button history-tag-remove-button"
              type="button"
              :title="`${props.t('removeTag')} ${tagDisplayName(color)}`"
              :aria-label="`${props.t('removeTag')} ${tagDisplayName(color)}`"
              @click.stop="removeTagColor(color)"
            >
              <span aria-hidden="true">×</span>
            </button>
          </span>
        </div>
      </div>
      <span class="timestamp">{{ relativeTimeLabel }}</span>
    </div>

    <div
      class="entry-content"
      :class="{
        'entry-content-text-only': !item.imageDataUrl,
        'entry-content-mixed': hasMixedPreview,
      }"
    >
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
      <div class="entry-body" :class="{ 'entry-body-mixed': hasMixedPreview }">
        <div v-if="hasTextPreview" class="entry-text-preview">
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
          ref="tagTriggerRef"
          class="entry-action-button icon-only tag-action"
          type="button"
          :title="t('manageTags')"
          :aria-label="t('manageTags')"
          @mousedown.stop
          @click.stop="toggleTagPicker"
        >
          <svg viewBox="0 0 1024 1024" aria-hidden="true" class="action-icon-balance action-icon-balance-tag">
            <path
              d="M420.8 919.2c-40 0-78.4-16-108.8-46.4l-160-160.8c-29.6-29.6-45.6-68.8-44.8-110.4 0-41.6 16.8-80 45.6-108l373.6-373.6c10.4-10.4 22.4-15.2 33.6-15.2h310.4c26.4 0 48 21.6 48 48V464c0 11.2-4.8 23.2-15.2 33.6l-373.6 373.6c-32.8 32-69.6 48-108.8 48z m151.2-734.4L208 548.8c-13.6 13.6-21.6 32.8-21.6 52 0 19.2 7.2 39.2 21.6 53.6L368 814.4c29.6 29.6 75.2 29.6 104.8 0L838.4 448V184.8h-266.4z"
              fill="currentColor"
            />
            <path
              d="M672.8 470.4c-66.4 0-120-53.6-120-120s53.6-120 120-120 120 53.6 120 120-53.6 120-120 120z m0-176c-30.4 0-56 25.6-56 56s25.6 56 56 56 56-25.6 56-56-25.6-56-56-56z"
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

  <Teleport to="body">
    <div
      v-if="showTagPicker"
      ref="tagPickerRef"
      class="history-tag-picker"
      :style="tagPickerStyle"
      @click.stop
    >
      <div class="history-tag-picker-head">
        <span>{{ t('manageTags') }}</span>
        <span class="history-tag-picker-count">{{ tagColors.length }}/3</span>
      </div>
      <div class="history-tag-picker-grid">
        <button
          v-for="color in tagColorOptions"
          :key="color"
          class="history-tag-picker-option"
          :class="[tagToneClass(color), { active: isTagSelected(color) }]"
          type="button"
          :title="t(`tagColor${color[0].toUpperCase()}${color.slice(1)}`)"
          :aria-label="t(`tagColor${color[0].toUpperCase()}${color.slice(1)}`)"
          :disabled="!isTagSelected(color) && !canAddMoreTags"
          @click.stop="toggleTagColor(color)"
        >
          <span class="history-tag-picker-swatch"></span>
          <span class="history-tag-picker-option-label">{{ tagDisplayName(color) }}</span>
        </button>
      </div>
    </div>
  </Teleport>

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
