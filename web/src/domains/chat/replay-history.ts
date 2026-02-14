import { processStreamEvent } from "./event-processors";
import { initialChatState, updateTurn } from "./reducer-utils";
import type { ChatSession, ChatState, StreamEvent, UserEvent, UserTextBlock } from "./types";

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
export function replayHistory(
  session: ChatSession,
  events: StreamEvent[],
  tokenCounts?: { inputTokens: number; outputTokens: number },
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

    if (event.type === "result" || event.type === "compact.completed") {
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

  // Apply persisted token counts from meta.json (authoritative source).
  // The events.ndjson may miss the final usage events if persist_handle
  // was aborted before they were written, so prefer the meta values.
  if (tokenCounts && tokenCounts.inputTokens > 0) {
    replayed = {
      ...replayed,
      contextUsage: {
        inputTokens: tokenCounts.inputTokens,
        outputTokens: tokenCounts.outputTokens,
        cacheReadTokens: replayed.contextUsage?.cacheReadTokens,
        cacheWriteTokens: replayed.contextUsage?.cacheWriteTokens,
        totalCost: replayed.contextUsage?.totalCost,
      },
    };
  }

  return replayed;
}
