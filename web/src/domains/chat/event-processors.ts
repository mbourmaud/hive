import { extractThinkingTopic, findCurrentTurn, nextPartId, updateTurn } from "./reducer-utils";
import type {
  AssistantEvent,
  AssistantPart,
  ChatState,
  CompactEvent,
  FinishReason,
  ResultEvent,
  StreamEvent,
  SystemEvent,
  ThinkingPart,
  UsageEvent,
  UserEvent,
} from "./types";

// ── Process a single stream event into state ────────────────────────────────

export function processStreamEvent(state: ChatState, event: StreamEvent): ChatState {
  const now = Date.now();
  const base = { ...state, lastEventAt: now, isStale: false };

  switch (event.type) {
    case "system":
      return processSystemEvent(base, event);
    case "assistant":
      return processAssistantEvent(base, event);
    case "user":
      return processUserEvent(base, event);
    case "result":
      return processResultEvent(base, event);
    case "usage":
      return processUsageEvent(base, event);
    case "compact.completed":
      return processCompactEvent(base, event);
    default:
      return base;
  }
}

// ── Individual event processors ─────────────────────────────────────────────

function processSystemEvent(state: ChatState, event: SystemEvent): ChatState {
  if (event.subtype === "init") {
    return {
      ...state,
      session: state.session
        ? { ...state.session, status: "busy" }
        : {
            id: event.session_id,
            status: "busy",
            cwd: "",
            createdAt: new Date().toISOString(),
          },
      isStreaming: true,
    };
  }
  return state;
}

function processAssistantEvent(state: ChatState, event: AssistantEvent): ChatState {
  const turn = findCurrentTurn(state);
  if (!turn) return state;

  const newParts: AssistantPart[] = [...turn.assistantParts];

  for (const block of event.message.content) {
    switch (block.type) {
      case "text": {
        const lastPart = newParts[newParts.length - 1];
        if (lastPart && lastPart.type === "text") {
          newParts[newParts.length - 1] = {
            ...lastPart,
            text: lastPart.text + block.text,
          };
        } else {
          newParts.push({
            type: "text",
            id: nextPartId(),
            text: block.text,
          });
        }
        break;
      }
      case "thinking": {
        const lastPart = newParts[newParts.length - 1];
        if (lastPart && lastPart.type === "thinking") {
          const merged = lastPart.text + block.thinking;
          newParts[newParts.length - 1] = {
            ...lastPart,
            text: merged,
            topic: extractThinkingTopic(merged),
          };
        } else {
          const thinkingPart: ThinkingPart = {
            type: "thinking",
            id: nextPartId(),
            text: block.thinking,
            topic: extractThinkingTopic(block.thinking),
          };
          newParts.push(thinkingPart);
        }
        break;
      }
      case "tool_use": {
        newParts.push({
          type: "tool_use",
          id: block.id,
          name: block.name,
          input: block.input,
          status: "running",
          startedAt: Date.now(),
        });
        break;
      }
    }
  }

  return {
    ...state,
    turns: updateTurn(state.turns, turn.id, (t) => ({
      ...t,
      assistantParts: newParts,
      status: "streaming",
    })),
  };
}

function processUserEvent(state: ChatState, event: UserEvent): ChatState {
  const turn = findCurrentTurn(state);
  if (!turn) return state;

  const newParts: AssistantPart[] = [...turn.assistantParts];

  for (const block of event.message.content) {
    if (block.type === "tool_result") {
      const toolIdx = newParts.findIndex(
        (p) => p.type === "tool_use" && p.id === block.tool_use_id,
      );
      if (toolIdx !== -1) {
        const toolPart = newParts[toolIdx];
        if (toolPart && toolPart.type === "tool_use") {
          const toolDuration = toolPart.startedAt ? Date.now() - toolPart.startedAt : undefined;
          newParts[toolIdx] = {
            ...toolPart,
            status: block.is_error ? "error" : "completed",
            duration: toolDuration,
          };
        }
      }

      newParts.push({
        type: "tool_result",
        id: nextPartId(),
        toolUseId: block.tool_use_id,
        content: block.content,
        isError: block.is_error,
      });
    }
  }

  return {
    ...state,
    turns: updateTurn(state.turns, turn.id, (t) => ({
      ...t,
      assistantParts: newParts,
    })),
  };
}

function deriveFinishReason(event: ResultEvent): FinishReason {
  if (event.is_error) {
    if (event.error_code === "aws_sso_expired" || event.error_code === "aws_credentials") {
      return "aws_sso_expired";
    }
    const lower = (event.result ?? "").toLowerCase();
    if (lower.includes("cancel") || lower.includes("abort")) {
      return "canceled";
    }
    if (
      lower.includes("max_tokens") ||
      lower.includes("max tokens") ||
      lower.includes("token limit")
    ) {
      return "max_tokens";
    }
    return "error";
  }
  return "end_turn";
}

function processResultEvent(state: ChatState, event: ResultEvent): ChatState {
  const turn = findCurrentTurn(state);
  if (!turn) return state;

  const now = Date.now();
  const duration = now - turn.startedAt;
  const finishReason = deriveFinishReason(event);

  const newParts: AssistantPart[] = [...turn.assistantParts];
  if (event.result && !event.is_error) {
    const lastPart = newParts[newParts.length - 1];
    if (lastPart && lastPart.type === "text") {
      if (!lastPart.text.endsWith(event.result)) {
        newParts[newParts.length - 1] = {
          ...lastPart,
          text: lastPart.text + event.result,
        };
      }
    }
  }

  // Only update cost here — token counts come from the authoritative `usage` event
  // that follows each `result` event (with correct running totals from the backend).
  const contextUsage =
    event.cost && state.contextUsage
      ? {
          ...state.contextUsage,
          totalCost: (state.contextUsage.totalCost ?? 0) + event.cost.total_usd,
        }
      : state.contextUsage;

  return {
    ...state,
    turns: updateTurn(state.turns, turn.id, (t) => ({
      ...t,
      assistantParts: newParts,
      status: event.is_error ? "error" : "completed",
      duration,
      finishReason,
    })),
    currentTurnId: null,
    isStreaming: false,
    contextUsage,
    session: state.session
      ? {
          ...state.session,
          status: event.is_error ? "error" : "idle",
        }
      : null,
  };
}

function processUsageEvent(state: ChatState, event: UsageEvent): ChatState {
  return {
    ...state,
    contextUsage: {
      inputTokens: event.total_input,
      outputTokens: event.total_output,
      cacheReadTokens: event.cache_read_input_tokens ?? state.contextUsage?.cacheReadTokens,
      cacheWriteTokens: event.cache_creation_input_tokens ?? state.contextUsage?.cacheWriteTokens,
      totalCost: state.contextUsage?.totalCost,
    },
  };
}

function processCompactEvent(state: ChatState, event: CompactEvent): ChatState {
  const compactedTurn = {
    id: `compact-${Date.now()}`,
    userMessage: "[conversation compacted]",
    assistantParts: [
      {
        type: "text" as const,
        id: nextPartId(),
        text: event.summary,
      },
    ],
    status: "completed" as const,
    duration: null,
    startedAt: Date.now(),
    finishReason: "end_turn" as const,
  };

  return {
    ...state,
    turns: [compactedTurn],
    currentTurnId: null,
    isStreaming: false,
    contextUsage: {
      inputTokens: event.total_input,
      outputTokens: event.total_output,
      cacheReadTokens: undefined,
      cacheWriteTokens: undefined,
      totalCost: state.contextUsage?.totalCost,
    },
  };
}
