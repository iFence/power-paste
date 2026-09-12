<script setup>
// 收到局域网传输请求时的确认弹窗：接受、拒绝或接受并信任该设备。
import { computed } from "vue";

const props = defineProps({
    busy: { type: Boolean, default: false },
    request: { type: Object, required: true },
    t: { type: Function, required: true },
    onRespond: { type: Function, required: true },
});

const files = computed(() =>
    Array.isArray(props.request.files) ? props.request.files : [],
);
const isTextMessage = computed(() => Boolean(props.request.textMessage));

function formatBytes(size) {
    const value = Number(size || 0);
    if (value < 1000) {
        return `${value} B`;
    }
    if (value < 1_000_000) {
        return `${Math.round(value / 1000)} KB`;
    }
    return `${(value / 1_000_000).toFixed(1)} MB`;
}
</script>

<template>
    <div class="lan-incoming-backdrop">
        <section class="lan-incoming-card" role="dialog" aria-modal="true">
            <h2>{{ t("lanIncomingTitle") }}</h2>
            <p class="lan-incoming-from">
                <strong>{{ request.alias }}</strong>
                <small>
                    {{ request.deviceModel || "" }}
                    {{ request.deviceModel ? " · " : "" }}{{ request.ip }}
                </small>
            </p>

            <pre v-if="isTextMessage" class="lan-incoming-text">{{
                request.textMessage
            }}</pre>
            <ul v-else class="lan-incoming-files">
                <li v-for="file in files" :key="file.fileName">
                    <span>{{ file.fileName }}</span>
                    <small>{{ formatBytes(file.size) }}</small>
                </li>
            </ul>

            <div class="lan-incoming-actions">
                <button
                    class="ghost compact"
                    type="button"
                    :disabled="busy"
                    @click="onRespond(request.requestId, 'decline')"
                >
                    {{ t("lanIncomingDecline") }}
                </button>
                <button
                    class="ghost compact"
                    type="button"
                    :disabled="busy"
                    @click="onRespond(request.requestId, 'accept-and-trust')"
                >
                    {{ t("lanIncomingAcceptAndTrust") }}
                </button>
                <button
                    class="primary compact"
                    type="button"
                    :disabled="busy"
                    @click="onRespond(request.requestId, 'accept')"
                >
                    {{ t("lanIncomingAccept") }}
                </button>
            </div>
        </section>
    </div>
</template>
