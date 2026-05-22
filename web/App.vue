<script setup>
import { ref, computed } from "vue";
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

// Message view mode: 'text' | 'json'
const viewMode = ref("text");

function stateTagClass(state) {
    const s = stateLabel(state);
    if (s === "running") return "tag tag-green";
    if (s === "starting" || s === "stopping") return "tag tag-amber";
    if (s === "failed") return "tag tag-red";
    return "tag tag-gray";
}

// Syntax-highlighted JSON renderer
function highlight(json) {
    const str = JSON.stringify(json, null, 2);
    return str.replace(
        /("(\\u[a-zA-Z0-9]{4}|\\[^u]|[^\\"])*"(\s*:)?|\b(true|false|null)\b|-?\d+(?:\.\d*)?(?:[eE][+\-]?\d+)?)/g,
        (match) => {
            if (/^"/.test(match)) {
                if (/:$/.test(match)) {
                    return `<span class="json-key">${match}</span>`;
                }
                return `<span class="json-string">${match}</span>`;
            }
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
        <!-- ── Titlebar ─────────────────────────────────────────────── -->
        <header class="titlebar">
            <div class="flex items-center gap-2">
                <span class="titlebar-logo">◈</span>
                <span class="titlebar-name">hark</span>
                <span class="titlebar-sub">imap inspector</span>
            </div>
            <div class="flex items-center gap-3">
                <span v-if="connections.length > 0" class="stat-pill">
                    <span class="stat-dot running"></span>
                    {{ connections.length }}
                    {{
                        connections.length === 1 ? "connection" : "connections"
                    }}
                </span>
                <span v-else class="stat-pill">
                    <span class="stat-dot stopped"></span>
                    no connections
                </span>
            </div>
        </header>

        <!-- ── 3-pane body ───────────────────────────────────────────── -->
        <div class="body-grid">
            <!-- Pane 1 · Connections ────────────────────────────────── -->
            <aside class="pane pane-divider">
                <div class="pane-header">
                    <span>Connections</span>
                    <span class="ml-auto count-badge">{{
                        connections.length
                    }}</span>
                </div>

                <div class="pane-scroll">
                    <div v-if="connections.length === 0" class="empty-state">
                        <span class="empty-icon">⌀</span>
                        <span>no connections</span>
                    </div>

                    <div
                        v-for="conn in connections"
                        :key="conn.id"
                        class="conn-row"
                        :class="{ active: activeConnectionId === conn.id }"
                        @click="selectConnection(conn.id)"
                    >
                        <div
                            class="flex items-center justify-between gap-2 mb-1"
                        >
                            <span class="conn-id">{{ conn.id }}</span>
                            <span :class="stateTagClass(conn.state)">{{
                                stateLabel(conn.state)
                            }}</span>
                        </div>
                        <div class="conn-host">
                            {{ conn.connection?.host ?? "—" }}
                        </div>
                    </div>
                </div>
            </aside>

            <!-- Pane 2 · Message list ───────────────────────────────── -->
            <section class="pane pane-divider">
                <div class="pane-header">
                    <span>{{ activeConnectionId ?? "Messages" }}</span>
                    <span class="ml-auto count-badge">{{ messageCount }}</span>
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
                        class="msg-row"
                        :class="{ active: activeMessage?.id === msg.id }"
                        @click="selectMessage(msg)"
                    >
                        <div class="msg-from">
                            {{ formatAddress(msg.envelope?.from) }}
                        </div>
                        <div class="msg-subject">
                            {{ msg.subject || "(no subject)" }}
                        </div>
                        <div class="msg-date">
                            {{ formatDate(msg.envelope?.date) }}
                        </div>
                    </div>
                </div>
            </section>

            <!-- Pane 3 · Viewer ─────────────────────────────────────── -->
            <main class="pane viewer-pane">
                <!-- viewer header with view toggle -->
                <div class="pane-header">
                    <span>Message</span>
                    <div
                        v-if="activeMessage"
                        class="ml-auto flex items-center gap-1"
                    >
                        <button
                            class="view-btn"
                            :class="{ active: viewMode === 'text' }"
                            @click="viewMode = 'text'"
                        >
                            Text
                        </button>
                        <button
                            class="view-btn"
                            :class="{ active: viewMode === 'json' }"
                            @click="viewMode = 'json'"
                        >
                            JSON
                        </button>
                    </div>
                </div>

                <div class="pane-scroll">
                    <!-- Empty -->
                    <div v-if="!activeMessage" class="empty-state">
                        <span class="empty-icon">✉</span>
                        <span>select a message</span>
                    </div>

                    <!-- Text view -->
                    <div v-else-if="viewMode === 'text'" class="viewer-content">
                        <div class="viewer-meta">
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
                                            formatDate(
                                                activeMessage.envelope?.date,
                                            )
                                        }}
                                    </dd>
                                </div>
                            </dl>
                        </div>
                        <div class="viewer-divider"></div>
                        <pre class="viewer-body">{{
                            activeMessage.body_text?.[0] || "(empty)"
                        }}</pre>
                    </div>

                    <!-- JSON view -->
                    <div v-else class="viewer-content">
                        <pre class="json-view" v-html="highlightedJson"></pre>
                    </div>
                </div>
            </main>
        </div>
    </div>
</template>

<style scoped>
.app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--color-surface);
    color: var(--color-text);
    font-family: var(--font-mono);
    overflow: hidden;
}

/* ── Titlebar ── */
.titlebar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    height: 40px;
    background: var(--color-surface-1);
    border-bottom: 1px solid var(--color-border);
    flex-shrink: 0;
}
.titlebar-logo {
    color: var(--color-accent);
    font-size: 14px;
    line-height: 1;
}
.titlebar-name {
    font-size: 13px;
    font-weight: 700;
    color: var(--color-text);
    letter-spacing: 0.05em;
}
.titlebar-sub {
    font-size: 10px;
    color: var(--color-text-faint);
    letter-spacing: 0.06em;
    margin-top: 1px;
}
.stat-pill {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--color-text-muted);
}
.stat-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    flex-shrink: 0;
}
.stat-dot.running {
    background: var(--color-green);
    box-shadow: 0 0 6px var(--color-green);
}
.stat-dot.stopped {
    background: var(--color-text-faint);
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
.pane-divider {
    border-right: 1px solid var(--color-border);
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

.pane-header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 12px;
    height: 32px;
    background: var(--color-surface-1);
    border-bottom: 1px solid var(--color-border);
    color: var(--color-text-faint);
    text-transform: uppercase;
    letter-spacing: 0.08em;
    font-size: 10px;
    font-weight: 600;
    flex-shrink: 0;
}

.count-badge {
    font-size: 10px;
    color: var(--color-text-faint);
    background: var(--color-surface-3);
    border: 1px solid var(--color-border);
    border-radius: 10px;
    padding: 0 6px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    letter-spacing: 0;
}

.pane-scroll {
    flex: 1;
    overflow-y: auto;
}

/* ── Empty state ── */
.empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    height: 100%;
    color: var(--color-text-faint);
    font-size: 11px;
    letter-spacing: 0.04em;
}
.empty-icon {
    font-size: 20px;
    opacity: 0.3;
}

/* ── Connection rows ── */
.conn-row {
    padding: 10px 12px;
    border-bottom: 1px solid var(--color-border);
    cursor: pointer;
    transition: background 100ms ease;
}
.conn-row:hover {
    background: var(--color-surface-2);
}
.conn-row.active {
    background: var(--color-accent-dim);
    border-left: 2px solid var(--color-accent);
    padding-left: 10px;
}
.conn-id {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.conn-host {
    font-size: 11px;
    color: var(--color-text-faint);
    margin-top: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

/* ── Message rows ── */
.msg-row {
    padding: 10px 12px;
    border-bottom: 1px solid var(--color-border);
    cursor: pointer;
    transition: background 100ms ease;
}
.msg-row:hover {
    background: var(--color-surface-2);
}
.msg-row.active {
    background: var(--color-accent-dim);
    border-left: 2px solid var(--color-accent);
    padding-left: 10px;
}
.msg-from {
    font-size: 12px;
    font-weight: 600;
    color: var(--color-text);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.msg-subject {
    font-size: 11px;
    color: var(--color-text-muted);
    margin-top: 2px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}
.msg-date {
    font-size: 10px;
    color: var(--color-text-faint);
    margin-top: 3px;
}

/* ── Tags ── */
.tag {
    display: inline-flex;
    align-items: center;
    padding: 0 6px;
    border-radius: 3px;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.04em;
    height: 17px;
    flex-shrink: 0;
}
.tag-green {
    background: rgba(52, 211, 153, 0.12);
    color: var(--color-green);
}
.tag-amber {
    background: rgba(251, 191, 36, 0.12);
    color: var(--color-amber);
}
.tag-red {
    background: rgba(248, 113, 113, 0.12);
    color: var(--color-red);
}
.tag-gray {
    background: var(--color-surface-3);
    color: var(--color-text-faint);
}

/* ── View toggle buttons ── */
.view-btn {
    padding: 2px 8px;
    border-radius: 3px;
    cursor: pointer;
    font-size: 10px;
    font-weight: 600;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--color-text-faint);
    border: 1px solid var(--color-border);
    background: transparent;
    transition: all 100ms ease;
    font-family: var(--font-mono);
}
.view-btn:hover {
    color: var(--color-text-muted);
    border-color: var(--color-border-bright);
}
.view-btn.active {
    color: var(--color-accent);
    border-color: var(--color-accent);
    background: var(--color-accent-glow);
}

/* ── Message viewer ── */
.viewer-content {
    padding: 20px 24px;
}
.viewer-subject {
    font-size: 15px;
    font-weight: 700;
    color: var(--color-text);
    margin: 0 0 14px 0;
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
    gap: 12px;
    align-items: baseline;
}
.meta-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--color-text-faint);
    font-weight: 600;
    width: 32px;
    flex-shrink: 0;
}
.meta-value {
    font-size: 12px;
    color: var(--color-text-muted);
}
.viewer-divider {
    height: 1px;
    background: var(--color-border);
    margin: 18px 0;
}
.viewer-body {
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.7;
    color: var(--color-text-muted);
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
    color: var(--color-text-muted);
    white-space: pre;
    overflow-x: auto;
}
:deep(.json-key) {
    color: #a78bfa;
}
:deep(.json-string) {
    color: #86efac;
}
:deep(.json-number) {
    color: #fb923c;
}
:deep(.json-bool) {
    color: #38bdf8;
}
:deep(.json-null) {
    color: var(--color-text-faint);
}
</style>
