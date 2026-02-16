import type { StateCreator } from "zustand";
import { chatReducer, initialChatState } from "./reducer";
import type { ChatAction, ChatState } from "./types";

function extractChatState(slice: ChatSlice): ChatState {
  return {
    session: slice.session,
    turns: slice.turns,
    currentTurnId: slice.currentTurnId,
    isStreaming: slice.isStreaming,
    lastEventAt: slice.lastEventAt,
    isStale: slice.isStale,
    error: slice.error,
    contextUsage: slice.contextUsage,
  };
}

export interface ChatSlice extends ChatState {
  activeSessionId: string | null;
  promptDraft: string;
  isCreatingSession: boolean;

  setActiveSession: (id: string | null) => void;
  setPromptDraft: (text: string) => void;
  setCreatingSession: (v: boolean) => void;
  dispatchChat: (action: ChatAction) => void;
  resetChat: () => void;
}

export const createChatSlice: StateCreator<ChatSlice, [], [], ChatSlice> = (set, get) => ({
  ...initialChatState,

  activeSessionId: null,
  promptDraft: "",
  isCreatingSession: false,

  setActiveSession: (id) => set({ activeSessionId: id }),
  setPromptDraft: (text) => set({ promptDraft: text }),
  setCreatingSession: (v) => set({ isCreatingSession: v }),

  dispatchChat: (action) => {
    const next = chatReducer(extractChatState(get()), action);
    set(next);
  },

  resetChat: () => set({ ...initialChatState }),
});
