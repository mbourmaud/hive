import { useCallback, useMemo } from "react";
import type { SessionEntry } from "@/domains/chat/components/sessions-modal";
import { executeSlashCommand, type SlashCommandContext } from "@/domains/chat/slash-commands";
import type { ChatAction, ChatSession, ChatTurn, ImageAttachment } from "@/domains/chat/types";
import type { RightSidebarTab } from "@/domains/monitor/store";
import type { DroneInfo } from "@/domains/monitor/types";
import { useAppStore } from "@/store";

interface UseSendHandlerOptions {
  sessions: SessionEntry[];
  activeSessionId: string | null;
  chatSession: ChatSession | null;
  turns: ChatTurn[];
  selectedModel: string | null;
  drones: DroneInfo[];
  addSession: (title: string) => Promise<ChatSession | null>;
  sendMessage: (
    message: string,
    sessionOverride?: ChatSession,
    model?: string,
    images?: ImageAttachment[],
  ) => Promise<void>;
  resetSession: () => void;
  handleNewSession: () => void;
  handleDeleteSession: (id: string) => void;
  renameSessionMutate: (vars: { id: string; title: string }) => void;
  dispatchChat: (action: ChatAction) => void;
  toast: (message: string, variant: "success" | "error" | "info") => void;
}

export function useSendHandler({
  sessions,
  activeSessionId,
  chatSession,
  turns,
  selectedModel,
  drones,
  addSession,
  sendMessage,
  resetSession,
  handleNewSession,
  handleDeleteSession,
  renameSessionMutate,
  dispatchChat,
  toast,
}: UseSendHandlerOptions) {
  const setSelectedModel = useAppStore((s) => s.setSelectedModel);
  const setSessionsModalOpen = useAppStore((s) => s.setSessionsModalOpen);
  const rightSidebarCollapsed = useAppStore((s) => s.rightSidebarCollapsed);
  const openRightSidebar = useAppStore((s) => s.openRightSidebar);

  const setActiveSessionId = useAppStore((s) => s.setActiveSession);
  const reloadSession = useCallback(
    (id: string) => {
      dispatchChat({ type: "SESSION_RESET" });
      setActiveSessionId(null);
      queueMicrotask(() => setActiveSessionId(id));
    },
    [dispatchChat, setActiveSessionId],
  );

  const commandCtx: SlashCommandContext = useMemo(
    () => ({
      toast,
      dispatchChat,
      selectedModel,
      setSelectedModel,
      setSessionsModalOpen,
      handleNewSession,
      resetSession,
      drones,
      rightSidebarCollapsed,
      openRightSidebar: openRightSidebar as (tab: RightSidebarTab) => void,
      activeSessionId,
      reloadSession,
    }),
    [
      toast,
      dispatchChat,
      selectedModel,
      setSelectedModel,
      setSessionsModalOpen,
      handleNewSession,
      resetSession,
      drones,
      rightSidebarCollapsed,
      openRightSidebar,
      activeSessionId,
      reloadSession,
    ],
  );

  const handleSend = useCallback(
    async (message: string, images?: ImageAttachment[]) => {
      if (message.startsWith("/")) {
        const handled = await executeSlashCommand(message, commandCtx);
        if (handled) return;
      }

      // If streaming, enqueue the message instead of sending immediately
      const store = useAppStore.getState();
      if (store.isStreaming) {
        store.enqueueMessage(message, images);
        return;
      }

      let session = chatSession;

      if (!session) {
        const title = message.slice(0, 50) || `Session ${sessions.length + 1}`;
        useAppStore.getState().setCreatingSession(true);
        try {
          session = await addSession(title);
        } finally {
          useAppStore.getState().setCreatingSession(false);
        }
        if (!session) return;
      }

      if (turns.length === 0 && activeSessionId) {
        const newTitle = message.slice(0, 50);
        if (newTitle) {
          renameSessionMutate({ id: activeSessionId, title: newTitle });
        }
      }

      await sendMessage(message, session, selectedModel ?? undefined, images);
    },
    [
      commandCtx,
      chatSession,
      turns.length,
      activeSessionId,
      addSession,
      sessions.length,
      sendMessage,
      selectedModel,
      renameSessionMutate,
    ],
  );

  const handleClearConversation = useCallback(() => {
    if (!activeSessionId) return;
    if (window.confirm("Clear this conversation?")) {
      resetSession();
      toast("Conversation cleared", "info");
    }
  }, [activeSessionId, resetSession, toast]);

  const handleDeleteCurrentSession = useCallback(() => {
    if (!activeSessionId || !window.confirm("Delete this session? This cannot be undone.")) return;
    handleDeleteSession(activeSessionId);
  }, [activeSessionId, handleDeleteSession]);

  return { handleSend, handleClearConversation, handleDeleteCurrentSession };
}
