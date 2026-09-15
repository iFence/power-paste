<script setup>
// 未选择设备时的右栏空态：保留本机身份、在线状态、扫码互传入口与高级网络信息。
import { computed, ref } from "vue";

const props = defineProps({
  busy: { type: Boolean, default: false },
  localIps: { type: Array, default: () => [] },
  running: { type: Boolean, required: true },
  state: { type: Object, required: true },
  t: { type: Function, required: true },
});

const emit = defineEmits(["open-link", "start-service"]);

const advanced = ref(false);

const displayAlias = computed(
  () => props.state.alias || props.t("lanTransferUnknownDevice"),
);
const statusText = computed(() =>
  props.running
    ? props.t("lanTransferStatusRunning")
    : props.t("lanTransferStatusStopped"),
);
</script>

<template>
  <section class="lan-idle-panel">
    <div class="lan-idle-center">
      <div class="lan-idle-logo">
        <img src="/localsend.png" alt="" />
      </div>
      <h2>{{ displayAlias }}</h2>
      <p class="lan-idle-status">
        <i :class="{ online: running }"></i>
        {{ statusText }}
      </p>
      <button
        class="lan-idle-link"
        type="button"
        :disabled="busy || !running"
        @click="emit('open-link')"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M10 13.8a4 4 0 0 0 5.7.1l2.6-2.6a4 4 0 0 0-5.7-5.7l-1.5 1.5M14 10.2a4 4 0 0 0-5.7-.1l-2.6 2.6a4 4 0 1 0 5.7 5.7l1.5-1.5"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
          />
        </svg>
        {{ t("lanTransferReceiveLink") }}
      </button>
      <small class="lan-idle-hint">{{ t("lanConversationSelectHint") }}</small>
      <button
        v-if="!running && state.enabled !== false"
        class="primary compact"
        type="button"
        :disabled="busy"
        @click="emit('start-service')"
      >
        {{ t("lanTransferStartService") }}
      </button>
      <small v-if="state.enabled === false" class="lan-idle-disabled">
        {{ t("lanTransferDisabledHint") }}
      </small>
    </div>

    <button
      class="toolbar-icon-button lan-idle-advanced-button"
      :class="{ active: advanced }"
      type="button"
      :title="t('lanTransferAdvanced')"
      :aria-label="t('lanTransferAdvanced')"
      @click="advanced = !advanced"
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="12" cy="12" r="8.2" fill="none" stroke="currentColor" stroke-width="1.8" />
        <path d="M12 10.5v6M12 7.4v.3" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
      </svg>
    </button>

    <aside v-if="advanced" class="lan-idle-advanced">
      <div>
        <span>{{ t("lanTransferDeviceName") }}</span>
        <strong>{{ displayAlias }}</strong>
      </div>
      <div>
        <span>{{ t("lanTransferLocalIp") }}</span>
        <strong v-if="localIps.length">{{ localIps.join(" / ") }}</strong>
        <strong v-else>{{ t("lanTransferUnknownIp") }}</strong>
      </div>
      <div>
        <span>{{ t("lanTransferPort") }}</span>
        <strong>{{ state.port || "-" }}</strong>
      </div>
    </aside>
  </section>
</template>

<style scoped>
.lan-idle-panel {
  position: relative;
  display: grid;
  place-items: center;
  height: 100%;
  min-height: 0;
}

.lan-idle-center {
  display: grid;
  align-content: center;
  justify-items: center;
  gap: 12px;
  padding: 20px;
}

.lan-idle-logo {
  display: grid;
  place-items: center;
  width: 108px;
  height: 108px;
}

.lan-idle-logo img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.lan-idle-center h2 {
  max-width: 100%;
  overflow: hidden;
  margin: 0;
  font-size: clamp(1.5rem, 4vw, 2.3rem);
  line-height: 1.1;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lan-idle-status {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  margin: 0;
  color: var(--app-muted);
  font-size: 0.82rem;
}

.lan-idle-status i {
  width: 9px;
  height: 9px;
  border-radius: 999px;
  background: #f55656;
}

.lan-idle-status i.online {
  background: #44d17f;
  box-shadow: 0 0 10px rgba(68, 209, 127, 0.7);
}

.lan-idle-link {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
  padding: 9px 16px;
  border: 1px solid color-mix(in srgb, var(--accent-primary) 56%, var(--app-panel-border));
  border-radius: 999px;
  background: transparent;
  color: var(--accent-primary);
  font: inherit;
  font-size: 0.82rem;
  font-weight: 650;
  cursor: pointer;
}

.lan-idle-link svg {
  width: 17px;
  height: 17px;
}

.lan-idle-link:disabled {
  cursor: default;
  opacity: 0.45;
}

.lan-idle-hint,
.lan-idle-disabled {
  color: var(--app-muted);
  font-size: 0.74rem;
}

.lan-idle-advanced-button {
  position: absolute;
  top: 14px;
  right: 16px;
}

.lan-idle-advanced-button.active {
  color: var(--accent-primary);
}

.lan-idle-advanced {
  position: absolute;
  right: 16px;
  bottom: 16px;
  display: grid;
  gap: 8px;
  width: min(320px, calc(100% - 32px));
  padding: 12px;
  border: 1px solid var(--app-panel-border);
  border-radius: 13px;
  background: var(--app-select-menu-bg);
  color: var(--app-select-menu-text);
  box-shadow: 0 18px 42px rgba(0, 0, 0, 0.26);
}

.lan-idle-advanced div {
  display: grid;
  grid-template-columns: 88px minmax(0, 1fr);
  gap: 8px;
  align-items: baseline;
}

.lan-idle-advanced span {
  color: var(--app-muted);
  font-size: 0.7rem;
}

.lan-idle-advanced strong {
  overflow: hidden;
  font-size: 0.75rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
