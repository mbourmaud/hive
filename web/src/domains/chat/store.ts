import type { StateCreator } from "zustand";
import { chatReducer, initialChatState } from "./reducer";
import type {
  ChatAction,
  ChatState,
  ImageAttachment,
  ProjectChatSnapshot,
  QueuedMessage,
} from "./types";

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
    messageQueue: slice.messageQueue,
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
  restoreChat: (snapshot: ProjectChatSnapshot) => void;
  enqueueMessage: (text: string, images?: ImageAttachment[]) => void;
  cancelQueuedMessage: (messageId: string) => void;
  clearQueue: () => void;
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

  restoreChat: (snapshot) =>
    set({
      ...snapshot.chatState,
      activeSessionId: snapshot.activeSessionId,
      promptDraft: snapshot.promptDraft,
      isCreatingSession: false,
    }),

  enqueueMessage: (text, images) => {
    const message: QueuedMessage = {
      id: `queued-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      text,
      images,
      queuedAt: Date.now(),
    };
    const next = chatReducer(extractChatState(get()), { type: "ENQUEUE_MESSAGE", message });
    set(next);
  },

  cancelQueuedMessage: (messageId) => {
    const next = chatReducer(extractChatState(get()), { type: "CANCEL_QUEUED_MESSAGE", messageId });
    set(next);
  },

  clearQueue: () => {
    const next = chatReducer(extractChatState(get()), { type: "CLEAR_QUEUE" });
    set(next);
  },
});
