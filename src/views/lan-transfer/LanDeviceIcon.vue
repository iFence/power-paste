<script setup>
// 设备类型小图标：手机 / 平板 / 浏览器 / 桌面，用在设备列表每一项的左侧。
import { computed } from "vue";

const props = defineProps({
  device: { type: Object, required: true },
  offline: { type: Boolean, default: false },
});

const type = computed(() => {
  const value = String(props.device?.deviceType || "").toLowerCase();
  if (value.includes("mobile") || value.includes("phone")) {
    return "phone";
  }
  if (value.includes("tablet")) {
    return "tablet";
  }
  if (value.includes("web") || value.includes("browser")) {
    return "browser";
  }
  return "desktop";
});
</script>

<template>
  <span class="lan-device-icon" :data-device="type" :class="{ offline }">
    <svg v-if="type === 'phone'" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="7" y="2.5" width="10" height="19" rx="2" fill="none" stroke="currentColor" stroke-width="2" />
      <path d="M10.3 18.5h3.4" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
    </svg>
    <svg v-else-if="type === 'tablet'" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="4.5" y="3" width="15" height="18" rx="2" fill="none" stroke="currentColor" stroke-width="2" />
    </svg>
    <svg v-else-if="type === 'browser'" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="12" cy="12" r="8.5" fill="none" stroke="currentColor" stroke-width="2" />
      <path d="M3.8 9h16.4M3.8 15h16.4" fill="none" stroke="currentColor" stroke-width="1.7" />
    </svg>
    <svg v-else viewBox="0 0 24 24" aria-hidden="true">
      <rect x="3.5" y="4.5" width="17" height="12" rx="1.8" fill="none" stroke="currentColor" stroke-width="2" />
      <path d="M8.5 20h7M12 16.5V20" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
    </svg>
  </span>
</template>

<style scoped>
/* 只保留线条图标本身，不加底色方块，避免列表显得臃肿。 */
.lan-device-icon {
  display: grid;
  place-items: center;
  width: 14px;
  height: 14px;
  flex: 0 0 auto;
  color: var(--accent-primary);
}

.lan-device-icon.offline {
  color: var(--app-muted);
}

.lan-device-icon svg {
  width: 14px;
  height: 14px;
}
</style>
