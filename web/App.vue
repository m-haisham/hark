<script setup>
import { ref, computed } from "vue";
import { useDark, useToggle } from "@vueuse/core";
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
</script>

<template>
    <div class="app-shell">
        <!-- Titlebar -->
        <header class="titlebar">
            <div class="flex items-center gap-2">
                <span class="titlebar-logo">◈</span>
                <span class="titlebar-name">hark</span>
                <span class="titlebar-sub">imap inspector</span>
            </div>
            <div class="flex items-center gap-3">
                <span class="stat-pill">
                    <span
                        class="stat-dot"
                        :class="
                            connections.length > 0
                                ? 'stat-dot--ok'
                                : 'stat-dot--off'
                        "
                    ></span>
                    {{ connections.length }}
                    {{
                        connections.length === 1 ? "connection" : "connections"
                    }}
                </span>
                <button
                    class="theme-btn"
                    @click="toggleDark()"
                    :title="isDark ? 'Switch to light' : 'Switch to dark'"
                >
                    {{ isDark ? "○" : "●" }}
                </button>
            </div>
        </header>

        <!-- 3-pane body -->
        <div class="body-grid">
            <!-- Pane 1 · Connections -->
            <aside class="pane pane--border">
                <div class="pane-header">
                    <span>Connections</span>
                    <span class="count-badge">{{ connections.length }}</span>
                </div>
                <div class="pane-scroll">
                    <div v-if="connections.length === 0" class="empty-state">
                        <span class="empty-icon">⌀</span>
                        <span>no connections</span>
                    </div>
                    <div
                        v-for="conn in connections"
                        :key="conn.id"
                        class="list-row"
                        :class="{
                            'list-row--active': activeConnectionId === conn.id,
                        }"
                        @click="selectConnection(conn.id)"
                    >
                        <div
                            class="flex items-center justify-between gap-2 mb-1 min-w-0"
                        >
                            <span class="row-title">{{ conn.id }}</span>
                            <span :class="stateTagClass(conn.state)">{{
                                stateLabel(conn.state)
                            }}</span>
                        </div>
                        <div class="row-sub">
                            {{ conn.connection?.host ?? "—" }}
                        </div>
                    </div>
                </div>
            </aside>

            <!-- Pane 2 · Message list -->
            <section class="pane pane--border">
                <div class="pane-header">
                    <span class="truncate">{{
                        activeConnectionId ?? "Messages"
                    }}</span>
                    <span class="count-badge">{{ messageCount }}</span>
                </div>
                <div class="pane-scroll">
                    <div v-if="!activeConnectionId" class="empty-state">
                        <span class="empty-icon">←</span>
                        <span>select a connection</span>
                    </div>
                    <div v-else-if="messageCount === 0" class="empty-state">
                        <span class="empty-icon">⌀</span>
                        <span>no messages</span>
                    </div>
                    <div
                        v-for="msg in activeMessages"
                        :key="msg.id"
                        class="list-row"
                        :class="{
                            'list-row--active': activeMessage?.id === msg.id,
                        }"
                        @click="selectMessage(msg)"
                    >
                        <div class="row-title">
                            {{ formatAddress(msg.envelope?.from) }}
                        </div>
                        <div class="row-sub" style="margin-top: 2px">
                            {{ msg.subject || "(no subject)" }}
                        </div>
                        <div class="row-date">
                            {{ formatDate(msg.envelope?.date) }}
                        </div>
                    </div>
                </div>
            </section>

            <!-- Pane 3 · Viewer -->
            <main class="pane viewer-pane">
                <div class="pane-header">
                    <span>Message</span>
                    <div
                        v-if="activeMessage"
                        class="ml-auto flex items-center gap-1"
                    >
                        <button
                            class="view-btn"
                            :class="{ 'view-btn--active': viewMode === 'text' }"
                            @click="viewMode = 'text'"
                        >
                            Text
                        </button>
                        <button
                            class="view-btn"
                            :class="{ 'view-btn--active': viewMode === 'json' }"
                            @click="viewMode = 'json'"
                        >
                            JSON
                        </button>
                    </div>
                </div>
                <div class="pane-scroll">
                    <div v-if="!activeMessage" class="empty-state">
                        <span class="empty-icon">✉</span>
                        <span>select a message</span>
                    </div>
                    <div v-else-if="viewMode === 'text'" class="viewer-content">
                        <h2 class="viewer-subject">
                            {{ activeMessage.subject || "(no subject)" }}
                        </h2>
                        <dl class="meta-grid">
                            <div class="meta-row">
                                <dt class="meta-label">From</dt>
                                <dd class="meta-value">
                                    {{
                                        formatAddress(
                                            activeMessage.envelope?.from,
                                        )
                                    }}
                                </dd>
                            </div>
                            <div class="meta-row">
                                <dt class="meta-label">To</dt>
                                <dd class="meta-value">
                                    {{
                                        formatAddress(
                                            activeMessage.envelope?.to,
                                        )
                                    }}
                                </dd>
                            </div>
                            <div class="meta-row">
                                <dt class="meta-label">Date</dt>
                                <dd class="meta-value">
                                    {{
                                        formatDate(activeMessage.envelope?.date)
                                    }}
                                </dd>
                            </div>
                        </dl>
                        <div class="viewer-divider"></div>
                        <pre class="viewer-body">{{
                            activeMessage.body_text?.[0] || "(empty)"
                        }}</pre>
                    </div>
                    <div v-else class="viewer-content">
                        <pre class="json-view" v-html="highlightedJson"></pre>
                    </div>
                </div>
            </main>
        </div>
    </div>
</template>

<style scoped>
/* ── Shell ── */
.app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    background: var(--surface-base);
    color: var(--text-primary);
    font-family: var(--font-mono);
    font-size: 12px;
}

/* ── Titlebar ── */
.titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    height: 40px;
    flex-shrink: 0;
    background: var(--surface-raised);
    border-bottom: 1px solid var(--border-subtle);
}
.titlebar-logo {
    color: var(--accent);
    font-size: 14px;
    line-height: 1;
}
.titlebar-name {
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0.05em;
}
.titlebar-sub {
    font-size: 11px;
    color: var(--text-tertiary);
    letter-spacing: 0.04em;
}
.stat-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-secondary);
}
.stat-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
}
.stat-dot--ok {
    background: var(--status-ok);
    box-shadow: 0 0 5px var(--status-ok-glow);
}
.stat-dot--off {
    background: var(--text-tertiary);
}

.theme-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 26px;
    height: 22px;
    border-radius: 4px;
    border: 1px solid var(--border-subtle);
    background: transparent;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: 13px;
    cursor: pointer;
    transition:
        color 100ms,
        border-color 100ms;
}
.theme-btn:hover {
    color: var(--text-secondary);
    border-color: var(--border-default);
}

/* ── Layout ── */
.body-grid {
    display: flex;
    flex: 1;
    overflow: hidden;
}
.pane {
    display: flex;
    flex-direction: column;
    overflow: hidden;
}
.pane--border {
    border-right: 1px solid var(--border-subtle);
}
aside.pane {
    width: 220px;
    flex-shrink: 0;
}
section.pane {
    width: 260px;
    flex-shrink: 0;
}
.viewer-pane {
    flex: 1;
}

/* ── Pane header ── */
.pane-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    height: 32px;
    flex-shrink: 0;
    background: var(--surface-raised);
    border-bottom: 1px solid var(--border-subtle);
    color: var(--text-tertiary);
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
}

.count-badge {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    height: 16px;
    padding: 0 6px;
    border-radius: 10px;
    border: 1px solid var(--border-subtle);
    background: var(--surface-sunken);
    color: var(--text-tertiary);
    font-size: 10px;
    letter-spacing: 0;
}

/* ── Pane scroll ── */
.pane-scroll {
    flex: 1;
    overflow-y: auto;
    background: var(--surface-base);
}

/* ── Empty state ── */
.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 100%;
    color: var(--text-tertiary);
    font-size: 11px;
    letter-spacing: 0.04em;
}
.empty-icon {
    font-size: 20px;
    opacity: 0.25;
}

/* ── List rows ── */
.list-row {
    padding: 9px 12px 9px 10px;
    border-bottom: 1px solid var(--border-subtle);
    border-left: 2px solid transparent;
    cursor: pointer;
    transition: background 80ms ease;
}
.list-row:hover {
    background: var(--surface-overlay);
}
.list-row--active {
    border-left-color: var(--accent);
    background: var(--surface-overlay);
}
.list-row--active .row-title {
    color: var(--accent);
}

.row-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: color 80ms;
}
.row-sub {
    font-size: 11px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.row-date {
    font-size: 10px;
    color: var(--text-tertiary);
    margin-top: 3px;
}

/* ── Tags ── */
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

/* ── View toggle ── */
.view-btn {
    padding: 2px 7px;
    border-radius: 3px;
    border: 1px solid var(--border-subtle);
    background: transparent;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    cursor: pointer;
    transition: all 80ms ease;
}
.view-btn:hover {
    color: var(--text-secondary);
    border-color: var(--border-default);
}
.view-btn--active {
    color: var(--accent);
    border-color: var(--accent-border);
    background: var(--accent-subtle);
}

/* ── Viewer ── */
.viewer-content {
    padding: 22px 26px;
}
.viewer-subject {
    font-size: 14px;
    font-weight: 700;
    color: var(--text-primary);
    margin: 0 0 14px;
    line-height: 1.3;
    letter-spacing: -0.01em;
}
.meta-grid {
    display: flex;
    flex-direction: column;
    gap: 5px;
}
.meta-row {
    display: flex;
    gap: 14px;
    align-items: baseline;
}
.meta-label {
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--text-tertiary);
    width: 32px;
    flex-shrink: 0;
}
.meta-value {
    font-size: 12px;
    color: var(--text-secondary);
}
.viewer-divider {
    height: 1px;
    background: var(--border-subtle);
    margin: 18px 0;
}
.viewer-body {
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.7;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
    margin: 0;
}

/* ── JSON view ── */
.json-view {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.65;
    color: var(--text-secondary);
    white-space: pre;
    overflow-x: auto;
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
