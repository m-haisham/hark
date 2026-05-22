Alpine.store('app', {
  connections: [],
  activeConnectionId: null,
  messages: {
    // connectionId: [msg1, msg2, ...]
  },
  activeMessage: null,

  init() {
    this.source = new EventSource('/sse')

    this.source.addEventListener('connections', (e) => {
      this.connections = JSON.parse(e.data)
      if (!this.activeConnectionId && this.connections.length > 0) {
        this.activeConnectionId = this.connections[0].id
      }
    })

    this.source.addEventListener('message', (e) => {
      const payload = JSON.parse(e.data)
      const id = payload.connection_id
      if (!this.messages[id]) {
        this.messages = { ...this.messages, [id]: [] }
      }

      const message = {
        id: new Date().getTime(),
        ...payload.message,
      }

      this.messages[id].push(message)
      this.messages = { ...this.messages }
    })

    this.source.onerror = () => {
      console.warn('SSE connection lost, reconnecting...')
    }
  },

  get activeConnection() {
    return (
      this.connections.find((c) => c.id === this.activeConnectionId) || null
    )
  },

  get activeMessages() {
    return this.messages[this.activeConnectionId] || []
  },

  selectConnection(id) {
    this.activeConnectionId = id
    this.activeMessage = null
  },

  selectMessage(msg) {
    this.activeMessage = msg
  },

  formatAddress(addr) {
    if (!addr) return '—'
    const addrs =
      addr.List || (addr.Group ? addr.Group.flatMap((g) => g.addresses) : [])
    return (
      addrs
        .map((a) => (a.name ? `${a.name} <${a.email}>` : a.email))
        .join(', ') || '—'
    )
  },

  formatDate(dateStr) {
    if (!dateStr) return '—'
    const d = new Date(dateStr)
    if (isNaN(d)) return dateStr
    const today = new Date()
    const isToday =
      d.getFullYear() === today.getFullYear() &&
      d.getMonth() === today.getMonth() &&
      d.getDate() === today.getDate()
    if (isToday) {
      return d.toLocaleTimeString([], {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
      })
    }
    return d.toLocaleDateString([], {
      month: 'short',
      day: 'numeric',
      year: d.getFullYear() !== today.getFullYear() ? 'numeric' : undefined,
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    })
  },

  stateLabel(state) {
    if (!state) return 'unknown'
    return state.type || 'unknown'
  },
})
