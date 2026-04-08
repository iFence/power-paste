<script setup>
defineProps({
  actionFeedback: { type: String, default: "" },
  onInstallUpdate: { type: Function, required: true },
  onWindowAction: { type: Function, required: true },
  onClear: { type: Function, required: true },
  onOpenSettings: { type: Function, required: true },
  placeholder: { type: String, required: true },
  query: { type: String, required: true },
  settingsLabel: { type: String, required: true },
  showUpdateAction: { type: Boolean, required: true },
  updateLabel: { type: String, required: true },
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
      <button
        v-if="showUpdateAction"
        class="toolbar-icon-button update-available-button"
        type="button"
        :title="updateLabel"
        :aria-label="updateLabel"
        @click="onInstallUpdate"
      >
        <svg viewBox="0 0 1024 1024" aria-hidden="true">
          <path
            d="M245.76 286.72h552.96c124.928 0 225.28 100.352 225.28 225.28s-100.352 225.28-225.28 225.28H0V532.48c0-135.168 110.592-245.76 245.76-245.76z m133.12 348.16V401.408H348.16v178.176l-112.64-178.176H204.8V634.88h30.72v-178.176L348.16 634.88h30.72z m182.272-108.544v-24.576h-96.256v-75.776h110.592v-24.576h-141.312V634.88h143.36v-24.576h-112.64v-83.968h96.256z m100.352 28.672l-34.816-151.552h-34.816l55.296 233.472H675.84l47.104-161.792 4.096-20.48 4.096 20.48 47.104 161.792h28.672l57.344-233.472h-34.816l-32.768 151.552-4.096 30.72-6.144-30.72-40.96-151.552h-30.72l-40.96 151.552-6.144 30.72-6.144-30.72z"
            fill="currentColor"
          />
        </svg>
      </button>
      <button class="toolbar-icon-button" type="button" :title="settingsLabel" :aria-label="settingsLabel" @click="onOpenSettings">
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
