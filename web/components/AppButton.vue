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
    background: var(--accent);
    border-color: var(--accent);
    color: #fff;
}
.btn-primary:hover:not(:disabled) {
    filter: brightness(1.12);
}

.btn-secondary {
    background: var(--surface-overlay);
    border-color: var(--border-default);
    color: var(--text-secondary);
}
.btn-secondary:hover:not(:disabled) {
    background: var(--surface-sunken);
    color: var(--text-primary);
}

.btn-ghost {
    background: transparent;
    border-color: transparent;
    color: var(--text-tertiary);
}
.btn-ghost:hover:not(:disabled) {
    background: var(--surface-overlay);
    color: var(--text-secondary);
}

.btn-danger {
    background: color-mix(in srgb, var(--status-error) 12%, transparent);
    border-color: color-mix(in srgb, var(--status-error) 28%, transparent);
    color: var(--status-error);
}
.btn-danger:hover:not(:disabled) {
    background: color-mix(in srgb, var(--status-error) 20%, transparent);
}
</style>
