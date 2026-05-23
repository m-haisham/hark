<script setup>
defineProps({
    variant: { type: String, default: "secondary" }, // primary | secondary | ghost | danger
    disabled: Boolean,
    type: { type: String, default: "button" },
});
</script>

<template>
    <button
        :type="type"
        class="btn"
        :class="`btn-${variant}`"
        :disabled="disabled"
    >
        <slot />
    </button>
</template>

<style>
@reference "../style.css";

.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 28px;
    padding: 0 12px;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.03em;
    border: 1px solid transparent;
    cursor: pointer;
    transition:
        filter 80ms,
        background 80ms,
        color 80ms,
        border-color 80ms;
    white-space: nowrap;
    flex-shrink: 0;
}
.btn:disabled {
    opacity: 0.4;
    cursor: not-allowed;
}

.btn-primary {
    @apply bg-accent border-accent text-white;
}
.btn-primary:hover:not(:disabled) {
    filter: brightness(1.12);
}

.btn-secondary {
    @apply bg-surface-overlay border-border-default text-secondary;
}
.btn-secondary:hover:not(:disabled) {
    @apply bg-surface-sunken text-primary;
}

.btn-ghost {
    @apply bg-transparent border-transparent text-tertiary;
}
.btn-ghost:hover:not(:disabled) {
    @apply bg-surface-overlay text-secondary;
}

.btn-danger {
    @apply bg-status-error/10 border-status-error/25 text-status-error;
}
.btn-danger:hover:not(:disabled) {
    @apply bg-status-error/20;
}
</style>
