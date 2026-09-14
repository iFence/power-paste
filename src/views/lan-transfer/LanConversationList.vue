<script setup>
// 会话列表：只列出发现的设备，底部是刷新 / 手动添加 / 历史工具。
import LanDeviceIcon from "./LanDeviceIcon.vue";

const props = defineProps({
  activeFingerprint: { type: String, default: "" },
  conversations: { type: Array, default: () => [] },
  dropTargetFingerprint: { type: String, default: "" },
  running: { type: Boolean, required: true },
  scanRunning: { type: Boolean, default: false },
  t: { type: Function, required: true },
});

const emit = defineEmits(["add-device", "refresh", "select"]);
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

    <div class="lan-conversation-tools">
      <button
        class="toolbar-icon-button"
        :class="{ spinning: scanRunning }"
        type="button"
        :title="scanRunning ? t('lanScanCancel') : t('lanTransferRefresh')"
        :aria-label="t('lanTransferRefresh')"
        :disabled="!running"
        @click="emit('refresh')"
      >
        <svg viewBox="0 0 1024 1024" aria-hidden="true">
          <path
            d="M958.681412 457.499032c-6.170072-50.632177-20.854483-99.563886-43.643361-145.434552-45.779694-92.144205-122.249797-166.333021-215.325711-208.898719-20.083724-9.18513-43.810309-0.349891-52.995439 19.734833-9.18413 20.082724-0.349891 43.810309 19.733833 52.996438 159.26323 72.834239 245.755201 249.640987 205.658732 420.410622-30.735395 130.876101-129.201624 233.321087-256.187941 270.333521l-0.262918-70.800875-196.843487 114.650172 197.690222 113.176632-0.275914-74.43274c75.398438-17.911403 144.809747-54.929834 202.084849-108.039237 65.597501-60.827991 111.122274-139.186504 131.651859-226.606186 12.170197-51.828803 15.10328-104.683286 8.715276-157.089909zM408.299406-0.001l0.271915 74.43374c-75.404436 17.911403-144.820744 54.931834-202.099843 108.046235-65.6005 60.83099-111.124274 139.191503-131.651859 226.616183-7.987504 34.034364-11.994252 68.507591-11.994252 103.010809 0 17.994377 1.090659 35.996751 3.271978 53.946142 6.152077 50.59119 20.803499 99.48891 43.545392 145.333583 45.678725 92.080225 122.012871 166.270041 214.936832 208.900718 20.071728 9.209122 43.810309 0.401874 53.018432-19.670852 9.210122-20.076726 0.400875-43.810309-19.671853-53.019432-158.963324-72.92821-245.278351-249.658982-205.24886-420.22368 30.732396-130.883099 129.201624-233.333083 256.195939-270.345517l0.259919 70.801874 196.850484-114.640174L408.299406-0.001z"
            fill="currentColor"
          />
        </svg>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.lan-conversation-list {
  display: grid;
  grid-template-rows: minmax(0, 1fr) auto;
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
  border-color: color-mix(in srgb, var(--accent-primary) 42%, var(--app-panel-border));
  background: color-mix(in srgb, var(--accent-primary-soft) 32%, var(--app-panel-bg));
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

.lan-conversation-tools {
  display: flex;
  justify-content: center;
  gap: 4px;
  padding: 6px;
  border-top: 1px solid var(--app-panel-border);
}

.lan-conversation-tools .toolbar-icon-button.spinning svg {
  animation: lan-conversation-spin 900ms linear infinite;
}

@keyframes lan-conversation-spin {
  to {
    transform: rotate(360deg);
  }
}

</style>
