import { initialChatState } from "./reducer-utils";
import type { ChatState, ProjectChatSnapshot } from "./types";

// ── Constants ──────────────────────────────────────────────────────────────

const STORAGE_KEY = "hive-project-cache";
const LRU_MAX = 20;

// ── In-memory cache ─────────────────────────────────────────────────────────

const cache = new Map<string, ProjectChatSnapshot>();

// ── Public API ──────────────────────────────────────────────────────────────

export function defaultProjectSnapshot(): ProjectChatSnapshot {
  return {
    chatState: { ...initialChatState },
    activeSessionId: null,
    promptDraft: "",
    selectedModel: null,
    effort: "medium",
    chatMode: "code",
    wasStreaming: false,
    streamingSessionId: null,
    streamingTurnId: null,
    messageQueue: [],
  };
}

/**
 * Build a snapshot from the current app store state.
 * Accepts a pick of the fields we need — avoids importing the full AppStore type.
 */
interface SnapshotSource {
  session: ChatState["session"];
  turns: ChatState["turns"];
  currentTurnId: ChatState["currentTurnId"];
  isStreaming: ChatState["isStreaming"];
  lastEventAt: ChatState["lastEventAt"];
  isStale: ChatState["isStale"];
  error: ChatState["error"];
  contextUsage: ChatState["contextUsage"];
  messageQueue: ChatState["messageQueue"];
  activeSessionId: string | null;
  promptDraft: string;
  selectedModel: string | null;
  effort: "low" | "medium" | "high";
  chatMode: "code" | "hive-plan" | "plan";
}

export function buildSnapshot(state: SnapshotSource): ProjectChatSnapshot {
  const chatState: ChatState = {
    session: state.session,
    turns: state.turns,
    currentTurnId: state.currentTurnId,
    isStreaming: state.isStreaming,
    lastEventAt: state.lastEventAt,
    isStale: state.isStale,
    error: state.error,
    contextUsage: state.contextUsage,
    messageQueue: state.messageQueue,
  };
  return {
    chatState,
    activeSessionId: state.activeSessionId,
    promptDraft: state.promptDraft,
    selectedModel: state.selectedModel,
    effort: state.effort,
    chatMode: state.chatMode,
    wasStreaming: state.isStreaming,
    streamingSessionId: state.isStreaming ? (state.session?.id ?? null) : null,
    streamingTurnId: state.isStreaming ? state.currentTurnId : null,
    messageQueue: state.messageQueue,
  };
}

export function saveProjectSnapshot(projectPath: string, snapshot: ProjectChatSnapshot): void {
  // LRU eviction
  if (cache.size >= LRU_MAX && !cache.has(projectPath)) {
    const firstKey = cache.keys().next().value;
    if (firstKey !== undefined) cache.delete(firstKey);
  }
  cache.set(projectPath, snapshot);
}

export function loadProjectSnapshot(projectPath: string): ProjectChatSnapshot | null {
  return cache.get(projectPath) ?? null;
}

export function clearProjectSnapshot(projectPath: string): void {
  cache.delete(projectPath);
}

// ── localStorage persistence ────────────────────────────────────────────────

/** Strip volatile streaming state before persisting. */
function stripStreamingState(snap: ProjectChatSnapshot): ProjectChatSnapshot {
  return {
    ...snap,
    chatState: {
      ...snap.chatState,
      isStreaming: false,
      isStale: false,
      lastEventAt: null,
      messageQueue: [],
    },
    wasStreaming: false,
    streamingSessionId: null,
    streamingTurnId: null,
    messageQueue: [],
  };
}

export function persistCache(): void {
  try {
    const entries: Array<[string, ProjectChatSnapshot]> = [];
    for (const [key, val] of cache) {
      entries.push([key, stripStreamingState(val)]);
    }
    localStorage.setItem(STORAGE_KEY, JSON.stringify(entries));
  } catch {
    // quota exceeded or serialization error — silently skip
  }
}

export function hydrateCache(): void {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return;
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return;
    cache.clear();
    for (const entry of parsed) {
      if (!Array.isArray(entry) || entry.length !== 2) continue;
      const [key, val] = entry as [unknown, unknown];
      if (typeof key !== "string" || !isSnapshotShape(val)) continue;
      cache.set(key, val);
    }
  } catch {
    // corrupted storage — start fresh
  }
}

function isSnapshotShape(v: unknown): v is ProjectChatSnapshot {
  if (typeof v !== "object" || v === null) return false;
  const obj = v as Record<string, unknown>;
  return (
    typeof obj.chatState === "object" &&
    obj.chatState !== null &&
    "activeSessionId" in obj &&
    "promptDraft" in obj
  );
}
