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

export interface AssistantThinkingBlock {
  type: "thinking";
  thinking: string;
}

export type AssistantContentBlock =
  | AssistantTextBlock
  | AssistantToolUseBlock
  | AssistantThinkingBlock;

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

export interface UserTextBlock {
  type: "text";
  text: string;
}

export type UserContentBlock = ToolResultBlock | UserTextBlock;

export interface UserEvent {
  type: "user";
  message: {
    content: UserContentBlock[];
  };
}

export interface ResultEvent {
  type: "result";
  subtype: "success" | "error";
  result: string;
  is_error: boolean;
  error_code?: string;
  cost?: {
    input_tokens: number;
    output_tokens: number;
    total_usd: number;
  };
  usage?: {
    input_tokens: number;
    output_tokens: number;
  };
}

export interface UsageEvent {
  type: "usage";
  input_tokens: number;
  output_tokens: number;
  total_input: number;
  total_output: number;
  cache_creation_input_tokens?: number;
  cache_read_input_tokens?: number;
}

export interface CompactEvent {
  type: "compact.completed";
  summary: string;
  total_input: number;
  total_output: number;
}

export interface SessionCompletedEvent {
  type: "session.completed";
}

export type StreamEvent =
  | SystemEvent
  | AssistantEvent
  | UserEvent
  | ResultEvent
  | UsageEvent
  | CompactEvent
  | SessionCompletedEvent;

// ── Assistant part types (rendered in UI) ───────────────────────────────────

export type ToolStatus = "pending" | "running" | "completed" | "error";

export interface TextPart {
  type: "text";
  id: string;
  text: string;
}

export interface ThinkingPart {
  type: "thinking";
  id: string;
  text: string;
  topic?: string;
}

export interface ToolUsePart {
  type: "tool_use";
  id: string;
  name: string;
  input: Record<string, unknown>;
  status: ToolStatus;
  startedAt?: number;
  duration?: number;
}

export interface ToolResultPart {
  type: "tool_result";
  id: string;
  toolUseId: string;
  content: string;
  isError: boolean;
}

export type AssistantPart = TextPart | ThinkingPart | ToolUsePart | ToolResultPart;

// ── Chat turn (one user prompt → assistant response cycle) ──────────────────

export type TurnStatus = "pending" | "streaming" | "completed" | "error";

export type FinishReason = "end_turn" | "canceled" | "error" | "max_tokens" | "aws_sso_expired";

export interface ChatTurn {
  id: string;
  userMessage: string;
  images?: ImageAttachment[];
  assistantParts: AssistantPart[];
  status: TurnStatus;
  duration: number | null;
  startedAt: number;
  finishReason?: FinishReason;
  model?: string;
  /** When set, this turn represents a drone launch (not a normal chat exchange). */
  droneName?: string;
}

// ── Chat session ────────────────────────────────────────────────────────────

export type SessionStatus = "idle" | "busy" | "completed" | "error";

export interface ChatSession {
  id: string;
  status: SessionStatus;
  cwd: string;
  createdAt: string;
}

// ── Image attachment (pasted/dropped into prompt) ───────────────────────────

export interface ImageAttachment {
  id: string;
  dataUrl: string;
  mimeType: string;
  name: string;
}

// ── Queued message (typed while Claude is streaming) ────────────────────────

export interface QueuedMessage {
  id: string;
  text: string;
  images?: ImageAttachment[];
  queuedAt: number;
}

// ── Slash command definition ────────────────────────────────────────────────

export interface SlashCommand {
  name: string;
  description: string;
  shortcut?: string;
  category?: "session" | "config" | "view" | "info" | "drone";
  type?: "builtin" | "custom";
  source?: "project" | "user" | "tools";
}

// ── Reducer state & actions ─────────────────────────────────────────────────

export interface ContextUsage {
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens?: number;
  cacheWriteTokens?: number;
  totalCost?: number;
}

export interface ChatState {
  session: ChatSession | null;
  turns: ChatTurn[];
  currentTurnId: string | null;
  isStreaming: boolean;
  lastEventAt: number | null;
  isStale: boolean;
  error: string | null;
  contextUsage: ContextUsage | null;
  messageQueue: QueuedMessage[];
}

export type ChatAction =
  | { type: "SESSION_CREATED"; session: ChatSession }
  | { type: "SESSION_RESET" }
  | { type: "TURN_STARTED"; turnId: string; userMessage: string; model?: string; images?: ImageAttachment[] }
  | { type: "STREAM_EVENT"; event: StreamEvent }
  | { type: "STREAM_EVENT_BATCH"; events: StreamEvent[] }
  | { type: "TURN_COMPLETED"; turnId: string }
  | { type: "TURN_ERROR"; turnId: string; error: string }
  | { type: "MARK_STALE" }
  | { type: "CONNECTION_ERROR"; error: string }
  | {
      type: "REPLAY_HISTORY";
      session: ChatSession;
      events: StreamEvent[];
      tokenCounts?: { inputTokens: number; outputTokens: number };
    }
  | { type: "DRONE_LAUNCHED"; droneName: string; prompt: string }
  | { type: "ENQUEUE_MESSAGE"; message: QueuedMessage }
  | { type: "DEQUEUE_MESSAGE" }
  | { type: "CANCEL_QUEUED_MESSAGE"; messageId: string }
  | { type: "CLEAR_QUEUE" };

// ── Per-project snapshot (saved/restored on project switch) ────────────────

export interface ProjectChatSnapshot {
  chatState: ChatState;
  activeSessionId: string | null;
  promptDraft: string;
  selectedModel: string | null;
  effort: "low" | "medium" | "high";
  chatMode: "code" | "hive-plan" | "plan";
  wasStreaming: boolean;
  streamingSessionId: string | null;
  streamingTurnId: string | null;
  messageQueue: QueuedMessage[];
}
