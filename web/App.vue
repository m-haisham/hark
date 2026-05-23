<script setup>
import { ref, computed } from "vue";
import { useDark, useToggle } from "@vueuse/core";
import { useAppStore } from "./store/app.js";
import AppButton from "./components/AppButton.vue";
import ConnectionModal from "./components/ConnectionModal.vue";
import MessageViewer from "./components/MessageViewer.vue";
import StateTag from "./components/StateTag.vue";

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
        class="flex flex-col h-screen overflow-hidden bg-surface-base text-primary font-mono text-[15px]"
    >
        <!-- Titlebar -->
        <header
            class="flex items-center justify-between px-4 h-10 shrink-0 bg-surface-raised border-b border-border-subtle"
        >
            <div class="flex items-center gap-2">
                <span class="text-lg leading-none text-accent">◈</span>
                <span class="text-lg font-bold tracking-wide">hark</span>
                <span class="text-base tracking-wide text-tertiary"
                    >imap to http proxy</span
                >
            </div>
            <div class="flex items-center gap-3">
                <span
                    class="flex items-center gap-1.5 text-base text-secondary"
                >
                    <span
                        class="w-1.5 h-1.5 rounded-full shrink-0 transition-all"
                        :class="
                            connections.length > 0
                                ? 'bg-status-ok shadow-[0_0_5px_var(--status-ok-glow)]'
                                : 'bg-tertiary'
                        "
                    ></span>
                    {{ connections.length }}
                    {{
                        connections.length === 1 ? "connection" : "connections"
                    }}
                </span>
                <AppButton
                    variant="ghost"
                    class="!h-6 !w-6 !p-0 text-base"
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
                class="flex flex-col shrink-0 overflow-hidden w-[260px] border-r border-border-subtle"
            >
                <!-- Pane header -->
                <div
                    class="flex items-center gap-2 px-3 h-8 shrink-0 bg-surface-raised border-b border-border-subtle text-base font-semibold uppercase tracking-widest text-tertiary"
                >
                    <span>Connections</span>
                    <span
                        class="ml-auto inline-flex items-center h-4 px-1.5 rounded-full text-base border bg-surface-sunken border-border-subtle text-tertiary"
                    >
                        {{ connections.length }}
                    </span>
                    <AppButton
                        variant="ghost"
                        class="!h-5 !w-5 !p-0 !text-base !leading-none shrink-0"
                        title="New connection"
                        @click="openCreateModal"
                        >+</AppButton
                    >
                </div>
                <!-- List -->
                <div class="flex-1 overflow-y-auto bg-surface-base">
                    <div
                        v-if="connections.length === 0"
                        class="flex flex-col items-center justify-center gap-1.5 h-full text-base tracking-wide text-tertiary"
                    >
                        <span class="text-3xl opacity-20">⌀</span>
                        <span>no connections</span>
                    </div>
                    <div
                        v-for="conn in connections"
                        :key="conn.id"
                        class="group relative px-3 py-2 cursor-pointer border-b border-border-subtle transition-colors"
                        :class="
                            activeConnectionId === conn.id
                                ? 'border-l-2 border-l-accent pl-[10px] bg-surface-overlay'
                                : 'border-l-2 border-l-transparent hover:bg-surface-overlay'
                        "
                        @click="selectConnection(conn.id)"
                    >
                        <div class="flex items-center gap-1.5 min-w-0">
                            <span
                                class="text-base font-semibold truncate flex-1"
                                :class="
                                    activeConnectionId === conn.id
                                        ? 'text-accent'
                                        : 'text-primary'
                                "
                                >{{ conn.id }}</span
                            >
                            <AppButton
                                variant="ghost"
                                class="!h-5 !w-5 !p-0 text-base opacity-0 group-hover:opacity-100 shrink-0"
                                title="Edit"
                                @click.stop="openEditModal(conn)"
                                >✎</AppButton
                            >
                            <StateTag
                                :state="conn.state"
                                :label="stateLabel(conn.state)"
                            />
                        </div>
                        <div class="text-base mt-0.5 truncate text-secondary">
                            {{ conn.connection?.host ?? "—" }}
                        </div>
                    </div>
                </div>
            </aside>

            <!-- Pane 2 · Message list -->
            <section
                class="flex flex-col shrink-0 overflow-hidden w-[260px] border-r border-border-subtle"
            >
                <!-- Pane header -->
                <div
                    class="flex items-center gap-2 px-3 h-8 shrink-0 bg-surface-raised border-b border-border-subtle text-base font-semibold uppercase tracking-widest text-tertiary"
                >
                    <span class="truncate">{{
                        activeConnectionId ?? "Messages"
                    }}</span>
                    <span
                        class="ml-auto inline-flex items-center h-4 px-1.5 rounded-full text-base border shrink-0 bg-surface-sunken border-border-subtle text-tertiary"
                    >
                        {{ messageCount }}
                    </span>
                </div>
                <!-- List -->
                <div class="flex-1 overflow-y-auto bg-surface-base">
                    <div
                        v-if="!activeConnectionId"
                        class="flex flex-col items-center justify-center gap-1.5 h-full text-base tracking-wide text-tertiary"
                    >
                        <span class="text-3xl opacity-20">←</span>
                        <span>select a connection</span>
                    </div>
                    <div
                        v-else-if="messageCount === 0"
                        class="flex flex-col items-center justify-center gap-1.5 h-full text-base tracking-wide text-tertiary"
                    >
                        <span class="text-3xl opacity-20">⌀</span>
                        <span>no messages</span>
                    </div>
                    <div
                        v-for="msg in activeMessages"
                        :key="msg.id"
                        class="px-3 py-2 cursor-pointer border-b border-border-subtle transition-colors"
                        :class="
                            activeMessage?.id === msg.id
                                ? 'border-l-2 border-l-accent pl-[10px] bg-surface-overlay'
                                : 'border-l-2 border-l-transparent hover:bg-surface-overlay'
                        "
                        @click="selectMessage(msg)"
                    >
                        <div
                            class="text-base font-semibold truncate"
                            :class="
                                activeMessage?.id === msg.id
                                    ? 'text-accent'
                                    : 'text-primary'
                            "
                        >
                            {{ formatAddress(msg.envelope?.from) }}
                        </div>
                        <div class="text-base truncate mt-0.5 text-secondary">
                            {{ msg.subject || "(no subject)" }}
                        </div>
                        <div class="text-base mt-1 text-tertiary">
                            {{ formatDate(msg.envelope?.date) }}
                        </div>
                    </div>
                </div>
            </section>

            <!-- Pane 3 · Viewer -->
            <MessageViewer
                :message="activeMessage"
                :format-address="formatAddress"
                :format-date="formatDate"
            />
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
