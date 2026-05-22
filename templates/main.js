Alpine.store('app', {
  connections: [],
  activeConnectionId: null,
  messages: [],
  activeMessage: null,

  init() {
    this.source = new EventSource('/sse')

    this.source.addEventListener('connections', (e) => {
      this.connections = JSON.parse(e.data)
      if (!this.activeConnectionId && this.connections.length > 0) {
        this.activeConnectionId = this.connections[0].id
      }
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

  selectConnection(id) {
    this.activeConnectionId = id
    this.activeMessage = null
  },

  selectMessage(msg) {
    this.activeMessage = msg
  },

  stateLabel(state) {
    if (!state) return 'unknown'
    return state.type || 'unknown'
  },
})
