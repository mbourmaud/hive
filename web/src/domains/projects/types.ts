// ── Project Registry Types ──────────────────────────────────────────────────

export interface ProjectProfile {
  id: string;
  name: string;
  path: string;
  color_theme: string | null;
  image_url: string | null;
}

export interface ProjectContext {
  git: GitContext | null;
  runtimes: RuntimeInfo[];
  key_files: string[];
  open_pr: PrInfo | null;
}

export interface GitContext {
  branch: string;
  remote_url: string;
  platform: string;
  ahead: number;
  behind: number;
  dirty_count: number;
  last_commit: string | null;
}

export interface RuntimeInfo {
  name: string;
  version: string | null;
  marker_file: string;
}

export interface PrInfo {
  number: number;
  title: string;
  url: string;
  state: string;
  is_draft: boolean;
}

// ── Detection SSE Events ────────────────────────────────────────────────────

export type DetectionStepStatus = "pending" | "running" | "completed" | "failed";

export interface DetectionStep {
  step: string;
  label: string;
  status: DetectionStepStatus;
  result?: unknown;
  error?: string;
}

export type DetectionEvent =
  | { type: "step_started"; step: string; label: string }
  | { type: "step_completed"; step: string; label: string; result: unknown }
  | { type: "step_failed"; step: string; label: string; error: string }
  | { type: "all_complete"; context: ProjectContext };

// ── Request DTOs ────────────────────────────────────────────────────────────

export interface CreateProjectRequest {
  name: string;
  path: string;
  color_theme?: string;
}

export interface UpdateProjectRequest {
  name?: string;
  color_theme?: string;
}
