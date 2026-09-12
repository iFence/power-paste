<script setup>
// 传输历史面板：集中展示接收文件与收发传输记录。
import { computed, ref } from "vue";

const props = defineProps({
  busy: { type: Boolean, default: false },
  receivedFiles: { type: Array, default: () => [] },
  t: { type: Function, required: true },
  transfers: { type: Array, default: () => [] },
  onCancelTransfer: { type: Function, required: true },
  onOpenFile: { type: Function, required: true },
  onRevealFile: { type: Function, required: true },
});

const emit = defineEmits(["close"]);
const activeSection = ref("received");

const orderedTransfers = computed(() => props.transfers.slice().reverse());

function formatBytes(size) {
  const value = Number(size || 0);
  if (value < 1000) {
    return `${value} B`;
  }
  if (value < 1_000_000) {
    return `${Math.round(value / 1000)} KB`;
  }
  if (value < 1_000_000_000) {
    return `${(value / 1_000_000).toFixed(1)} MB`;
  }
  return `${(value / 1_000_000_000).toFixed(2)} GB`;
}

function progressOf(transfer) {
  const total = Number(transfer.totalBytes || 0);
  if (!total) {
    return 0;
  }
  return Math.max(
    0,
    Math.min(100, Math.round((Number(transfer.doneBytes || 0) / total) * 100)),
  );
}

function statusLabel(transfer) {
  const map = {
    active: props.t("lanTransferStatusActive"),
    done: props.t("lanTransferStatusDone"),
    failed: props.t("lanTransferStatusFailed"),
    cancelled: props.t("lanTransferStatusCancelled"),
  };
  return map[transfer.status] || transfer.status;
}
</script>

<template>
  <div class="lan-history-backdrop" @click.self="emit('close')">
    <aside class="lan-history-panel" :aria-label="t('lanTransferHistory')">
      <header class="lan-history-head">
        <strong>{{ t("lanTransferHistory") }}</strong>
        <button
          class="toolbar-icon-button"
          type="button"
          :title="t('closeAction')"
          :aria-label="t('closeAction')"
          @click="emit('close')"
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path
              d="M7 7l10 10M17 7L7 17"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
              stroke-linecap="round"
            />
          </svg>
        </button>
      </header>

      <div class="lan-history-tabs" role="tablist">
        <button
          type="button"
          :class="{ active: activeSection === 'received' }"
          @click="activeSection = 'received'"
        >
          {{ t("lanTransferReceived") }}
          <span>{{ receivedFiles.length }}</span>
        </button>
        <button
          type="button"
          :class="{ active: activeSection === 'transfers' }"
          @click="activeSection = 'transfers'"
        >
          {{ t("lanTransferTransfers") }}
          <span>{{ transfers.length }}</span>
        </button>
      </div>

      <div v-if="activeSection === 'received'" class="lan-history-list">
        <p v-if="!receivedFiles.length" class="lan-history-empty">
          {{ t("lanTransferReceivedEmpty") }}
        </p>
        <article v-for="file in receivedFiles.slice().reverse()" :key="file.id" class="lan-history-item">
          <div class="lan-history-info">
            <strong>{{ file.fileName }}</strong>
            <small>{{ file.fromAlias }} · {{ formatBytes(file.size) }}</small>
          </div>
          <div class="lan-history-actions">
            <button class="ghost compact" type="button" @click="onOpenFile(file.id)">
              {{ t("openAction") }}
            </button>
            <button class="ghost compact" type="button" @click="onRevealFile(file.id)">
              {{ t("revealInExplorer") }}
            </button>
          </div>
        </article>
      </div>

      <div v-else class="lan-history-list">
        <p v-if="!transfers.length" class="lan-history-empty">
          {{ t("lanTransferTransfersEmpty") }}
        </p>
        <article
          v-for="transfer in orderedTransfers"
          :key="transfer.id"
          class="lan-history-item"
        >
          <div class="lan-history-info">
            <strong>{{ transfer.label }}</strong>
            <small>
              {{ transfer.peerAlias }} · {{ statusLabel(transfer) }}
              <template v-if="transfer.status === 'active'">
                ({{ progressOf(transfer) }}%)
              </template>
            </small>
            <progress
              v-if="transfer.status === 'active'"
              :value="progressOf(transfer)"
              max="100"
            ></progress>
          </div>
          <button
            v-if="transfer.status === 'active'"
            class="ghost compact"
            type="button"
            :disabled="busy"
            @click="onCancelTransfer(transfer.id)"
          >
            {{ t("lanTransferCancel") }}
          </button>
        </article>
      </div>
    </aside>
  </div>
</template>

<style scoped>
.lan-history-backdrop {
  position: absolute;
  inset: 0;
  z-index: 45;
  display: flex;
  justify-content: flex-end;
  background: rgba(0, 0, 0, 0.24);
}

.lan-history-panel {
  display: grid;
  grid-template-rows: auto auto minmax(0, 1fr);
  width: min(92vw, 390px);
  height: 100%;
  padding: 14px;
  border-left: 1px solid var(--app-panel-border);
  background: var(--app-select-menu-bg);
  color: var(--app-select-menu-text);
  box-shadow: -18px 0 50px rgba(0, 0, 0, 0.28);
}

.lan-history-head,
.lan-history-tabs,
.lan-history-item,
.lan-history-actions {
  display: flex;
  align-items: center;
}

.lan-history-head {
  justify-content: space-between;
  gap: 8px;
  padding-bottom: 12px;
}

.lan-history-tabs {
  gap: 4px;
  margin-bottom: 10px;
  padding: 3px;
  border-radius: 10px;
  background: var(--app-input-bg);
}

.lan-history-tabs button {
  display: inline-flex;
  flex: 1;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 7px 8px;
  border: 0;
  border-radius: 8px;
  background: transparent;
  color: var(--app-muted);
  font: inherit;
  font-size: 0.76rem;
  cursor: pointer;
}

.lan-history-tabs button.active {
  background: var(--accent-primary-strong);
  color: var(--accent-primary-text);
}

.lan-history-tabs span {
  font-size: 0.68rem;
  opacity: 0.78;
}

.lan-history-list {
  display: grid;
  align-content: start;
  gap: 8px;
  min-height: 0;
  overflow: auto;
  padding-right: 2px;
}

.lan-history-item {
  justify-content: space-between;
  gap: 10px;
  padding: 10px;
  border: 1px solid var(--app-panel-border);
  border-radius: 11px;
  background: color-mix(in srgb, var(--app-panel-strong-bg) 72%, transparent);
}

.lan-history-info {
  display: grid;
  flex: 1;
  gap: 3px;
  min-width: 0;
}

.lan-history-info strong,
.lan-history-info small {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lan-history-info strong {
  font-size: 0.82rem;
}

.lan-history-info small {
  color: var(--app-muted);
  font-size: 0.7rem;
}

.lan-history-info progress {
  width: 100%;
  height: 5px;
  margin-top: 4px;
  overflow: hidden;
  border: 0;
  border-radius: 999px;
}

.lan-history-actions {
  flex: 0 0 auto;
  gap: 5px;
}

.lan-history-empty {
  margin: 0;
  padding: 36px 16px;
  border: 1px dashed var(--app-panel-border);
  border-radius: 12px;
  color: var(--app-muted);
  font-size: 0.78rem;
  text-align: center;
}
</style>
