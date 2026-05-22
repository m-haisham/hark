<script setup>
import { ref, reactive, watch } from "vue";
import { useAppStore } from "../store/app.js";
import AppButton from "./AppButton.vue";
import AppField from "./AppField.vue";
import AppInput from "./AppInput.vue";
import AppSelect from "./AppSelect.vue";
import AppCheckbox from "./AppCheckbox.vue";

const props = defineProps({
    mode: { type: String, required: true }, // 'create' | 'edit'
    connection: { type: Object, default: null },
});

const emit = defineEmits(["close"]);

const { createConnection, updateConnection, deleteConnection, testConnection } =
    useAppStore();

const form = reactive({
    name: "",
    host: "",
    port: 993,
    username: "",
    password: "",
    tls: true,
    mailbox: "INBOX",
    flavour: "",
});

watch(
    () => props.connection,
    (conn) => {
        if (!conn) return;
        form.name = conn.id ?? "";
        form.host = conn.connection?.host ?? "";
        form.port = conn.connection?.port ?? 993;
        form.username = conn.connection?.username ?? "";
        form.password = "";
        form.tls = conn.connection?.tls ?? true;
        form.mailbox = conn.connection?.mailbox ?? "INBOX";
        form.flavour = conn.connection?.flavour ?? "";
    },
    { immediate: true },
);

const loading = ref(false);
const error = ref("");
const testStatus = ref(""); // '' | 'testing' | 'ok' | 'error'
const testError = ref("");
const confirmDelete = ref(false);

const flavourOptions = [
    { value: "", label: "none" },
    { value: "gmail", label: "gmail" },
    { value: "outlook", label: "outlook" },
];

function buildPayload() {
    const p = {
        host: form.host,
        port: Number(form.port),
        username: form.username,
        auth: "password",
        tls: form.tls,
        mailbox: form.mailbox,
    };
    if (form.password) p.password = form.password;
    if (form.flavour) p.flavour = form.flavour;
    return p;
}

async function handleTest() {
    testStatus.value = "testing";
    testError.value = "";
    error.value = "";
    try {
        const p = buildPayload();
        if (props.mode === "create") p.name = form.name;
        await testConnection(p);
        testStatus.value = "ok";
    } catch (e) {
        testStatus.value = "error";
        testError.value = e.message;
    }
}

async function handleSave() {
    error.value = "";
    loading.value = true;
    try {
        if (props.mode === "create") {
            if (!form.password) throw new Error("Password is required");
            await createConnection({ name: form.name, ...buildPayload() });
        } else {
            await updateConnection(props.connection.id, buildPayload());
        }
        emit("close");
    } catch (e) {
        error.value = e.message;
    } finally {
        loading.value = false;
    }
}

async function handleDelete() {
    error.value = "";
    loading.value = true;
    try {
        await deleteConnection(props.connection.id);
        emit("close");
    } catch (e) {
        error.value = e.message;
        loading.value = false;
    }
}
</script>

<template>
    <!-- Overlay -->
    <div
        class="fixed inset-0 z-50 flex items-center justify-center"
        style="background: rgba(0, 0, 0, 0.6)"
        @click.self="emit('close')"
    >
        <!-- Panel -->
        <div
            class="flex flex-col w-[440px] max-w-[calc(100vw-2rem)] rounded-md overflow-hidden"
            style="
                background: var(--surface-raised);
                border: 1px solid var(--border-default);
                box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
                color: var(--text-primary);
                font-family: var(--font-mono);
                font-size: 12px;
            "
            role="dialog"
            aria-modal="true"
        >
            <!-- Header -->
            <div
                class="flex items-center justify-between px-4 h-10 shrink-0"
                style="
                    background: var(--surface-overlay);
                    border-bottom: 1px solid var(--border-subtle);
                "
            >
                <div class="flex items-center gap-2">
                    <span
                        class="text-[11px] font-bold uppercase tracking-widest"
                        style="color: var(--text-secondary)"
                    >
                        {{
                            mode === "create"
                                ? "New Connection"
                                : connection?.id
                        }}
                    </span>
                    <span
                        v-if="mode === 'edit'"
                        class="text-[9px] font-semibold uppercase tracking-wider px-1.5 py-px rounded"
                        style="
                            color: var(--text-tertiary);
                            background: var(--surface-sunken);
                            border: 1px solid var(--border-subtle);
                        "
                        >edit</span
                    >
                </div>
                <AppButton
                    variant="ghost"
                    class="!h-6 !w-6 !p-0 text-xs"
                    @click="emit('close')"
                    >✕</AppButton
                >
            </div>

            <!-- Body -->
            <div class="flex flex-col gap-3 p-4 overflow-y-auto">
                <!-- Name (create only) -->
                <AppField
                    v-if="mode === 'create'"
                    label="Name"
                    :required="true"
                    hint="1–20 chars · must start with a letter · a-z 0-9 _"
                >
                    <AppInput v-model="form.name" placeholder="my_connection" />
                </AppField>

                <!-- Host -->
                <AppField label="Host" :required="true">
                    <AppInput
                        v-model="form.host"
                        placeholder="imap.example.com"
                    />
                </AppField>

                <!-- Port + TLS -->
                <div class="flex gap-3 items-end">
                    <AppField label="Port" class="w-28 shrink-0">
                        <AppInput
                            v-model="form.port"
                            type="number"
                            placeholder="993"
                        />
                    </AppField>
                    <div class="pb-1.5">
                        <AppCheckbox v-model="form.tls" label="TLS" />
                    </div>
                </div>

                <!-- Username -->
                <AppField label="Username" :required="true">
                    <AppInput
                        v-model="form.username"
                        placeholder="user@example.com"
                    />
                </AppField>

                <!-- Password -->
                <AppField label="Password" :required="mode === 'create'">
                    <AppInput
                        v-model="form.password"
                        type="password"
                        :placeholder="
                            mode === 'edit' ? 'leave blank to keep current' : ''
                        "
                        autocomplete="new-password"
                    />
                </AppField>

                <!-- Mailbox + Flavour -->
                <div class="flex gap-3">
                    <AppField label="Mailbox" class="flex-1">
                        <AppInput v-model="form.mailbox" placeholder="INBOX" />
                    </AppField>
                    <AppField label="Flavour" class="w-28 shrink-0">
                        <AppSelect
                            v-model="form.flavour"
                            :options="flavourOptions"
                        />
                    </AppField>
                </div>

                <!-- Test result -->
                <div v-if="testStatus === 'ok'" class="feedback feedback-ok">
                    <span class="font-bold">✓</span> connected
                </div>
                <div
                    v-else-if="testStatus === 'error'"
                    class="feedback feedback-error"
                >
                    <span class="font-bold">✗</span>
                    {{ testError || "connection failed" }}
                </div>

                <!-- Save error -->
                <div v-if="error" class="feedback feedback-error">
                    <span class="font-bold">✗</span> {{ error }}
                </div>
            </div>

            <!-- Footer -->
            <div
                class="flex items-center justify-between gap-2 px-4 py-2.5 shrink-0"
                style="
                    background: var(--surface-overlay);
                    border-top: 1px solid var(--border-subtle);
                "
            >
                <!-- Left: delete -->
                <div class="flex items-center gap-2">
                    <template v-if="mode === 'edit' && !confirmDelete">
                        <AppButton
                            variant="danger"
                            :disabled="loading"
                            @click="confirmDelete = true"
                        >
                            Delete
                        </AppButton>
                    </template>
                    <template v-else-if="mode === 'edit' && confirmDelete">
                        <span
                            class="text-[11px] font-semibold"
                            style="color: var(--status-error)"
                            >Confirm?</span
                        >
                        <AppButton
                            variant="danger"
                            :disabled="loading"
                            @click="handleDelete"
                            >Yes</AppButton
                        >
                        <AppButton
                            variant="secondary"
                            :disabled="loading"
                            @click="confirmDelete = false"
                            >No</AppButton
                        >
                    </template>
                </div>

                <!-- Right: test / cancel / save -->
                <div class="flex items-center gap-2 ml-auto">
                    <AppButton
                        variant="secondary"
                        :disabled="loading || testStatus === 'testing'"
                        @click="handleTest"
                    >
                        {{ testStatus === "testing" ? "Testing…" : "Test" }}
                    </AppButton>
                    <AppButton
                        variant="secondary"
                        :disabled="loading"
                        @click="emit('close')"
                        >Cancel</AppButton
                    >
                    <AppButton
                        variant="primary"
                        :disabled="loading"
                        @click="handleSave"
                    >
                        {{ loading ? "Saving…" : "Save" }}
                    </AppButton>
                </div>
            </div>
        </div>
    </div>
</template>

<style>
.feedback {
    display: flex;
    align-items: flex-start;
    gap: 6px;
    font-size: 11px;
    padding: 6px 8px;
    border-radius: 4px;
    line-height: 1.4;
}
.feedback-ok {
    color: var(--status-ok);
    background: color-mix(in srgb, var(--status-ok) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--status-ok) 25%, transparent);
}
.feedback-error {
    color: var(--status-error);
    background: color-mix(in srgb, var(--status-error) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--status-error) 25%, transparent);
}
</style>
