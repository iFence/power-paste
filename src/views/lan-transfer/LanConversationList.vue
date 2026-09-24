<script setup>
// 会话列表：只列出发现的设备；刷新与扫码等页面级动作放在顶栏。
import LanDeviceIcon from "./LanDeviceIcon.vue";

const props = defineProps({
  activeFingerprint: { type: String, default: "" },
  conversations: { type: Array, default: () => [] },
  dropTargetFingerprint: { type: String, default: "" },
  running: { type: Boolean, required: true },
  t: { type: Function, required: true },
});

const emit = defineEmits(["add-device", "select"]);
</script>

<template>
  <aside class="lan-conversation-list">
    <div class="lan-conversation-scroll">
      <p v-if="!conversations.length" class="lan-conversation-empty">
        {{ t("lanTransferNoDevices") }}
        <!-- 组播不可用时没有可发现的设备，这里给出按 IP 手动添加的兜底入口。 -->
        <button
          class="lan-conversation-empty-action"
          type="button"
          :disabled="!running"
          @click="emit('add-device')"
        >
          {{ t("lanTransferManualSend") }}
        </button>
      </p>
      <button
        v-for="conversation in conversations"
        :key="conversation.peer.fingerprint"
        class="lan-conversation-item"
        :class="{
          active: conversation.peer.fingerprint === activeFingerprint,
          dropping: conversation.peer.fingerprint === dropTargetFingerprint,
        }"
        type="button"
        :data-peer-fingerprint="conversation.peer.fingerprint"
        @click="emit('select', conversation.peer.fingerprint)"
      >
        <LanDeviceIcon :device="conversation.peer" :offline="!conversation.peer.online" />
        <span class="lan-conversation-info">
          <strong class="lan-conversation-name">{{ conversation.peer.alias }}</strong>
        </span>
        <span v-if="!conversation.peer.online" class="lan-conversation-offline">
          {{ t("lanPeerOffline") }}
        </span>
        <span v-else-if="conversation.activeCount" class="lan-conversation-badge">
          {{ conversation.activeCount }}
        </span>
      </button>
    </div>

  </aside>
</template>

<style scoped>
.lan-conversation-list {
  display: grid;
  grid-template-rows: minmax(0, 1fr);
  width: 168px;
  flex: 0 0 auto;
  border-right: 1px solid var(--app-panel-border);
  background: color-mix(in srgb, var(--app-panel-bg) 72%, transparent);
}

.lan-conversation-scroll {
  display: grid;
  align-content: start;
  gap: 2px;
  min-height: 0;
  overflow: auto;
  padding: 5px;
}

.lan-conversation-empty {
  display: grid;
  justify-items: center;
  gap: 8px;
  margin: 0;
  padding: 18px 10px;
  color: var(--app-muted);
  font-size: 0.72rem;
  line-height: 1.5;
  text-align: center;
}

.lan-conversation-empty-action {
  padding: 3px 9px;
  border: 1px solid var(--app-panel-border);
  border-radius: 999px;
  background: transparent;
  color: var(--accent-primary);
  font: inherit;
  font-size: 0.7rem;
  cursor: pointer;
}

.lan-conversation-empty-action:hover:not(:disabled) {
  border-color: color-mix(in srgb, var(--accent-primary) 46%, var(--app-panel-border));
  background: var(--accent-primary-soft);
}

.lan-conversation-empty-action:disabled {
  cursor: default;
  opacity: 0.5;
}

.lan-conversation-item {
  display: flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  padding: 5px 7px;
  border: 1px solid transparent;
  border-radius: 10px;
  background: transparent;
  color: var(--app-text);
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition:
    background-color 150ms ease,
    border-color 150ms ease;
}

.lan-conversation-item:hover {
  background: color-mix(in srgb, var(--app-panel-strong-bg) 74%, transparent);
}

.lan-conversation-item.active {
  /* 选中态只靠底色区分，不画描边。 */
  border-color: transparent;
  background: color-mix(in srgb, var(--accent-primary-soft) 46%, var(--app-panel-bg));
}

.lan-conversation-item.dropping {
  border-color: var(--accent-primary);
  background: color-mix(in srgb, var(--accent-primary-soft) 52%, var(--app-panel-bg));
}

.lan-conversation-info {
  display: grid;
  flex: 1;
  gap: 2px;
  min-width: 0;
}

.lan-conversation-name {
  overflow: hidden;
  font-size: 0.76rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lan-conversation-offline {
  flex: 0 0 auto;
  padding: 1px 5px;
  border-radius: 999px;
  background: color-mix(in srgb, var(--app-panel-strong-bg) 80%, transparent);
  color: var(--app-muted);
  font-size: 0.58rem;
}

.lan-conversation-badge {
  min-width: 14px;
  height: 14px;
  flex: 0 0 auto;
  padding: 0 3px;
  border-radius: 999px;
  background: var(--accent-primary-strong);
  color: var(--accent-primary-text);
  font-size: 0.58rem;
  line-height: 14px;
  text-align: center;
}

</style>
