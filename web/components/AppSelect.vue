<script setup>
const model = defineModel();
const props = defineProps({
    options: { type: Array, required: true }, // [{ value, label }]
});
</script>

<template>
    <select class="app-select" v-model="model">
        <option
            v-for="opt in props.options"
            :key="opt.value"
            :value="opt.value"
        >
            {{ opt.label }}
        </option>
    </select>
</template>

<style>
.app-select {
    width: 100%;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 12px;
    outline: none;
    cursor: pointer;
    appearance: none;
    transition:
        border-color 80ms,
        box-shadow 80ms;
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    color: var(--text-primary);
    padding: 5px 28px 5px 8px;
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='%239898a8'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 8px center;
}
.app-select:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-subtle);
}
</style>
