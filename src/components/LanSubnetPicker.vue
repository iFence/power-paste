<script setup>
// 网段选择面板：列出本机网卡网段；纯展示组件，不访问后端。
const props = defineProps({
  busy: { type: Boolean, default: false },
  items: { type: Array, default: () => [] },
  lastSubnet: { type: String, default: "" },
  t: { type: Function, required: true },
});

const emit = defineEmits(["close", "select"]);
</script>

<template>
  <section class="lan-subnet-picker">
    <header class="lan-subnet-picker-head">
      <span class="meta-label">{{ t("lanScanMenuTitle") }}</span>
      <button
        class="toolbar-icon-button lan-subnet-picker-close"
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

    <p v-if="!items.length" class="lan-subnet-picker-empty">
      {{ t("lanScanNoInterface") }}
    </p>
    <ul v-else class="lan-subnet-picker-list">
      <li v-for="item in items" :key="item.cidr">
        <button
          class="lan-subnet-picker-item"
          :class="{ selected: item.cidr === lastSubnet }"
          type="button"
          :disabled="busy"
          @click="emit('select', item.cidr)"
        >
          <span class="lan-subnet-picker-info">
            <strong>{{ item.cidr }}</strong>
            <small>{{ item.interface }} · {{ item.address }}</small>
          </span>
          <span v-if="item.cidr === lastSubnet" class="lan-subnet-picker-last">
            {{ t("lanScanLastUsed") }}
          </span>
        </button>
      </li>
    </ul>

    <p class="lan-subnet-picker-hint">{{ t("lanScanMenuHint") }}</p>
  </section>
</template>
