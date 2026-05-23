<script setup>
defineProps({
    label: String,
    hint: String,
    required: Boolean,
});
</script>

<template>
    <div class="flex flex-col gap-1">
        <label
            v-if="label"
            class="text-[10px] font-semibold uppercase tracking-widest text-tertiary"
        >
            {{ label
            }}<span v-if="required" class="text-status-error ml-0.5">*</span>
        </label>
        <slot />
        <span v-if="hint" class="text-[10px] text-tertiary">{{ hint }}</span>
    </div>
</template>
