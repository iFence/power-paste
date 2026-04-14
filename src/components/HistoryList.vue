<script setup>
import HistoryListItem from "./HistoryListItem.vue";

defineProps({
  canClipboardWrite: { type: Boolean, required: true },
  canDirectPaste: { type: Boolean, required: true },
  historyPanelRef: { type: Object, required: true },
  items: { type: Array, required: true },
  loading: { type: Boolean, required: true },
  locale: { type: String, required: true },
  relativeTimeVersion: { type: Number, required: true },
  selectedId: { type: String, default: null },
  t: { type: Function, required: true },
  unsupportedClipboardWriteMessage: { type: String, required: true },
  unsupportedDirectPasteMessage: { type: String, required: true },
});

const emit = defineEmits(["copy", "edit", "open-link", "paste", "remove", "select", "toggle-pin"]);
</script>

<template>
  <main class="history-shell">
    <section :ref="historyPanelRef" class="history-panel">
      <div v-if="loading" class="empty-state">{{ t("loadingHistory") }}</div>

      <div v-else-if="!items.length" class="empty-state">
        {{ t("historyEmpty") }}
      </div>

      <div v-else class="history-list">
        <HistoryListItem
          v-for="item in items"
          :key="item.id"
          :item="item"
          :locale="locale"
          :relative-time-version="relativeTimeVersion"
          :can-clipboard-write="canClipboardWrite"
          :can-direct-paste="canDirectPaste"
          :selected="item.id === selectedId"
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
        />
      </div>
    </section>
  </main>
</template>
