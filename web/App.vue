<script setup>
import { ref, computed } from "vue";
import { useDark, useToggle } from "@vueuse/core";
import { useAppStore } from "./store/app.js";
import AppButton from "./components/AppButton.vue";
import ConnectionModal from "./components/ConnectionModal.vue";

const {
    connections,
    activeConnectionId,
    activeMessage,
    activeMessages,
    selectConnection,
    selectMessage,
    formatAddress,
    formatDate,
    stateLabel,
} = useAppStore();

const isDark = useDark();
const toggleDark = useToggle(isDark);

const viewMode = ref("text");

function stateTagClass(state) {
    const s = stateLabel(state);
    if (s === "running") return "tag tag-ok";
    if (s === "starting" || s === "stopping") return "tag tag-warn";
    if (s === "failed") return "tag tag-error";
    return "tag tag-muted";
}

function highlight(json) {
    const str = JSON.stringify(json, null, 2);
    return str.replace(
        /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
        (match) => {
            if (/^"/.test(match))
                return /:$/.test(match)
                    ? `<span class="json-key">${match}</span>`
                    : `<span class="json-string">${match}</span>`;
            if (/true|false/.test(match))
                return `<span class="json-bool">${match}</span>`;
            if (/null/.test(match))
                return `<span class="json-null">${match}</span>`;
            return `<span class="json-number">${match}</span>`;
        },
    );
}

const highlightedJson = computed(() =>
    activeMessage.value ? highlight(activeMessage.value) : "",
);

const messageCount = computed(() => activeMessages.value.length);

// Modal
const modalOpen = ref(false);
const modalMode = ref("create");
const modalConnection = ref(null);

function openCreateModal() {
    modalMode.value = "create";
    modalConnection.value = null;
    modalOpen.value = true;
}
function openEditModal(conn) {
    modalMode.value = "edit";
    modalConnection.value = conn;
    modalOpen.value = true;
}
function closeModal() {
    modalOpen.value = false;
    modalConnection.value = null;
}
</script>

<template>
    <div
        class="flex flex-col h-screen overflow-hidden"
        style="
            background: var(--surface-base);
            color: var(--text-primary);
            font-family: var(--font-mono);
            font-size: 12px;
        "
    >
        <!-- Titlebar -->
        <header
            class="flex items-center justify-between px-4 h-10 shrink-0"
            style="
                background: var(--surface-raised);
                border-bottom: 1px solid var(--border-subtle);
            "
        >
            <div class="flex items-center gap-2">
                <span class="text-sm leading-none" style="color: var(--accent)"
                    >◈</span
                >
                <span class="text-[13px] font-bold tracking-wide">hark</span>
                <span
                    class="text-[11px] tracking-wide"
                    style="color: var(--text-tertiary)"
                    >imap to http proxy</span
                >
            </div>
            <div class="flex items-center gap-3">
                <span
                    class="flex items-center gap-1.5 text-[11px]"
                    style="color: var(--text-secondary)"
                >
                    <span
                        class="w-1.5 h-1.5 rounded-full shrink-0 transition-all"
                        :style="
                            connections.length > 0
                                ? 'background: var(--status-ok); box-shadow: 0 0 5px var(--status-ok-glow)'
                                : 'background: var(--text-tertiary)'
                        "
                    ></span>
                    {{ connections.length }}
                    {{
                        connections.length === 1 ? "connection" : "connections"
                    }}
                </span>
                <AppButton
                    variant="ghost"
                    class="!h-6 !w-6 !p-0 text-[13px]"
                    :title="isDark ? 'Switch to light' : 'Switch to dark'"
                    @click="toggleDark()"
                >
                    {{ isDark ? "○" : "●" }}
                </AppButton>
            </div>
        </header>

        <!-- 3-pane body -->
        <div class="flex flex-1 overflow-hidden">
            <!-- Pane 1 · Connections -->
            <aside
                class="flex flex-col shrink-0 overflow-hidden"
                style="
                    width: 260px;
                    border-right: 1px solid var(--border-subtle);
                "
            >
                <div
                    class="flex items-center gap-2 px-3 h-8 shrink-0 text-[10px] font-semibold uppercase tracking-widest"
                    style="
                        background: var(--surface-raised);
                        border-bottom: 1px solid var(--border-subtle);
                        color: var(--text-tertiary);
                    "
                >
                    <span>Connections</span>
                    <span
                        class="ml-auto inline-flex items-center h-4 px-1.5 rounded-full text-[10px] border"
                        style="
                            background: var(--surface-sunken);
                            border-color: var(--border-subtle);
                            color: var(--text-tertiary);
                        "
                        >{{ connections.length }}</span
                    >
                    <AppButton
                        variant="ghost"
                        class="!h-5 !w-5 !p-0 !text-base !leading-none shrink-0"
                        title="New connection"
                        @click="openCreateModal"
                        >+</AppButton
                    >
                </div>
                <div
                    class="flex-1 overflow-y-auto"
                    style="background: var(--surface-base)"
                >
                    <div
                        v-if="connections.length === 0"
                        class="flex flex-col items-center justify-center gap-1.5 h-full text-[11px] tracking-wide"
                        style="color: var(--text-tertiary)"
                    >
                        <span class="text-xl opacity-20">⌀</span>
                        <span>no connections</span>
                    </div>
                    <div
                        v-for="conn in connections"
                        :key="conn.id"
                        class="group relative px-3 py-2 cursor-pointer"
                        :style="
                            activeConnectionId === conn.id
                                ? 'border-left: 2px solid var(--accent); padding-left: 10px; background: var(--surface-overlay); border-bottom: 1px solid var(--border-subtle)'
                                : 'border-left: 2px solid transparent; border-bottom: 1px solid var(--border-subtle)'
                        "
                        @mouseenter="
                            $event.currentTarget.style.background =
                                activeConnectionId === conn.id
                                    ? 'var(--surface-overlay)'
                                    : 'var(--surface-overlay)'
                        "
                        @mouseleave="
                            $event.currentTarget.style.background =
                                activeConnectionId === conn.id
                                    ? 'var(--surface-overlay)'
                                    : 'var(--surface-base)'
                        "
                        @click="selectConnection(conn.id)"
                    >
                        <div class="flex items-center gap-1.5 min-w-0">
                            <span
                                class="text-[12px] font-semibold truncate flex-1"
                                :style="
                                    activeConnectionId === conn.id
                                        ? 'color: var(--accent)'
                                        : 'color: var(--text-primary)'
                                "
                                >{{ conn.id }}</span
                            >
                            <AppButton
                                variant="ghost"
                                class="!h-5 !w-5 !p-0 text-[11px] opacity-0 group-hover:opacity-100 shrink-0"
                                title="Edit"
                                @click.stop="openEditModal(conn)"
                                >✎</AppButton
                            >
                            <span :class="stateTagClass(conn.state)">{{
                                stateLabel(conn.state)
                            }}</span>
                        </div>
                        <div
                            class="text-[11px] mt-0.5 truncate"
                            style="color: var(--text-secondary)"
                        >
                            {{ conn.connection?.host ?? "—" }}
                        </div>
                    </div>
                </div>
            </aside>

            <!-- Pane 2 · Message list -->
            <section
                class="flex flex-col shrink-0 overflow-hidden"
                style="
                    width: 260px;
                    border-right: 1px solid var(--border-subtle);
                "
            >
                <div
                    class="flex items-center gap-2 px-3 h-8 shrink-0 text-[10px] font-semibold uppercase tracking-widest"
                    style="
                        background: var(--surface-raised);
                        border-bottom: 1px solid var(--border-subtle);
                        color: var(--text-tertiary);
                    "
                >
                    <span class="truncate">{{
                        activeConnectionId ?? "Messages"
                    }}</span>
                    <span
                        class="ml-auto inline-flex items-center h-4 px-1.5 rounded-full text-[10px] border shrink-0"
                        style="
                            background: var(--surface-sunken);
                            border-color: var(--border-subtle);
                            color: var(--text-tertiary);
                        "
                        >{{ messageCount }}</span
                    >
                </div>
                <div
                    class="flex-1 overflow-y-auto"
                    style="background: var(--surface-base)"
                >
                    <div
                        v-if="!activeConnectionId"
                        class="flex flex-col items-center justify-center gap-1.5 h-full text-[11px] tracking-wide"
                        style="color: var(--text-tertiary)"
                    >
                        <span class="text-xl opacity-20">←</span>
                        <span>select a connection</span>
                    </div>
                    <div
                        v-else-if="messageCount === 0"
                        class="flex flex-col items-center justify-center gap-1.5 h-full text-[11px] tracking-wide"
                        style="color: var(--text-tertiary)"
                    >
                        <span class="text-xl opacity-20">⌀</span>
                        <span>no messages</span>
                    </div>
                    <div
                        v-for="msg in activeMessages"
                        :key="msg.id"
                        class="px-3 py-2 cursor-pointer"
                        :style="
                            activeMessage?.id === msg.id
                                ? 'border-left: 2px solid var(--accent); padding-left: 10px; background: var(--surface-overlay); border-bottom: 1px solid var(--border-subtle)'
                                : 'border-left: 2px solid transparent; border-bottom: 1px solid var(--border-subtle)'
                        "
                        @mouseenter="
                            $event.currentTarget.style.background =
                                'var(--surface-overlay)'
                        "
                        @mouseleave="
                            $event.currentTarget.style.background =
                                activeMessage?.id === msg.id
                                    ? 'var(--surface-overlay)'
                                    : 'var(--surface-base)'
                        "
                        @click="selectMessage(msg)"
                    >
                        <div
                            class="text-[12px] font-semibold truncate"
                            :style="
                                activeMessage?.id === msg.id
                                    ? 'color: var(--accent)'
                                    : 'color: var(--text-primary)'
                            "
                        >
                            {{ formatAddress(msg.envelope?.from) }}
                        </div>
                        <div
                            class="text-[11px] truncate mt-0.5"
                            style="color: var(--text-secondary)"
                        >
                            {{ msg.subject || "(no subject)" }}
                        </div>
                        <div
                            class="text-[10px] mt-1"
                            style="color: var(--text-tertiary)"
                        >
                            {{ formatDate(msg.envelope?.date) }}
                        </div>
                    </div>
                </div>
            </section>

            <!-- Pane 3 · Viewer -->
            <main class="flex flex-col flex-1 overflow-hidden">
                <div
                    class="flex items-center gap-2 px-3 h-8 shrink-0 text-[10px] font-semibold uppercase tracking-widest"
                    style="
                        background: var(--surface-raised);
                        border-bottom: 1px solid var(--border-subtle);
                        color: var(--text-tertiary);
                    "
                >
                    <span>Message</span>
                    <div
                        v-if="activeMessage"
                        class="ml-auto flex items-center gap-1"
                    >
                        <button
                            class="btn btn-ghost !h-6 !text-[10px] font-semibold uppercase tracking-wider border"
                            :style="
                                viewMode === 'text'
                                    ? 'color: var(--accent); border-color: var(--accent-border); background: var(--accent-subtle)'
                                    : 'border-color: var(--border-subtle); color: var(--text-tertiary)'
                            "
                            @click="viewMode = 'text'"
                        >
                            Text
                        </button>
                        <button
                            class="btn btn-ghost !h-6 !text-[10px] font-semibold uppercase tracking-wider border"
                            :style="
                                viewMode === 'json'
                                    ? 'color: var(--accent); border-color: var(--accent-border); background: var(--accent-subtle)'
                                    : 'border-color: var(--border-subtle); color: var(--text-tertiary)'
                            "
                            @click="viewMode = 'json'"
                        >
                            JSON
                        </button>
                    </div>
                </div>
                <div
                    class="flex-1 overflow-y-auto"
                    style="background: var(--surface-base)"
                >
                    <div
                        v-if="!activeMessage"
                        class="flex flex-col items-center justify-center gap-1.5 h-full text-[11px] tracking-wide"
                        style="color: var(--text-tertiary)"
                    >
                        <span class="text-xl opacity-20">✉</span>
                        <span>select a message</span>
                    </div>
                    <div v-else-if="viewMode === 'text'" class="p-6">
                        <h2
                            class="text-[14px] font-bold leading-snug mb-3"
                            style="
                                color: var(--text-primary);
                                letter-spacing: -0.01em;
                            "
                        >
                            {{ activeMessage.subject || "(no subject)" }}
                        </h2>
                        <dl class="flex flex-col gap-1.5">
                            <div
                                v-for="[label, val] in [
                                    [
                                        'From',
                                        formatAddress(
                                            activeMessage.envelope?.from,
                                        ),
                                    ],
                                    [
                                        'To',
                                        formatAddress(
                                            activeMessage.envelope?.to,
                                        ),
                                    ],
                                    [
                                        'Date',
                                        formatDate(
                                            activeMessage.envelope?.date,
                                        ),
                                    ],
                                ]"
                                :key="label"
                                class="flex gap-3 items-baseline"
                            >
                                <dt
                                    class="text-[10px] font-semibold uppercase tracking-widest w-8 shrink-0"
                                    style="color: var(--text-tertiary)"
                                >
                                    {{ label }}
                                </dt>
                                <dd
                                    class="text-[12px]"
                                    style="color: var(--text-secondary)"
                                >
                                    {{ val }}
                                </dd>
                            </div>
                        </dl>
                        <div
                            class="my-4"
                            style="
                                height: 1px;
                                background: var(--border-subtle);
                            "
                        ></div>
                        <pre
                            class="text-[12px] leading-relaxed whitespace-pre-wrap break-words"
                            style="
                                font-family: var(--font-mono);
                                color: var(--text-secondary);
                            "
                            >{{
                                activeMessage.body_text?.[0] || "(empty)"
                            }}</pre
                        >
                    </div>
                    <div v-else class="p-6">
                        <pre
                            class="text-[12px] leading-relaxed whitespace-pre overflow-x-auto"
                            style="
                                font-family: var(--font-mono);
                                color: var(--text-secondary);
                            "
                            v-html="highlightedJson"
                        ></pre>
                    </div>
                </div>
            </main>
        </div>
    </div>

    <!-- Connection modal -->
    <ConnectionModal
        v-if="modalOpen"
        :mode="modalMode"
        :connection="modalConnection"
        @close="closeModal"
    />
</template>

<style scoped>
.tag {
    display: inline-flex;
    align-items: center;
    padding: 0 5px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    height: 16px;
    flex-shrink: 0;
}
.tag-ok {
    background: color-mix(in srgb, var(--status-ok) 12%, transparent);
    color: var(--status-ok);
}
.tag-warn {
    background: color-mix(in srgb, var(--status-warn) 12%, transparent);
    color: var(--status-warn);
}
.tag-error {
    background: color-mix(in srgb, var(--status-error) 12%, transparent);
    color: var(--status-error);
}
.tag-muted {
    background: var(--surface-sunken);
    color: var(--text-tertiary);
}
:deep(.json-key) {
    color: #a78bfa;
}
:deep(.json-string) {
    color: color-mix(in srgb, var(--status-ok) 90%, var(--text-primary));
}
:deep(.json-number) {
    color: color-mix(in srgb, var(--status-warn) 90%, var(--text-primary));
}
:deep(.json-bool) {
    color: #38bdf8;
}
:deep(.json-null) {
    color: var(--text-tertiary);
}
</style>
