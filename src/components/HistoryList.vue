<script setup>
import HistoryListItem from "./HistoryListItem.vue";

defineProps({
  canClipboardWrite: { type: Boolean, required: true },
  canDirectPaste: { type: Boolean, required: true },
  copyStatsEnabled: { type: Boolean, required: true },
  pasteStatsEnabled: { type: Boolean, required: true },
  historyPanelRef: { type: Object, required: true },
  hasMore: { type: Boolean, required: true },
  items: { type: Array, required: true },
  loading: { type: Boolean, required: true },
  loadingMore: { type: Boolean, required: true },
  locale: { type: String, required: true },
  relativeTimeVersion: { type: Number, required: true },
  selectedId: { type: String, default: null },
  tagLabelMap: { type: Object, required: true },
  t: { type: Function, required: true },
  unsupportedClipboardWriteMessage: { type: String, required: true },
  unsupportedDirectPasteMessage: { type: String, required: true },
});

const emit = defineEmits([
  "copy",
  "edit",
  "load-more",
  "open-link",
  "paste",
  "remove",
  "select",
  "toggle-pin",
  "update-tags",
]);

function handleScroll(event) {
  const panel = event.currentTarget;
  const distanceToBottom =
    panel.scrollHeight - panel.scrollTop - panel.clientHeight;

  if (distanceToBottom < 180) {
    emit("load-more");
  }
}
</script>

<template>
  <main class="history-shell">
    <section :ref="historyPanelRef" class="history-panel" @scroll.passive="handleScroll">
      <div v-if="loading" class="empty-state">{{ t("loadingHistory") }}</div>

      <div v-else-if="!items.length" class="empty-state">
        {{ hasMore ? t("loadingHistory") : t("historyEmpty") }}
      </div>

      <div v-else class="history-list">
        <HistoryListItem
          v-for="(item, index) in items"
          :key="item.id"
          :item="item"
          :shortcut-label="index < 10 ? (index === 9 ? '0' : String(index + 1)) : ''"
          :locale="locale"
          :relative-time-version="relativeTimeVersion"
          :can-clipboard-write="canClipboardWrite"
          :can-direct-paste="canDirectPaste"
          :copy-stats-enabled="copyStatsEnabled"
          :paste-stats-enabled="pasteStatsEnabled"
          :selected="item.id === selectedId"
          :tag-label-map="tagLabelMap"
          :t="t"
          :unsupported-clipboard-write-message="unsupportedClipboardWriteMessage"
          :unsupported-direct-paste-message="unsupportedDirectPasteMessage"
          @copy="emit('copy', $event)"
          @edit="emit('edit', $event)"
          @open-link="emit('open-link', $event)"
          @paste="emit('paste', $event)"
          @remove="emit('remove', $event)"
          @select="emit('select', $event)"
          @toggle-pin="emit('toggle-pin', $event)"
          @update-tags="emit('update-tags', $event)"
        />

        <div v-if="loadingMore" class="history-load-more">
          {{ t("loadingHistory") }}
        </div>
      </div>
    </section>
  </main>
</template>
