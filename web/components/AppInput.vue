<script setup>
const model = defineModel();
const props = defineProps({
    type: { type: String, default: "text" },
    placeholder: String,
    autocomplete: { type: String, default: "off" },
});

function onInput(e) {
    const v = e.target.value;
    model.value = props.type === "number" ? (v === "" ? "" : Number(v)) : v;
}
</script>

<template>
    <input
        class="app-input"
        :type="props.type"
        :value="model"
        :placeholder="props.placeholder"
        :autocomplete="props.autocomplete"
        spellcheck="false"
        @input="onInput"
    />
</template>

<style>
.app-input {
    width: 100%;
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 12px;
    outline: none;
    transition:
        border-color 80ms,
        box-shadow 80ms;
    background: var(--surface-base);
    border: 1px solid var(--border-default);
    color: var(--text-primary);
    padding: 5px 8px;
}
.app-input::placeholder {
    color: var(--text-tertiary);
}
.app-input:focus {
    border-color: var(--accent);
    box-shadow: 0 0 0 2px var(--accent-subtle);
}
</style>
