import { ref, computed, onMounted } from 'vue'

// Singleton state at module scope
const connections = ref([])
const activeConnectionId = ref(null)
const messages = ref({}) // connectionId -> array of messages
const activeMessage = ref(null)

let sseInitialized = false

function initSSE() {
  if (sseInitialized) return
  sseInitialized = true

  const source = new EventSource('/sse')

  source.addEventListener('connections', (e) => {
    connections.value = JSON.parse(e.data)
    if (!activeConnectionId.value && connections.value.length > 0) {
      activeConnectionId.value = connections.value[0].id
    }
  })

  source.addEventListener('message', (e) => {
    const payload = JSON.parse(e.data)
    const id = payload.connection_id
    const current = messages.value[id] || []
    const message = { id: new Date().getTime(), ...payload.message }
    messages.value = { ...messages.value, [id]: [...current, message] }
  })

  source.onerror = () => console.warn('SSE connection lost, reconnecting...')
}

// Computed helpers (module-level, reused across all useAppStore() calls)
const activeConnection = computed(() =>
  connections.value.find(c => c.id === activeConnectionId.value) || null
)

const activeMessages = computed(() =>
  messages.value[activeConnectionId.value] || []
)

function selectConnection(id) {
  activeConnectionId.value = id
  activeMessage.value = null
}

function selectMessage(msg) {
  activeMessage.value = msg
}

function formatAddress(addr) {
  if (!addr) return '—'
  const addrs = addr.List || (addr.Group ? addr.Group.flatMap(g => g.addresses) : [])
  return addrs.map(a => a.name ? `${a.name} <${a.email}>` : a.email).join(', ') || '—'
}

function formatDate(dateStr) {
  if (!dateStr) return '—'
  const d = new Date(dateStr)
  if (isNaN(d)) return dateStr
  const today = new Date()
  const isToday =
    d.getFullYear() === today.getFullYear() &&
    d.getMonth() === today.getMonth() &&
    d.getDate() === today.getDate()
  if (isToday) {
    return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })
  }
  return d.toLocaleDateString([], {
    month: 'short',
    day: 'numeric',
    year: d.getFullYear() !== today.getFullYear() ? 'numeric' : undefined,
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
  })
}

function stateLabel(state) {
  if (!state) return 'unknown'
  return state.type || 'unknown'
}

export function useAppStore() {
  onMounted(() => {
    initSSE()
  })

  return {
    connections,
    activeConnectionId,
    messages,
    activeMessage,
    activeConnection,
    activeMessages,
    selectConnection,
    selectMessage,
    formatAddress,
    formatDate,
    stateLabel,
  }
}
