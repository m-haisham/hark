Alpine.store('app', {
  connections: [],
  activeConnectionId: null,

  init() {
    this.source = new EventSource('/sse')

    this.source.addEventListener('connections', (e) => {
      this.$store.connections.items = JSON.parse(e.data)
      if (
        !this.activeConnectionId &&
        this.$store.connections.items.length > 0
      ) {
        this.activeConnectionId = this.$store.connections.items[0].id
      }
    })

    this.source.onerror = () => {
      console.warn('SSE connection lost, reconnecting...')
    }
  },
})
