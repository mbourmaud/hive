export interface Agent {
  id: string
  name: string
  worktree_path: string
  branch: string
  port: number
  status: 'ready' | 'busy' | 'stopped' | 'error' | 'starting'
  specialty?: string
  created_at: string
  error?: string
}

export interface Task {
  id: string
  agent_id: string
  agent_name?: string
  status: 'pending' | 'assigned' | 'in_progress' | 'completed' | 'failed' | 'cancelled'
  current_step: number
  plan?: {
    title: string
    description?: string
    steps?: TaskStep[]
  }
}

export interface TaskStep {
  action: string
  description?: string
  dod?: string[]
  autonomy?: string
  status?: string
}

export interface Solicitation {
  id: string
  agent_id: string
  agent_name: string
  type: string
  urgency: 'low' | 'medium' | 'high' | 'critical'
  message: string
  status: 'pending' | 'responded' | 'dismissed' | 'expired'
  options?: string[]
  created_at: string
}

export interface Message {
  role: 'user' | 'assistant'
  content: string
}

export interface CreateTaskRequest {
  agent_id: string
  agent_name?: string
  title: string
  description?: string
  steps: { action: string; dod?: string[]; autonomy?: string }[]
}

export interface HiveData {
  agents: Agent[]
  tasks: Task[]
  solicitations: Solicitation[]
}
