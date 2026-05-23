<script setup>
const model = defineModel();
const props = defineProps({
    label: { type: String, required: true },
});
</script>

<template>
    <label
        class="inline-flex items-center gap-2 cursor-pointer select-none text-xs text-secondary"
    >
        <input
            type="checkbox"
            class="w-3.5 h-3.5 cursor-pointer rounded shrink-0 accent-accent"
            v-model="model"
        />
        {{ props.label }}
    </label>
</template>
