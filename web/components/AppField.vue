<script setup>
defineProps({
    label: String,
    hint: String,
    required: Boolean,
});
</script>

<template>
    <div class="app-field">
        <label v-if="label" class="app-field-label">
            {{ label }}<span v-if="required" class="app-field-required">*</span>
        </label>
        <slot />
        <span v-if="hint" class="app-field-hint">{{ hint }}</span>
    </div>
</template>

<style>
.app-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
}
.app-field-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-tertiary);
}
.app-field-required {
    color: var(--status-error);
    margin-left: 2px;
}
.app-field-hint {
    font-size: 10px;
    color: var(--text-tertiary);
}
</style>
