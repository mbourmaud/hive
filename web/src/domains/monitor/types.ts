export interface ProjectInfo {
  name: string;
  path: string;
  drones: DroneInfo[];
  total_cost: number;
  active_count: number;
}

export interface DroneInfo {
  name: string;
  title: string | null;
  description: string | null;
  status: string;
  branch: string;
  worktree: string;
  lead_model: string | null;
  phase: string | null;
  started: string;
  updated: string;
  elapsed: string;
  tasks: TaskInfo[];
  members: MemberInfo[];
  messages: MessageInfo[];
  progress: [number, number];
  cost: CostInfo;
  liveness: string;
}

export interface TaskInfo {
  id: string;
  subject: string;
  description: string;
  status: string;
  owner: string | null;
  active_form: string | null;
  is_internal: boolean;
  duration: string | null;
  retry_count: number;
  blocked_by: string | null;
}

export interface MemberInfo {
  name: string;
  agent_type: string;
  model: string;
  liveness: string;
  current_task_id: string | null;
}

export interface MessageInfo {
  from: string;
  to: string;
  text: string;
  timestamp: string;
}

export interface CostInfo {
  total_usd: number;
  input_tokens: number;
  output_tokens: number;
  cache_creation_tokens: number;
  cache_read_tokens: number;
}
