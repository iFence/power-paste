<script setup>
defineProps({
  actionFeedback: { type: String, default: "" },
  onWindowAction: { type: Function, required: true },
  onClear: { type: Function, required: true },
  onOpenSettings: { type: Function, required: true },
  placeholder: { type: String, required: true },
  query: { type: String, required: true },
  settingsLabel: { type: String, required: true },
  clearLabel: { type: String, required: true },
});

const emit = defineEmits(["update:query"]);
</script>

<template>
  <section class="searchbar-shell">
    <div class="titlebar-search searchbar-search-group">
      <input
        id="history-search"
        :value="query"
        class="search"
        type="search"
        :placeholder="placeholder"
        @input="emit('update:query', $event.target.value)"
      />
      <p v-if="actionFeedback" class="action-feedback">{{ actionFeedback }}</p>
    </div>

    <div class="titlebar-actions searchbar-actions action-cluster">
      <button class="toolbar-icon-button" type="button" :title="settingsLabel" :aria-label="settingsLabel" @click="onOpenSettings">
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M6.6 1.9h2.8l.3 1.5a4.9 4.9 0 0 1 1.1.5l1.4-.7 1.4 1.4-.7 1.4c.2.4.4.7.5 1.1l1.5.3v2l-1.5.3a4.9 4.9 0 0 1-.5 1.1l.7 1.4-1.4 1.4-1.4-.7a4.9 4.9 0 0 1-1.1.5l-.3 1.5H6.6l-.3-1.5a4.9 4.9 0 0 1-1.1-.5l-1.4.7-1.4-1.4.7-1.4a4.9 4.9 0 0 1-.5-1.1l-1.5-.3v-2l1.5-.3c.1-.4.3-.7.5-1.1l-.7-1.4L3.8 3l1.4.7c.4-.2.7-.4 1.1-.5l.3-1.3Zm1.4 3.5a2.6 2.6 0 1 0 0 5.2 2.6 2.6 0 0 0 0-5.2Z"
            fill="currentColor"
          />
        </svg>
      </button>
      <button
        class="toolbar-icon-button danger clear-history-button"
        type="button"
        :title="clearLabel"
        :aria-label="clearLabel"
        @click="onClear"
      >
        <svg viewBox="0 0 1024 1024" aria-hidden="true" class="delete-action-icon">
          <path
            d="M896 352l-73.792 556.608A96 96 0 0 1 727.04 992H296.96a96 96 0 0 1-95.168-83.392L128 352h768zM528 32A80 80 0 0 1 608 112V128h288a64 64 0 1 1 0 128H128a64 64 0 1 1 0-128h320v-16A80 80 0 0 1 528 32z"
            fill="currentColor"
          />
        </svg>
      </button>
    </div>
  </section>
</template>
