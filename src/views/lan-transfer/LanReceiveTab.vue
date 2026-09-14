<script setup>
// 接收主页面：设备身份、在线状态、链接入口与高级网络信息。
import { computed, ref } from "vue";

const props = defineProps({
  busy: { type: Boolean, default: false },
  historyCount: { type: Number, default: 0 },
  localIps: { type: Array, default: () => [] },
  running: { type: Boolean, required: true },
  state: { type: Object, required: true },
  t: { type: Function, required: true },
});

const emit = defineEmits([
  "open-history",
  "open-link",
  "start-service",
  "toggle-advanced",
]);

const advanced = ref(false);

const displayAlias = computed(() => props.state.alias || props.t("lanUnknownDevice"));
const statusText = computed(() =>
  props.running ? props.t("lanTransferStatusRunning") : props.t("lanTransferStatusStopped"),
);

function toggleAdvanced() {
  advanced.value = !advanced.value;
  emit("toggle-advanced", advanced.value);
}
</script>

<template>
  <section class="lan-receive-tab">
    <div class="lan-receive-corner">
      <button
        class="toolbar-icon-button"
        type="button"
        :title="t('lanTransferHistory')"
        :aria-label="t('lanTransferHistory')"
        @click="emit('open-history')"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path
            d="M12 7v5l3 2M4.5 12a7.5 7.5 0 1 0 2.2-5.3L4 9.4M4 5v4.4h4.4"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
          />
        </svg>
        <span v-if="historyCount" class="lan-receive-badge">{{ historyCount }}</span>
      </button>
      <button
        class="toolbar-icon-button"
        :class="{ active: advanced }"
        type="button"
        :title="t('lanTransferAdvanced')"
        :aria-label="t('lanTransferAdvanced')"
        @click="toggleAdvanced"
      >
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <circle cx="12" cy="12" r="8.2" fill="none" stroke="currentColor" stroke-width="1.8" />
          <path d="M12 10.5v6M12 7.4v.3" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
        </svg>
      </button>
    </div>

    <div class="lan-receive-center">
      <div class="lan-receive-logo">
        <img src="/localsend.png" alt="" />
      </div>
      <h2>{{ displayAlias }}</h2>
      <p class="lan-receive-status">
        <i :class="{ online: running }"></i>
        {{ statusText }}
      </p>
      <button
        class="lan-receive-link"
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
      <button
        v-if="!running && state.enabled !== false"
        class="primary compact"
        type="button"
        :disabled="busy"
        @click="emit('start-service')"
      >
        {{ t("lanTransferStartService") }}
      </button>
      <small v-if="state.enabled === false" class="lan-receive-disabled">
        {{ t("lanTransferDisabledHint") }}
      </small>
    </div>

    <aside v-if="advanced" class="lan-receive-advanced">
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
.lan-receive-tab {
  position: relative;
  display: grid;
  height: 100%;
  min-height: 0;
  padding: 26px 30px 30px;
}

.lan-receive-corner {
  position: absolute;
  top: 16px;
  right: 18px;
  display: flex;
  gap: 8px;
  z-index: 2;
}

.lan-receive-corner .toolbar-icon-button {
  position: relative;
}

.lan-receive-corner .toolbar-icon-button.active {
  color: var(--accent-primary);
}

.lan-receive-badge {
  position: absolute;
  top: -2px;
  right: -2px;
  min-width: 15px;
  height: 15px;
  padding: 0 4px;
  border-radius: 999px;
  background: var(--accent-primary-strong);
  color: var(--accent-primary-text);
  font-size: 0.6rem;
  line-height: 15px;
  text-align: center;
}

.lan-receive-center {
  display: grid;
  align-content: center;
  justify-items: center;
  gap: 13px;
  min-height: 0;
}

.lan-receive-logo {
  display: grid;
  place-items: center;
  width: 128px;
  height: 128px;
}

.lan-receive-logo img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.lan-receive-center h2 {
  max-width: 100%;
  overflow: hidden;
  margin: 0;
  font-size: clamp(1.8rem, 5vw, 3rem);
  line-height: 1.1;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lan-receive-status {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  margin: 0;
  color: var(--app-muted);
  font-size: 0.86rem;
}

.lan-receive-status i {
  width: 9px;
  height: 9px;
  border-radius: 999px;
  background: #f55656;
}

.lan-receive-status i.online {
  background: #44d17f;
  box-shadow: 0 0 10px rgba(68, 209, 127, 0.7);
}

.lan-receive-link {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  margin-top: 8px;
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

.lan-receive-link svg {
  width: 17px;
  height: 17px;
}

.lan-receive-link:disabled {
  cursor: default;
  opacity: 0.45;
}

.lan-receive-disabled {
  color: var(--app-muted);
  font-size: 0.74rem;
}

.lan-receive-advanced {
  position: absolute;
  right: 18px;
  bottom: 18px;
  display: grid;
  gap: 8px;
  width: min(330px, calc(100% - 36px));
  padding: 12px;
  border: 1px solid var(--app-panel-border);
  border-radius: 13px;
  background: var(--app-select-menu-bg);
  box-shadow: 0 18px 42px rgba(0, 0, 0, 0.26);
}

.lan-receive-advanced div {
  display: grid;
  grid-template-columns: 88px minmax(0, 1fr);
  gap: 8px;
  align-items: baseline;
}

.lan-receive-advanced span {
  color: var(--app-muted);
  font-size: 0.7rem;
}

.lan-receive-advanced strong {
  overflow: hidden;
  font-size: 0.75rem;
  text-overflow: ellipsis;
  white-space: nowrap;
}

@media (max-width: 699px) {
  .lan-receive-tab {
    padding: 22px 18px 84px;
  }

  .lan-receive-advanced {
    right: 14px;
    bottom: 78px;
  }
}
</style>
