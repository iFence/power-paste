<script setup>
const props = defineProps({
    busy: { type: Boolean, required: true },
    error: { type: String, default: "" },

    onClose: { type: Function, required: true },
    show: { type: Boolean, required: true },
    state: { type: Object, required: true },
    statusLabel: { type: String, required: true },
    t: { type: Function, required: true },
});

const emit = defineEmits(["close"]);

async function closeModal() {
    await props.onClose();
    emit("close");
}
</script>

<template>
    <div v-if="show" class="modal-backdrop" @click="closeModal">
        <section class="lan-receiver-modal" @click.stop>
            <header class="modal-header lan-receiver-header">
                <div class="modal-title-block">
                    <h2>{{ t("lanReceiverTitle") }}</h2>
                    <span class="modal-version">{{
                        t("lanReceiverSubtitle")
                    }}</span>
                </div>
            </header>

            <div class="lan-receiver-body">
                <div class="lan-qr-frame" v-html="state.qrSvg"></div>
                <div class="lan-receiver-meta">
                    <span class="meta-label">{{ t("lanReceiverStatus") }}</span>
                    <strong>{{ statusLabel }}</strong>
                    <p>
                        {{ t("lanTransferConnectedDevices") }}:
                        {{ state.connectedDevices ?? 0 }}
                    </p>
                </div>
                <input
                    class="lan-url-input"
                    type="text"
                    readonly
                    :value="state.url || ''"
                />
                <p v-if="error" class="settings-save-feedback">{{ error }}</p>
            </div>

            <footer class="modal-footer lan-receiver-footer">
                <button
                    class="ghost"
                    type="button"
                    :disabled="busy"
                    @click="closeModal"
                >
                    {{ t("closeAction") }}
                </button>
            </footer>
        </section>
    </div>
</template>
