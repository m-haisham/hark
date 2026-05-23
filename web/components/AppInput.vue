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
        class="w-full rounded font-mono text-xs px-2 py-1.5 outline-none transition bg-surface-base border border-border-default text-primary placeholder:text-tertiary focus:border-accent focus:ring-2 focus:ring-accent-subtle"
        :type="props.type"
        :value="model"
        :placeholder="props.placeholder"
        :autocomplete="props.autocomplete"
        spellcheck="false"
        @input="onInput"
    />
</template>
