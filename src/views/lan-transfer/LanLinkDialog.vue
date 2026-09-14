<script setup>
// 浏览器互传弹窗：手机扫码打开页面，把文件或文字传回本机。
import { computed } from "vue";

const props = defineProps({
  busy: { type: Boolean, default: false },
  state: { type: Object, required: true },
  t: { type: Function, required: true },
});

const emit = defineEmits(["close"]);

const webUrl = computed(() => props.state.webUrl || "");

async function copyUrl() {
  if (webUrl.value) {
    await navigator.clipboard.writeText(webUrl.value);
  }
}
</script>

<template>
  <div class="lan-link-backdrop" @click.self="emit('close')">
    <section class="lan-link-card" role="dialog" aria-modal="true">
      <header class="lan-link-head">
        <strong>{{ t("lanTransferWebReceive") }}</strong>
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

      <div v-if="webUrl" class="lan-link-body">
        <div class="lan-link-qr" v-html="state.webQrSvg"></div>
        <div class="lan-link-info">
          <a class="lan-link-url" :href="webUrl" target="_blank" rel="noreferrer">
            {{ webUrl }}
          </a>
          <p>{{ t("lanTransferWebReceiveHint") }}</p>
          <div class="lan-link-actions">
            <button class="ghost compact" type="button" @click="copyUrl">
              {{ t("copy") }}
            </button>
            <a class="primary compact" :href="webUrl" target="_blank" rel="noreferrer">
              {{ t("lanTransferWebOpen") }}
            </a>
          </div>
        </div>
      </div>

      <p v-else class="lan-link-loading">
        {{ busy ? t("lanTransferWebStarting") : t("lanTransferWebIdleHint") }}
      </p>

    </section>
  </div>
</template>

<style scoped>
.lan-link-backdrop {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: grid;
  place-items: center;
  padding: 20px;
  background: rgba(0, 0, 0, 0.42);
}

.lan-link-card {
  display: grid;
  gap: 14px;
  width: min(94vw, 460px);
  padding: 16px;
  border: 1px solid var(--app-panel-border);
  border-radius: 16px;
  background: var(--app-select-menu-bg);
  color: var(--app-select-menu-text);
  box-shadow: 0 24px 60px rgba(0, 0, 0, 0.34);
}

.lan-link-head,
.lan-link-actions {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
}

.lan-link-body {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr);
  gap: 16px;
  align-items: center;
}

.lan-link-qr {
  display: grid;
  place-items: center;
  width: 144px;
  height: 144px;
  padding: 8px;
  border-radius: 12px;
  background: #fff;
}

.lan-link-qr :deep(svg) {
  width: 100%;
  height: 100%;
}

.lan-link-info {
  display: grid;
  gap: 10px;
  min-width: 0;
}

.lan-link-info p,
.lan-link-loading {
  margin: 0;
  color: var(--app-muted);
  font-size: 0.75rem;
  line-height: 1.5;
}

.lan-link-url {
  overflow: hidden;
  padding: 8px 10px;
  border: 1px solid var(--app-panel-border);
  border-radius: 9px;
  background: var(--app-input-bg);
  color: var(--app-text);
  font-size: 0.78rem;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lan-link-actions {
  justify-content: flex-start;
}

.lan-link-actions a {
  display: inline-flex;
  align-items: center;
  text-decoration: none;
}

@media (max-width: 520px) {
  .lan-link-body {
    grid-template-columns: 1fr;
    justify-items: center;
  }

  .lan-link-info {
    width: 100%;
  }
}
</style>
