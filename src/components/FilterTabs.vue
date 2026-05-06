<script setup>
defineProps({
  activeFilterTab: { type: String, required: true },
  activeTagFilter: { type: String, default: '' },
  ariaLabel: { type: String, required: true },
  tabs: { type: Array, required: true },
  tagFilters: { type: Array, default: () => [] },
  tagLabelPrefix: { type: String, required: true },
});

const emit = defineEmits(["select", "select-tag"]);
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
    <div v-if="tagFilters.length" class="tag-filter-row" :aria-label="tagLabelPrefix">
      <button
        v-for="tag in tagFilters"
        :key="tag.key"
        class="tag-filter-chip"
        :class="[`history-tag-${tag.color}`, { active: activeTagFilter === tag.key }]"
        type="button"
        @click="emit('select-tag', tag.key)"
      >
        <span class="history-tag-dot" :class="`history-tag-${tag.color}`"></span>
        <span>{{ tag.label }}</span>
      </button>
    </div>
  </section>
</template>
