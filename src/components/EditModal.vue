<script setup>
defineProps({
  draft: { type: String, required: true },
  show: { type: Boolean, required: true },
  t: { type: Function, required: true },
});

const emit = defineEmits(["close", "save", "update:draft"]);
</script>

<template>
  <div v-if="show" class="modal-backdrop">
    <section class="edit-modal" @click.stop>
      <header class="modal-header">
        <div class="edit-modal-title">
          <h2>{{ t("editTextItem") }}</h2>
        </div>
      </header>

      <textarea
        :value="draft"
        class="edit-textarea"
        rows="10"
        autofocus
        @input="emit('update:draft', $event.target.value)"
      />

      <footer class="modal-footer edit-modal-footer">
        <button class="ghost" @click="emit('close')">
          {{ t("closeAction") }}
        </button>
        <button class="primary edit-save-button" @click="emit('save')">
          {{ t("saveChanges") }}
        </button>
      </footer>
    </section>
  </div>
</template>
