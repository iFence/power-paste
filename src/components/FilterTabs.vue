<script setup>
defineProps({
  activeFilterTab: { type: String, required: true },
  ariaLabel: { type: String, required: true },
  tabs: { type: Array, required: true },
});

const emit = defineEmits(["select"]);
</script>

<template>
  <section class="filter-tabs-shell" aria-label="History filters">
    <div class="filter-tabs" role="tablist" :aria-label="ariaLabel">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="filter-tab"
        :class="{ active: activeFilterTab === tab.key }"
        type="button"
        role="tab"
        :aria-selected="activeFilterTab === tab.key"
        @click="emit('select', tab.key)"
      >
        {{ tab.label }}
      </button>
    </div>
  </section>
</template>
