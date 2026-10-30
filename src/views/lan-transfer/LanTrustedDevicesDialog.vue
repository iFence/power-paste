<script setup>
// 已信任设备弹窗：列出被信任的对端并可移除；纯展示组件，不直接访问后端。
const props = defineProps({
  busy: { type: Boolean, default: false },
  devices: { type: Array, default: () => [] },
  t: { type: Function, required: true },
});

const emit = defineEmits(["close", "remove"]);

// 只显示指纹末 8 位：既能区分设备，又不会让整串指纹铺满一行。
function shortFingerprint(device) {
  return String(device?.fingerprint || "").slice(-8);
}
</script>

<template>
  <div class="lan-trusted-backdrop" @click.self="emit('close')">
    <section class="lan-trusted-card" role="dialog" aria-modal="true">
      <header class="lan-trusted-head">
        <strong>{{ t("lanTrustedDevices") }}</strong>
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

      <p class="lan-trusted-hint">{{ t("lanTrustedDevicesTip") }}</p>

      <p v-if="!devices.length" class="lan-trusted-empty">
        {{ t("lanTrustedDevicesEmpty") }}
      </p>
      <ul v-else class="lan-trusted-list">
        <li v-for="device in devices" :key="device.fingerprint">
          <span>
            <strong>{{ device.alias || t("lanTrustedUnknownDevice") }}</strong>
            <small>{{ shortFingerprint(device) }}</small>
          </span>
          <button
            class="ghost compact"
            type="button"
            :disabled="busy"
            @click="emit('remove', device.fingerprint)"
          >
            {{ t("removeAction") }}
          </button>
        </li>
      </ul>
    </section>
  </div>
</template>

<style scoped>
.lan-trusted-backdrop {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(0, 0, 0, 0.42);
}

.lan-trusted-card {
  display: grid;
  gap: 12px;
  width: min(94vw, 420px);
  max-height: min(70vh, 420px);
  padding: 16px;
  overflow: auto;
  border: 1px solid var(--app-panel-border);
  border-radius: 16px;
  background: var(--app-select-menu-bg);
  color: var(--app-select-menu-text);
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.34);
}

.lan-trusted-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.lan-trusted-hint,
.lan-trusted-empty {
  margin: 0;
  color: var(--app-muted);
  font-size: 0.74rem;
  line-height: 1.5;
}

.lan-trusted-empty {
  padding: 14px 0;
  text-align: center;
}
</style>
