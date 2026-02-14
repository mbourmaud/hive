import type {
  AssistantEvent,
  AssistantPart,
  ChatAction,
  ChatSession,
  ChatState,
  ChatTurn,
  FinishReason,
  ResultEvent,
  StreamEvent,
  SystemEvent,
  ThinkingPart,
  UsageEvent,
  UserEvent,
  UserTextBlock,
} from "./types";

// ── Initial state ───────────────────────────────────────────────────────────

export const initialChatState: ChatState = {
  session: null,
  turns: [],
  currentTurnId: null,
  isStreaming: false,
  lastEventAt: null,
  isStale: false,
  error: null,
  contextUsage: null,
};

// ── Helpers ─────────────────────────────────────────────────────────────────

let partCounter = 0;

function nextPartId(): string {
  return `part-${++partCounter}`;
}

/** Extract topic from thinking text — looks for **topic** on the first line */
function extractThinkingTopic(text: string): string | undefined {
  const firstLine = text.split("\n")[0] ?? "";
  const match = firstLine.match(/^\*\*(.+?)\*\*/);
  return match?.[1];
}

function findCurrentTurn(state: ChatState): ChatTurn | undefined {
  if (!state.currentTurnId) return undefined;
  return state.turns.find((t) => t.id === state.currentTurnId);
}

function updateTurn(
  turns: ChatTurn[],
  turnId: string,
  updater: (turn: ChatTurn) => ChatTurn,
): ChatTurn[] {
  return turns.map((t) => (t.id === turnId ? updater(t) : t));
}

// ── Process a single stream event into state ────────────────────────────────

function processStreamEvent(state: ChatState, event: StreamEvent): ChatState {
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
    default:
      return base;
  }
}

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

  const contextUsage = event.usage
    ? {
        inputTokens: (state.contextUsage?.inputTokens ?? 0) + event.usage.input_tokens,
        outputTokens: (state.contextUsage?.outputTokens ?? 0) + event.usage.output_tokens,
        cacheReadTokens: state.contextUsage?.cacheReadTokens,
        cacheWriteTokens: state.contextUsage?.cacheWriteTokens,
        totalCost: event.cost
          ? (state.contextUsage?.totalCost ?? 0) + event.cost.total_usd
          : state.contextUsage?.totalCost,
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

// ── Main reducer (pure function, no React dependency) ────────────────────────

export function chatReducer(state: ChatState, action: ChatAction): ChatState {
  switch (action.type) {
    case "SESSION_CREATED":
      return {
        ...state,
        session: action.session,
        turns: [],
        currentTurnId: null,
        isStreaming: false,
        error: null,
      };

    case "SESSION_RESET":
      return initialChatState;

    case "TURN_STARTED": {
      const newTurn: ChatTurn = {
        id: action.turnId,
        userMessage: action.userMessage,
        assistantParts: [],
        status: "pending",
        duration: null,
        startedAt: Date.now(),
        model: action.model,
      };
      return {
        ...state,
        turns: [...state.turns, newTurn],
        currentTurnId: action.turnId,
        isStreaming: true,
        error: null,
      };
    }

    case "STREAM_EVENT":
      return processStreamEvent(state, action.event);

    case "STREAM_EVENT_BATCH": {
      let current = state;
      for (const event of action.events) {
        current = processStreamEvent(current, event);
      }
      return current;
    }

    case "TURN_COMPLETED": {
      const isCurrentTurn = state.currentTurnId === action.turnId;
      const now = Date.now();
      return {
        ...state,
        turns: updateTurn(state.turns, action.turnId, (t) => ({
          ...t,
          status: "completed",
          duration: now - t.startedAt,
        })),
        currentTurnId: isCurrentTurn ? null : state.currentTurnId,
        isStreaming: isCurrentTurn ? false : state.isStreaming,
      };
    }

    case "TURN_ERROR": {
      const isCurrentTurn = state.currentTurnId === action.turnId;
      const now = Date.now();
      return {
        ...state,
        turns: updateTurn(state.turns, action.turnId, (t) => ({
          ...t,
          status: "error",
          duration: now - t.startedAt,
          finishReason: "error" as const,
        })),
        currentTurnId: isCurrentTurn ? null : state.currentTurnId,
        isStreaming: isCurrentTurn ? false : state.isStreaming,
        error: action.error,
      };
    }

    case "MARK_STALE":
      return { ...state, isStale: true };

    case "CONNECTION_ERROR":
      return {
        ...state,
        error: action.error,
        isStreaming: false,
      };

    case "DRONE_LAUNCHED": {
      const turnId = `drone-${action.droneName}-${Date.now()}`;
      const droneTurn: ChatTurn = {
        id: turnId,
        userMessage: action.prompt,
        assistantParts: [],
        status: "completed",
        duration: null,
        startedAt: Date.now(),
        droneName: action.droneName,
      };
      return {
        ...state,
        turns: [...state.turns, droneTurn],
      };
    }

    case "REPLAY_HISTORY":
      return replayHistory(action.session, action.events);
  }
}

// ── Replay helpers ─────────────────────────────────────────────────────────

/** Extract user text from a UserEvent (returns null if it only contains tool_results). */
function extractUserText(event: UserEvent): string | null {
  const textBlocks = event.message.content.filter((b): b is UserTextBlock => b.type === "text");
  if (textBlocks.length === 0) return null;
  return textBlocks.map((b) => b.text).join("");
}

/** Create a new replay turn and attach it to the state. */
function ensureTurnForEvent(
  state: ChatState,
  turnCounter: { value: number },
  userMessage: string,
): ChatState {
  const turnId = `replay-${++turnCounter.value}`;
  return {
    ...state,
    turns: [
      ...state.turns,
      {
        id: turnId,
        userMessage,
        assistantParts: [],
        status: "pending",
        duration: null,
        startedAt: Date.now(),
      },
    ],
    currentTurnId: turnId,
    isStreaming: true,
  };
}

/** Replay a full history of events into a ChatState, reconstructing turns. */
function replayHistory(
  session: ChatSession,
  events: StreamEvent[],
): ChatState {
  let replayed: ChatState = {
    ...initialChatState,
    session,
  };
  const turnCounter = { value: 0 };

  for (const event of events) {
    // User text event (not tool_result) → start a new turn
    if (event.type === "user" && replayed.currentTurnId === null) {
      const userText = extractUserText(event);
      if (userText) {
        replayed = ensureTurnForEvent(replayed, turnCounter, userText);
        continue;
      }
    }

    // Assistant event without a current turn → create turn (fallback for
    // sessions where user event was not persisted)
    if (replayed.currentTurnId === null && event.type === "assistant") {
      replayed = ensureTurnForEvent(replayed, turnCounter, "");
    }

    replayed = processStreamEvent(replayed, event);

    if (event.type === "result") {
      replayed = { ...replayed, currentTurnId: null, isStreaming: false };
    }
  }

  // Mark any still-streaming turn as completed (history is done)
  if (replayed.currentTurnId !== null) {
    replayed = {
      ...replayed,
      turns: updateTurn(replayed.turns, replayed.currentTurnId, (t) => ({
        ...t,
        status: "completed",
        duration: 0,
      })),
      currentTurnId: null,
      isStreaming: false,
    };
  }

  return replayed;
}
