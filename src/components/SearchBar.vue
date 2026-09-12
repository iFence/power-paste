<script setup>
const props = defineProps({
  actionFeedback: { type: String, default: '' },
  clearLabel: { type: String, required: true },
  clearSearchLabel: { type: String, required: true },
  onClear: { type: Function, required: true },
  onClearQuery: { type: Function, required: true },
  onOpenLanReceiver: { type: Function, required: true },
  onOpenSettings: { type: Function, required: true },
  onWindowAction: { type: Function, required: true },
  placeholder: { type: String, required: true },
  query: { type: String, required: true },
  settingsLabel: { type: String, required: true },
  lanReceiverLabel: { type: String, required: true },
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
      <button
        class="toolbar-icon-button searchbar-qr-button"
        type="button"
        :title="props.lanReceiverLabel"
        :aria-label="props.lanReceiverLabel"
        @click="props.onOpenLanReceiver"
      >
        <svg viewBox="0 0 1024 1024" aria-hidden="true">
          <path
            d="M435.2 894.464c-34.816-7.68-37.888-16.896-12.8-38.912l18.432-15.872h143.872l17.92 17.92c9.728 9.728 16.896 20.48 15.872 24.064-3.584 10.24-52.224 18.432-106.496 18.432-26.624 0-61.44-2.56-76.8-5.632zM293.376 834.56c-34.816-23.04-82.944-71.68-108.032-110.08-18.944-28.672-16.384-33.28 18.944-33.28 21.504 0 24.064 2.048 75.264 53.76 51.2 51.2 53.248 54.272 53.248 75.264 0 36.352-4.608 37.888-39.424 14.336zM694.272 846.848c-1.536-4.608-3.072-17.408-3.072-28.672 0-18.944 3.584-23.552 53.248-73.728 51.712-51.2 54.272-53.248 75.776-53.248 34.816 0 37.376 4.608 18.432 33.792-21.504 32.768-93.184 103.936-120.32 118.784-18.432 9.728-21.504 10.24-24.064 3.072zM463.36 716.8c-117.248-29.184-186.88-152.064-151.552-266.752 39.424-125.952 188.416-186.368 301.568-121.344 132.608 76.8 145.92 258.048 25.088 349.696-48.128 36.352-119.808 52.224-175.104 38.4zM136.192 615.424c-5.12-8.704-13.312-71.168-13.312-103.424 0-37.888 8.704-97.792 15.36-104.448 6.144-6.144 33.792 11.776 40.448 26.112 7.68 16.384 7.168 142.336 0 156.672-11.776 20.992-35.84 35.328-42.496 25.088zM855.552 601.6l-15.872-18.432v-68.608c0-38.4 2.56-74.24 5.632-80.896 6.656-14.336 34.304-32.256 40.448-26.112 14.336 14.848 19.456 128.512 8.704 181.248-7.68 34.816-16.896 37.888-38.912 12.8zM175.104 328.704c-14.848-9.728 65.536-104.448 118.272-139.264 35.328-23.04 39.424-21.504 39.424 14.336 0 21.504-2.048 24.576-53.248 75.776-51.712 51.2-54.272 53.248-75.776 53.248-12.288 0-25.088-2.048-28.672-4.096zM744.96 279.552c-51.712-51.712-53.76-54.272-53.76-75.776 0-36.864 4.096-37.888 44.032-10.24 40.448 28.672 81.408 70.656 103.936 106.496 18.432 28.16 15.872 32.768-18.944 32.768-20.992 0-24.064-2.048-75.264-53.248zM432.64 178.688c-14.336-7.168-29.696-29.184-26.112-37.888 3.584-9.728 41.984-15.36 105.472-15.36 66.048 0 101.888 5.632 105.984 16.384 2.048 5.632-3.072 14.336-15.36 25.6l-18.944 16.896-70.144-0.512c-38.912 0-75.264-2.56-80.896-5.12z"
            fill="currentColor"
          />
        </svg>
      </button>
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
