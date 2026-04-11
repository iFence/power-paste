<script setup>
const props = defineProps({
  actionFeedback: { type: String, default: '' },
  clearLabel: { type: String, required: true },
  clearSearchLabel: { type: String, required: true },
  onClear: { type: Function, required: true },
  onClearQuery: { type: Function, required: true },
  onOpenSettings: { type: Function, required: true },
  onWindowAction: { type: Function, required: true },
  placeholder: { type: String, required: true },
  query: { type: String, required: true },
  settingsLabel: { type: String, required: true },
})

const emit = defineEmits(['update:query'])

function handleInput(event) {
  const target = event.target
  emit('update:query', target.value)
}
</script>

<template>
  <section class="searchbar-shell">
    <div class="titlebar-search searchbar-search-group">
      <div class="search-input-wrap">
        <input
          id="history-search"
          :value="props.query"
          class="search"
          type="text"
          :placeholder="props.placeholder"
          @input="handleInput"
        />
        <button
          v-if="props.query"
          class="shortcut-clear-button"
          type="button"
          :title="props.clearSearchLabel"
          :aria-label="props.clearSearchLabel"
          @click="props.onClearQuery"
        >
          <span aria-hidden="true">×</span>
        </button>
      </div>
      <p v-if="actionFeedback" class="action-feedback">{{ actionFeedback }}</p>
    </div>

    <div class="titlebar-actions searchbar-actions action-cluster">
      <button class="toolbar-icon-button" type="button" :title="props.settingsLabel" :aria-label="props.settingsLabel" @click="props.onOpenSettings">
        <svg viewBox="0 0 1024 1024" aria-hidden="true">
          <path
            d="M816.64 551.936c1.536-12.8 2.56-26.112 2.56-39.936 0-13.824-1.024-27.136-3.072-39.936l86.528-67.584a21.162667 21.162667 0 0 0 5.12-26.112l-81.92-141.824a20.821333 20.821333 0 0 0-25.088-9.216l-101.888 40.96a299.946667 299.946667 0 0 0-69.12-39.936l-15.36-108.544a20.437333 20.437333 0 0 0-20.48-17.408h-163.84a19.925333 19.925333 0 0 0-19.968 17.408l-15.36 108.544a308.010667 308.010667 0 0 0-69.12 39.936l-101.888-40.96a20.266667 20.266667 0 0 0-25.088 9.216l-81.92 141.824a19.84 19.84 0 0 0 5.12 26.112l86.528 67.584c-2.048 12.8-3.584 26.624-3.584 39.936 0 13.312 1.024 27.136 3.072 39.936L121.344 619.52a21.162667 21.162667 0 0 0-5.12 26.112l81.92 141.824c5.12 9.216 15.872 12.288 25.088 9.216l101.888-40.96a299.946667 299.946667 0 0 0 69.12 39.936l15.36 108.544c2.048 10.24 10.24 17.408 20.48 17.408h163.84c10.24 0 18.944-7.168 19.968-17.408l15.36-108.544a308.010667 308.010667 0 0 0 69.12-39.936l101.888 40.96c9.216 3.584 19.968 0 25.088-9.216l81.92-141.824a19.84 19.84 0 0 0-5.12-26.112l-85.504-67.584zM512 665.6A154.026667 154.026667 0 0 1 358.4 512c0-84.48 69.12-153.6 153.6-153.6s153.6 69.12 153.6 153.6-69.12 153.6-153.6 153.6z"
            fill="currentColor"
          />
        </svg>
      </button>
      <button
        class="toolbar-icon-button danger clear-history-button"
        type="button"
        :title="props.clearLabel"
        :aria-label="props.clearLabel"
        @click="props.onClear"
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
