<script setup>
const model = defineModel();
const props = defineProps({
    options: { type: Array, required: true }, // [{ value, label }]
});
</script>

<template>
    <select
        class="app-select w-full rounded font-mono text-xs px-2 py-1.5 outline-none cursor-pointer appearance-none transition bg-surface-base border border-border-default text-primary focus:border-accent focus:ring-2 focus:ring-accent-subtle"
        v-model="model"
    >
        <option
            v-for="opt in props.options"
            :key="opt.value"
            :value="opt.value"
        >
            {{ opt.label }}
        </option>
    </select>
</template>

<style scoped>
.app-select {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6'%3E%3Cpath d='M0 0l5 6 5-6z' fill='%239898a8'/%3E%3C/svg%3E");
    background-repeat: no-repeat;
    background-position: right 8px center;
    padding-right: 28px;
}
</style>
