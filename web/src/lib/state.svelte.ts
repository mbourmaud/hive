import type { Agent, Task, Solicitation, Message } from './types'
import { api } from './api'

class HiveState {
  agents = $state<Agent[]>([])
  tasks = $state<Task[]>([])
  solicitations = $state<Solicitation[]>([])
  selectedId = $state<string | null>(null)
  conversation = $state<Message[]>([])
  connected = $state(false)
  loading = $state(false)
  
  private ws: WebSocket | null = null
  private refreshInterval: number | null = null

  get activeDrones() {
    return this.agents.filter(a => a.status === 'ready' || a.status === 'busy')
  }

  get selectedDrone() {
    return this.agents.find(a => a.id === this.selectedId) ?? null
  }

  get tasksByDrone(): Map<string, Task[]> {
    const map = new Map<string, Task[]>()
    for (const drone of this.activeDrones) {
      map.set(drone.id, [])
    }
    for (const task of this.tasks) {
      const droneId = task.agent_id
      const list = map.get(droneId) ?? []
      list.push(task)
      map.set(droneId, list)
    }
    return map
  }

  get pendingSolicitations() {
    return this.solicitations.filter(s => s.status === 'pending')
  }

  async refresh() {
    try {
      const data = await api.getData()
      this.agents = data.agents ?? []
      this.tasks = data.tasks ?? []
      this.solicitations = data.solicitations ?? []
    } catch (e) {
      console.error('Failed to refresh:', e)
    }
  }

  async selectDrone(id: string | null) {
    this.selectedId = id
    if (id) {
      await this.loadConversation()
    } else {
      this.conversation = []
    }
  }

  async loadConversation() {
    if (!this.selectedId) return
    try {
      this.conversation = await api.getConversation(this.selectedId)
    } catch (e) {
      console.error('Failed to load conversation:', e)
      this.conversation = []
    }
  }

  async sendMessage(content: string) {
    if (!this.selectedId || !content.trim()) return
    await api.sendMessage(this.selectedId, content)
    setTimeout(() => this.loadConversation(), 500)
  }

  async killDrone() {
    if (!this.selectedId) return
    await api.killAgent(this.selectedId)
    this.selectedId = null
    this.conversation = []
    await this.refresh()
  }

  async destroyDrone() {
    if (!this.selectedId) return
    await api.destroyAgent(this.selectedId)
    this.selectedId = null
    this.conversation = []
    await this.refresh()
  }

  connectWebSocket() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const wsUrl = `${protocol}//${window.location.host}/ws`
    
    this.ws = new WebSocket(wsUrl)
    
    this.ws.onopen = () => {
      this.connected = true
    }
    
    this.ws.onclose = () => {
      this.connected = false
      setTimeout(() => this.connectWebSocket(), 3000)
    }
    
    this.ws.onmessage = () => {
      this.refresh()
      if (this.selectedId) this.loadConversation()
    }
  }

  startPolling() {
    this.refresh()
    this.refreshInterval = window.setInterval(() => this.refresh(), 5000)
    
    window.setInterval(() => {
      if (this.selectedId) this.loadConversation()
    }, 3000)
  }

  init() {
    this.connectWebSocket()
    this.startPolling()
  }

  destroy() {
    this.ws?.close()
    if (this.refreshInterval) clearInterval(this.refreshInterval)
  }
}

export const hive = new HiveState()
