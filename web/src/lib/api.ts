import type { HiveData, Message, CreateTaskRequest } from './types'

async function get<T>(path: string): Promise<T> {
  const res = await fetch(path)
  if (!res.ok) throw new Error(`GET ${path}: ${res.status}`)
  return res.json()
}

async function post<T = unknown>(path: string, body?: object): Promise<T> {
  const res = await fetch(path, {
    method: 'POST',
    headers: body ? { 'Content-Type': 'application/json' } : {},
    body: body ? JSON.stringify(body) : undefined
  })
  if (!res.ok) throw new Error(`POST ${path}: ${res.status}`)
  return res.json()
}

async function del<T = unknown>(path: string): Promise<T> {
  const res = await fetch(path, { method: 'DELETE' })
  if (!res.ok) throw new Error(`DELETE ${path}: ${res.status}`)
  return res.json()
}

export const api = {
  getData: () => get<HiveData>('/api/data'),
  getConversation: (id: string) => get<Message[]>(`/api/agents/${id}/conversation`),
  sendMessage: (id: string, content: string) => post(`/api/agents/${id}/message`, { content }),
  killAgent: (id: string) => del(`/api/agents/${id}`),
  destroyAgent: (id: string) => del(`/api/agents/${id}/destroy`),
  createTask: (data: CreateTaskRequest) => post('/api/tasks', data),
  taskAction: (id: string, action: 'start' | 'complete' | 'cancel') => 
    post(`/api/tasks/${id}/${action}`),
  respondSolicitation: (id: string, response: string) => 
    post(`/api/solicitations/${id}/respond`, { response }),
  dismissSolicitation: (id: string) => post(`/api/solicitations/${id}/dismiss`)
}
