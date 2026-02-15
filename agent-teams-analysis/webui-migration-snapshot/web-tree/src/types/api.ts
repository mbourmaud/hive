export interface DroneInfo {
  name: string;
  status: string;
  branch: string;
  worktree: string;
  lead_model: string | null;
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
}

export interface MemberInfo {
  name: string;
  agent_type: string;
  model: string;
  liveness: string;
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
}
