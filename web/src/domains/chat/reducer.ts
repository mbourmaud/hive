import { processStreamEvent } from "./event-processors";
import { initialChatState, updateTurn } from "./reducer-utils";
import { replayHistory } from "./replay-history";
import type { ChatAction, ChatState, ChatTurn } from "./types";

// Re-export so existing consumers don't need import changes
export { initialChatState } from "./reducer-utils";

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
        messageQueue: [],
      };

    case "SESSION_RESET":
      return initialChatState;

    case "TURN_STARTED": {
      const newTurn: ChatTurn = {
        id: action.turnId,
        userMessage: action.userMessage,
        images: action.images,
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
      return replayHistory(action.session, action.events, action.tokenCounts);

    case "ENQUEUE_MESSAGE":
      return { ...state, messageQueue: [...state.messageQueue, action.message] };

    case "DEQUEUE_MESSAGE":
      return { ...state, messageQueue: state.messageQueue.slice(1) };

    case "CANCEL_QUEUED_MESSAGE":
      return {
        ...state,
        messageQueue: state.messageQueue.filter((m) => m.id !== action.messageId),
      };

    case "CLEAR_QUEUE":
      return { ...state, messageQueue: [] };
  }
}
