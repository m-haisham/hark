<script setup>
import { useAppStore } from "./store/app.js";

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

const currentYear = new Date().getFullYear();
</script>

<template>
    <div class="h-screen flex flex-col text-sm bg-default">
        <!-- Header -->
        <header
            class="flex items-center justify-between px-4 h-11 shrink-0 border-b border-subtle"
        >
            <span class="font-semibold">Hark</span>
            <span class="text-xs text-faint">
                {{ connections.length }}
                {{ connections.length === 1 ? "connection" : "connections" }}
            </span>
        </header>

        <!-- 3-pane layout -->
        <div
            class="flex flex-1 overflow-hidden divide-x divide-gray-200/70 dark:divide-gray-700/50"
        >
            <!-- Pane 1: Connections -->
            <div class="w-72 flex flex-col shrink-0 overflow-hidden">
                <div
                    class="px-3 py-2 text-xs font-semibold text-faint border-b shrink-0"
                >
                    Connections
                </div>
                <div class="flex-1 overflow-y-auto">
                    <p
                        v-if="connections.length === 0"
                        class="p-6 text-center text-xs text-faint"
                    >
                        No connections
                    </p>
                    <div
                        v-for="conn in connections"
                        :key="conn.id"
                        class="px-3 py-2.5 cursor-pointer border-b border-subtle hover:bg-indigo-50 dark:hover:bg-gray-800"
                        :class="
                            activeConnectionId === conn.id ? 'item-active' : ''
                        "
                        @click="selectConnection(conn.id)"
                    >
                        <span class="font-semibold truncate block">{{
                            conn.id
                        }}</span>
                        <span
                            class="flex items-center gap-1 text-xs text-faint mt-0.5"
                        >
                            <span
                                class="inline-block w-1.5 h-1.5 rounded-full shrink-0"
                                :class="{
                                    'bg-green-500':
                                        stateLabel(conn.state) === 'running',
                                    'bg-amber-400': [
                                        'starting',
                                        'stopping',
                                    ].includes(stateLabel(conn.state)),
                                    'bg-red-500':
                                        stateLabel(conn.state) === 'failed',
                                    'bg-gray-400': [
                                        'stopped',
                                        'unknown',
                                    ].includes(stateLabel(conn.state)),
                                }"
                            ></span>
                            <span>{{ stateLabel(conn.state) }}</span>
                            <span class="opacity-40">·</span>
                            <span class="truncate">{{
                                conn.connection.host
                            }}</span>
                        </span>
                    </div>
                </div>
            </div>

            <!-- Pane 2: Message list -->
            <div class="w-72 flex flex-col shrink-0 overflow-hidden">
                <div
                    class="px-3 py-2 text-xs font-semibold text-faint border-b border-subtle shrink-0"
                >
                    {{ activeConnectionId || "Messages" }}
                </div>
                <div class="flex-1 overflow-y-auto">
                    <p
                        v-if="!activeConnectionId"
                        class="p-6 text-center text-xs text-faint"
                    >
                        Select a connection
                    </p>
                    <p
                        v-else-if="activeMessages.length === 0"
                        class="p-6 text-center text-xs text-faint"
                    >
                        No messages
                    </p>
                    <div
                        v-for="msg in activeMessages"
                        :key="msg.id"
                        class="px-3 py-2.5 cursor-pointer border-b border-subtle hover:bg-indigo-50 dark:hover:bg-gray-800"
                        :class="
                            activeMessage?.id === msg.id ? 'item-active' : ''
                        "
                        @click="selectMessage(msg)"
                    >
                        <div class="font-semibold truncate">
                            {{ formatAddress(msg.envelope.from) }}
                        </div>
                        <div class="text-xs truncate mt-0.5">
                            {{ msg.subject || "(no subject)" }}
                        </div>
                        <div class="text-xs mt-0.5 opacity-50">
                            {{ formatDate(msg.envelope.date) }}
                        </div>
                    </div>
                </div>
            </div>

            <!-- Pane 3: Message viewer -->
            <div class="flex flex-col flex-1 overflow-hidden">
                <div
                    class="px-3 py-2 text-xs font-semibold text-faint border-b shrink-0"
                >
                    Message
                </div>
                <div class="flex-1 overflow-y-auto">
                    <div
                        v-if="!activeMessage"
                        class="h-full flex items-center justify-center text-xs text-faint"
                    >
                        Select a message to read
                    </div>
                    <div v-else class="p-6">
                        <div class="pb-4 mb-4 border-b border-subtle">
                            <h2 class="text-base font-bold mb-2">
                                {{ activeMessage.subject || "(no subject)" }}
                            </h2>
                            <dl class="text-xs text-muted space-y-0.5">
                                <div class="flex gap-1">
                                    <dt class="font-semibold w-10 shrink-0">
                                        From
                                    </dt>
                                    <dd>
                                        {{
                                            formatAddress(
                                                activeMessage.envelope.from,
                                            )
                                        }}
                                    </dd>
                                </div>
                                <div class="flex gap-1">
                                    <dt class="font-semibold w-10 shrink-0">
                                        To
                                    </dt>
                                    <dd>
                                        {{
                                            formatAddress(
                                                activeMessage.envelope.to,
                                            )
                                        }}
                                    </dd>
                                </div>
                                <div class="flex gap-1">
                                    <dt class="font-semibold w-10 shrink-0">
                                        Date
                                    </dt>
                                    <dd>
                                        {{
                                            formatDate(
                                                activeMessage.envelope.date,
                                            )
                                        }}
                                    </dd>
                                </div>
                            </dl>
                        </div>
                        <p
                            class="text-sm leading-relaxed whitespace-pre-wrap text-muted"
                        >
                            {{ activeMessage.body_text?.[0] || "(empty)" }}
                        </p>
                    </div>
                </div>
            </div>
        </div>

        <!-- Footer -->
        <footer
            class="flex items-center justify-between px-4 h-7 bg-default text-xs text-faint shrink-0 border-t"
        >
            <span>Hark — IMAP client</span>
            <span>{{ currentYear }}</span>
        </footer>
    </div>
</template>
