<script setup>
import { ref, computed } from "vue";

const props = defineProps({
    message: {
        type: Object,
        default: null,
    },
    formatAddress: {
        type: Function,
        required: true,
    },
    formatDate: {
        type: Function,
        required: true,
    },
});

const viewMode = ref("text");

function highlight(json) {
    const str = JSON.stringify(json, null, 2);
    return str.replace(
        /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
        (match) => {
            if (/^"/.test(match))
                return /:$/.test(match)
                    ? `<span class="json-key">${match}</span>`
                    : `<span class="json-string">${match}</span>`;
            if (/true|false/.test(match))
                return `<span class="json-bool">${match}</span>`;
            if (/null/.test(match))
                return `<span class="json-null">${match}</span>`;
            return `<span class="json-number">${match}</span>`;
        },
    );
}

const highlightedJson = computed(() =>
    props.message ? highlight(props.message) : "",
);

defineExpose({ viewMode });
</script>

<template>
    <div class="flex flex-col flex-1 overflow-hidden">
        <!-- Toolbar -->
        <div
            class="flex items-center gap-2 px-3 h-8 shrink-0 text-base font-semibold uppercase tracking-widest"
            style="
                background: var(--surface-raised);
                border-bottom: 1px solid var(--border-subtle);
                color: var(--text-tertiary);
            "
        >
            <span>Message</span>
            <div v-if="message" class="ml-auto flex items-center gap-1">
                <button
                    class="btn btn-ghost !h-6 text-base font-semibold uppercase tracking-wider border"
                    :style="
                        viewMode === 'text'
                            ? 'color: var(--accent); border-color: var(--accent-border); background: var(--accent-subtle)'
                            : 'border-color: var(--border-subtle); color: var(--text-tertiary)'
                    "
                    @click="viewMode = 'text'"
                >
                    Text
                </button>
                <button
                    class="btn btn-ghost !h-6 text-base font-semibold uppercase tracking-wider border"
                    :style="
                        viewMode === 'json'
                            ? 'color: var(--accent); border-color: var(--accent-border); background: var(--accent-subtle)'
                            : 'border-color: var(--border-subtle); color: var(--text-tertiary)'
                    "
                    @click="viewMode = 'json'"
                >
                    JSON
                </button>
            </div>
        </div>

        <!-- Content -->
        <div
            class="flex-1 overflow-y-auto"
            style="background: var(--surface-base)"
        >
            <!-- Empty state -->
            <div
                v-if="!message"
                class="flex flex-col items-center justify-center gap-1.5 h-full text-base tracking-wide"
                style="color: var(--text-tertiary)"
            >
                <span class="text-3xl opacity-20">✉</span>
                <span>select a message</span>
            </div>

            <!-- Text view -->
            <div v-else-if="viewMode === 'text'" class="p-6">
                <h2
                    class="text-lg font-bold leading-snug mb-3"
                    style="color: var(--text-primary); letter-spacing: -0.01em"
                >
                    {{ message.subject || "(no subject)" }}
                </h2>
                <dl class="flex flex-col gap-1.5">
                    <div
                        v-for="[label, val] in [
                            ['From', formatAddress(message.envelope?.from)],
                            ['To', formatAddress(message.envelope?.to)],
                            ['Date', formatDate(message.envelope?.date)],
                        ]"
                        :key="label"
                        class="flex gap-3 items-baseline"
                    >
                        <dt
                            class="text-base font-semibold uppercase tracking-widest w-8 shrink-0"
                            style="color: var(--text-tertiary)"
                        >
                            {{ label }}
                        </dt>
                        <dd
                            class="text-base"
                            style="color: var(--text-secondary)"
                        >
                            {{ val }}
                        </dd>
                    </div>
                </dl>
                <div
                    class="my-4"
                    style="height: 1px; background: var(--border-subtle)"
                ></div>
                <pre
                    class="text-base leading-relaxed whitespace-pre-wrap break-words"
                    style="
                        font-family: var(--font-mono);
                        color: var(--text-secondary);
                    "
                    >{{ message.body_text?.[0] || "(empty)" }}</pre
                >
            </div>

            <!-- JSON view -->
            <div v-else class="p-6">
                <pre
                    class="text-base leading-relaxed whitespace-pre overflow-x-auto"
                    style="
                        font-family: var(--font-mono);
                        color: var(--text-secondary);
                    "
                    v-html="highlightedJson"
                ></pre>
            </div>
        </div>
    </div>
</template>

<style scoped>
:deep(.json-key) {
    color: #a78bfa;
}
:deep(.json-string) {
    color: color-mix(in srgb, var(--status-ok) 90%, var(--text-primary));
}
:deep(.json-number) {
    color: color-mix(in srgb, var(--status-warn) 90%, var(--text-primary));
}
:deep(.json-bool) {
    color: #38bdf8;
}
:deep(.json-null) {
    color: var(--text-tertiary);
}
</style>
