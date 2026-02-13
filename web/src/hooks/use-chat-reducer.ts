import { useReducer } from "react";
import type {
  ChatState,
  ChatAction,
  ChatTurn,
  AssistantPart,
  StreamEvent,
  AssistantEvent,
  UserEvent,
  ResultEvent,
  SystemEvent,
} from "@/types/chat";

// ── Initial state ───────────────────────────────────────────────────────────

export const initialChatState: ChatState = {
  session: null,
  turns: [],
  currentTurnId: null,
  isStreaming: false,
  lastEventAt: null,
  isStale: false,
  error: null,
};

// ── Helpers ─────────────────────────────────────────────────────────────────

let partCounter = 0;

function nextPartId(): string {
  return `part-${++partCounter}`;
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
  }
}

function processSystemEvent(state: ChatState, event: SystemEvent): ChatState {
  if (event.subtype === "init") {
    return {
      ...state,
      session: state.session
        ? { ...state.session, id: event.session_id, status: "busy" }
        : {
            id: event.session_id,
            status: "busy",
            cwd: "",
            createdAt: new Date().toISOString(),
          },
      isStreaming: true,
    };
  }
  // heartbeat — just update lastEventAt (already done in base)
  return state;
}

function processAssistantEvent(
  state: ChatState,
  event: AssistantEvent,
): ChatState {
  const turn = findCurrentTurn(state);
  if (!turn) return state;

  const newParts: AssistantPart[] = [...turn.assistantParts];

  for (const block of event.message.content) {
    switch (block.type) {
      case "text": {
        // Coalesce: if last part is text, append to it
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
      case "tool_use": {
        // Mark any previously "pending" or "running" tool as running
        // (only one tool runs at a time in practice)
        newParts.push({
          type: "tool_use",
          id: block.id,
          name: block.name,
          input: block.input,
          status: "running",
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
      // Find matching tool_use and mark completed/error
      const toolIdx = newParts.findIndex(
        (p) => p.type === "tool_use" && p.id === block.tool_use_id,
      );
      if (toolIdx !== -1) {
        const toolPart = newParts[toolIdx];
        if (toolPart && toolPart.type === "tool_use") {
          newParts[toolIdx] = {
            ...toolPart,
            status: block.is_error ? "error" : "completed",
          };
        }
      }

      // Add tool result part
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

function processResultEvent(state: ChatState, event: ResultEvent): ChatState {
  const turn = findCurrentTurn(state);
  if (!turn) return state;

  const now = Date.now();
  const duration = now - turn.startedAt;

  // If result contains final text, append it
  const newParts: AssistantPart[] = [...turn.assistantParts];
  if (event.result && !event.is_error) {
    const lastPart = newParts[newParts.length - 1];
    if (lastPart && lastPart.type === "text") {
      // Only append if result text differs from what we already have
      if (!lastPart.text.endsWith(event.result)) {
        newParts[newParts.length - 1] = {
          ...lastPart,
          text: lastPart.text + event.result,
        };
      }
    }
  }

  return {
    ...state,
    turns: updateTurn(state.turns, turn.id, (t) => ({
      ...t,
      assistantParts: newParts,
      status: event.is_error ? "error" : "completed",
      duration,
    })),
    currentTurnId: null,
    isStreaming: false,
    session: state.session
      ? {
          ...state.session,
          status: event.is_error ? "error" : "idle",
        }
      : null,
  };
}

// ── Main reducer ────────────────────────────────────────────────────────────

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
      return {
        ...state,
        turns: updateTurn(state.turns, action.turnId, (t) => ({
          ...t,
          status: "error",
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

    case "REPLAY_HISTORY": {
      let replayed: ChatState = {
        ...initialChatState,
        session: action.session,
      };
      let turnCounter = 0;

      for (const event of action.events) {
        // Create a turn if none exists when we receive assistant content
        if (replayed.currentTurnId === null && event.type === "assistant") {
          const turnId = `replay-${++turnCounter}`;
          replayed = {
            ...replayed,
            turns: [
              ...replayed.turns,
              {
                id: turnId,
                userMessage: "",
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

        replayed = processStreamEvent(replayed, event);

        // After a result event, clear the turn so next assistant starts fresh
        if (event.type === "result") {
          replayed = { ...replayed, currentTurnId: null, isStreaming: false };
        }
      }
      return replayed;
    }
  }
}

// ── Hook ────────────────────────────────────────────────────────────────────

export function useChatReducer() {
  return useReducer(chatReducer, initialChatState);
}
