<script setup>
import { computed } from "vue";

const props = defineProps({
    state: { type: String, required: true },
    label: { type: String, required: true },
});

const classes = computed(() => {
    switch (props.label) {
        case "running":
            return "bg-status-ok/10 text-status-ok";
        case "starting":
        case "stopping":
            return "bg-status-warn/10 text-status-warn";
        case "failed":
            return "bg-status-error/10 text-status-error";
        default:
            return "bg-surface-sunken text-tertiary";
    }
});
</script>

<template>
    <span
        :class="[
            'inline-flex items-center px-1.5 rounded h-4 shrink-0',
            'text-[10px] font-semibold tracking-wide uppercase',
            classes,
        ]"
    >
        {{ label }}
    </span>
</template>
