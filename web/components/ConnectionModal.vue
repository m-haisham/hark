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
        p.name = form.name;
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
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
        @click.self="emit('close')"
    >
        <!-- Panel -->
        <div
            class="flex flex-col w-[440px] max-w-[calc(100vw-2rem)] rounded-md overflow-hidden bg-surface-raised border border-border-default shadow-2xl text-primary font-mono text-xs"
            role="dialog"
            aria-modal="true"
        >
            <!-- Header -->
            <div
                class="flex items-center justify-between px-4 h-10 shrink-0 bg-surface-overlay border-b border-border-subtle"
            >
                <div class="flex items-center gap-2">
                    <span
                        class="text-xs font-bold uppercase tracking-widest text-secondary"
                    >
                        {{
                            mode === "create"
                                ? "New Connection"
                                : connection?.id
                        }}
                    </span>
                    <span
                        v-if="mode === 'edit'"
                        class="text-[9px] font-semibold uppercase tracking-wider px-1.5 py-px rounded text-tertiary bg-surface-sunken border border-border-subtle"
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
                <AppField
                    v-if="mode === 'create'"
                    label="Name"
                    :required="true"
                    hint="1–20 chars · must start with a letter · a-z 0-9 _"
                >
                    <AppInput v-model="form.name" placeholder="my_connection" />
                </AppField>

                <AppField label="Host" :required="true">
                    <AppInput
                        v-model="form.host"
                        placeholder="imap.example.com"
                    />
                </AppField>

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

                <AppField label="Username" :required="true">
                    <AppInput
                        v-model="form.username"
                        placeholder="user@example.com"
                    />
                </AppField>

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
                <div
                    v-if="testStatus === 'ok'"
                    class="flex items-start gap-1.5 text-xs px-2 py-1.5 rounded border text-status-ok bg-status-ok/10 border-status-ok/25"
                >
                    <span class="font-bold">✓</span> connected
                </div>
                <div
                    v-else-if="testStatus === 'error'"
                    class="flex items-start gap-1.5 text-xs px-2 py-1.5 rounded border text-status-error bg-status-error/10 border-status-error/25"
                >
                    <span class="font-bold">✗</span>
                    {{ testError || "connection failed" }}
                </div>

                <!-- Save error -->
                <div
                    v-if="error"
                    class="flex items-start gap-1.5 text-xs px-2 py-1.5 rounded border text-status-error bg-status-error/10 border-status-error/25"
                >
                    <span class="font-bold">✗</span> {{ error }}
                </div>
            </div>

            <!-- Footer -->
            <div
                class="flex items-center justify-between gap-2 px-4 py-2.5 shrink-0 bg-surface-overlay border-t border-border-subtle"
            >
                <!-- Left: delete -->
                <div class="flex items-center gap-2">
                    <template v-if="mode === 'edit' && !confirmDelete">
                        <AppButton
                            variant="danger"
                            :disabled="loading"
                            @click="confirmDelete = true"
                            >Delete</AppButton
                        >
                    </template>
                    <template v-else-if="mode === 'edit' && confirmDelete">
                        <span class="text-xs font-semibold text-status-error"
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
