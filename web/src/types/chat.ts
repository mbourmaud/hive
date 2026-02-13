// ── Stream event types (Claude Code stream-json NDJSON format) ──────────────

export interface SystemInitEvent {
  type: "system";
  subtype: "init";
  session_id: string;
}

export interface SystemHeartbeatEvent {
  type: "system";
  subtype: "heartbeat";
}

export type SystemEvent = SystemInitEvent | SystemHeartbeatEvent;

export interface AssistantTextBlock {
  type: "text";
  text: string;
}

export interface AssistantToolUseBlock {
  type: "tool_use";
  id: string;
  name: string;
  input: Record<string, unknown>;
}

export type AssistantContentBlock = AssistantTextBlock | AssistantToolUseBlock;

export interface AssistantEvent {
  type: "assistant";
  message: {
    content: AssistantContentBlock[];
  };
}

export interface ToolResultBlock {
  type: "tool_result";
  tool_use_id: string;
  content: string;
  is_error: boolean;
}

export interface UserEvent {
  type: "user";
  message: {
    content: ToolResultBlock[];
  };
}

export interface ResultEvent {
  type: "result";
  subtype: "success" | "error";
  result: string;
  is_error: boolean;
  cost?: {
    input_tokens: number;
    output_tokens: number;
    total_usd: number;
  };
}

export type StreamEvent =
  | SystemEvent
  | AssistantEvent
  | UserEvent
  | ResultEvent;

// ── Assistant part types (rendered in UI) ───────────────────────────────────

export type ToolStatus = "pending" | "running" | "completed" | "error";

export interface TextPart {
  type: "text";
  id: string;
  text: string;
}

export interface ToolUsePart {
  type: "tool_use";
  id: string;
  name: string;
  input: Record<string, unknown>;
  status: ToolStatus;
}

export interface ToolResultPart {
  type: "tool_result";
  id: string;
  toolUseId: string;
  content: string;
  isError: boolean;
}

export type AssistantPart = TextPart | ToolUsePart | ToolResultPart;

// ── Chat turn (one user prompt → assistant response cycle) ──────────────────

export type TurnStatus = "pending" | "streaming" | "completed" | "error";

export interface ChatTurn {
  id: string;
  userMessage: string;
  assistantParts: AssistantPart[];
  status: TurnStatus;
  duration: number | null;
  startedAt: number;
}

// ── Chat session ────────────────────────────────────────────────────────────

export type SessionStatus = "idle" | "busy" | "completed" | "error";

export interface ChatSession {
  id: string;
  status: SessionStatus;
  cwd: string;
  createdAt: string;
}

// ── Reducer state & actions ─────────────────────────────────────────────────

export interface ChatState {
  session: ChatSession | null;
  turns: ChatTurn[];
  currentTurnId: string | null;
  isStreaming: boolean;
  lastEventAt: number | null;
  isStale: boolean;
  error: string | null;
}

export type ChatAction =
  | { type: "SESSION_CREATED"; session: ChatSession }
  | { type: "SESSION_RESET" }
  | { type: "TURN_STARTED"; turnId: string; userMessage: string }
  | { type: "STREAM_EVENT"; event: StreamEvent }
  | { type: "STREAM_EVENT_BATCH"; events: StreamEvent[] }
  | { type: "TURN_COMPLETED"; turnId: string }
  | { type: "TURN_ERROR"; turnId: string; error: string }
  | { type: "MARK_STALE" }
  | { type: "CONNECTION_ERROR"; error: string }
  | { type: "REPLAY_HISTORY"; session: ChatSession; events: StreamEvent[] };
