<script setup>
import { ref, computed } from "vue";

const props = defineProps({
    message: { type: Object, default: null },
    formatAddress: { type: Function, required: true },
    formatDate: { type: Function, required: true },
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
</script>

<template>
    <div class="flex flex-col flex-1 overflow-hidden">
        <!-- Toolbar -->
        <div
            class="flex items-center gap-2 px-3 h-8 shrink-0 bg-surface-raised border-b border-border-subtle text-base font-semibold uppercase tracking-widest text-tertiary"
        >
            <span>Message</span>
            <div v-if="message" class="ml-auto flex items-center gap-1">
                <button
                    class="h-6 px-2 text-base font-semibold uppercase tracking-wider border rounded transition-colors"
                    :class="
                        viewMode === 'text'
                            ? 'text-accent border-accent-border bg-accent-subtle'
                            : 'border-border-subtle text-tertiary hover:text-secondary'
                    "
                    @click="viewMode = 'text'"
                >
                    Text
                </button>
                <button
                    class="h-6 px-2 text-base font-semibold uppercase tracking-wider border rounded transition-colors"
                    :class="
                        viewMode === 'json'
                            ? 'text-accent border-accent-border bg-accent-subtle'
                            : 'border-border-subtle text-tertiary hover:text-secondary'
                    "
                    @click="viewMode = 'json'"
                >
                    JSON
                </button>
            </div>
        </div>

        <!-- Content -->
        <div class="flex-1 overflow-y-auto bg-surface-base">
            <!-- Empty state -->
            <div
                v-if="!message"
                class="flex flex-col items-center justify-center gap-1.5 h-full text-base tracking-wide text-tertiary"
            >
                <span class="text-3xl opacity-20">✉</span>
                <span>select a message</span>
            </div>

            <!-- Text view -->
            <div v-else-if="viewMode === 'text'" class="p-6">
                <h2
                    class="text-lg font-bold leading-snug mb-3 text-primary tracking-tight"
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
                            class="text-base font-semibold uppercase tracking-widest w-8 shrink-0 text-tertiary"
                        >
                            {{ label }}
                        </dt>
                        <dd class="text-base text-secondary">{{ val }}</dd>
                    </div>
                </dl>
                <div class="my-4 h-px bg-border-subtle"></div>
                <pre
                    class="text-base leading-relaxed whitespace-pre-wrap break-words font-mono text-secondary"
                    >{{ message.body_text?.[0] || "(empty)" }}</pre
                >
            </div>

            <!-- JSON view -->
            <div v-else class="p-6">
                <pre
                    class="text-base leading-relaxed whitespace-pre overflow-x-auto font-mono text-secondary"
                    v-html="highlightedJson"
                ></pre>
            </div>
        </div>
    </div>
</template>

<style scoped>
@reference "../style.css";

:deep(.json-key) {
    @apply text-[#a78bfa];
}
:deep(.json-string) {
    color: color-mix(in srgb, var(--status-ok) 90%, var(--text-primary));
}
:deep(.json-number) {
    color: color-mix(in srgb, var(--status-warn) 90%, var(--text-primary));
}
:deep(.json-bool) {
    @apply text-[#38bdf8];
}
:deep(.json-null) {
    @apply text-tertiary;
}
</style>
