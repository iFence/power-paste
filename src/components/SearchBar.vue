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
        class="toolbar-icon-button danger"
        type="button"
        :title="clearLabel"
        :aria-label="clearLabel"
        @click="onClear"
      >
        <svg viewBox="0 0 16 16" aria-hidden="true">
          <path
            d="M6.2 2.5h3.6l.5 1.3h2.2v1.1H3.5V3.8h2.2l.5-1.3Zm-1 3.1h5.6l-.5 7.2a1 1 0 0 1-1 .9H6.7a1 1 0 0 1-1-.9l-.5-7.2Zm1.8 1.3v4.8h1.1V6.9H7Zm2 0v4.8h1.1V6.9H9Z"
            fill="currentColor"
          />
        </svg>
      </button>
    </div>
  </section>
</template>
