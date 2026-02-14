export interface SystemStatus {
  auth: AuthStatusSummary;
  session: SessionSummary;
  mcp_servers: McpServerInfo[];
  drones: DroneStatusBrief[];
  version: string;
}

export interface AuthStatusSummary {
  configured: boolean;
  auth_type: string | null;
  expired: boolean;
}

export interface SessionSummary {
  active_count: number;
  total_count: number;
}

export interface McpServerInfo {
  name: string;
  command: string;
  args: string[];
}

export interface DroneStatusBrief {
  name: string;
  liveness: string;
  progress: [number, number];
  elapsed: string;
  cost_usd: number;
  is_stuck: boolean;
}

export type OverallHealth = "healthy" | "warning" | "error" | "unknown";
