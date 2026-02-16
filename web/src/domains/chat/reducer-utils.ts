import type { ChatState, ChatTurn } from "./types";

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
  messageQueue: [],
};

// ── Part ID counter ─────────────────────────────────────────────────────────

let partCounter = 0;

export function nextPartId(): string {
  return `part-${++partCounter}`;
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/** Extract topic from thinking text — looks for **topic** on the first line */
export function extractThinkingTopic(text: string): string | undefined {
  const firstLine = text.split("\n")[0] ?? "";
  const match = firstLine.match(/^\*\*(.+?)\*\*/);
  return match?.[1];
}

export function findCurrentTurn(state: ChatState): ChatTurn | undefined {
  if (!state.currentTurnId) return undefined;
  return state.turns.find((t) => t.id === state.currentTurnId);
}

export function updateTurn(
  turns: ChatTurn[],
  turnId: string,
  updater: (turn: ChatTurn) => ChatTurn,
): ChatTurn[] {
  return turns.map((t) => (t.id === turnId ? updater(t) : t));
}
