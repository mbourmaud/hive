import { useCallback, useEffect } from "react";
import { DronePanel } from "@/app/layout/drone-panel";
import { IconBar } from "@/app/layout/icon-bar";
import { MobileNav } from "@/app/layout/mobile-nav";
import type { SessionEntry } from "@/app/layout/session-panel";
import { SessionPanel } from "@/app/layout/session-panel";
import { ChatLayout } from "@/domains/chat/components/chat-layout";
import { useDeleteSession, useRenameSession } from "@/domains/chat/mutations";
import { useSessionsQuery } from "@/domains/chat/queries";
import type { ImageAttachment } from "@/domains/chat/types";
import { useChat } from "@/domains/chat/use-chat-stream";
import { useProjectsSSE } from "@/domains/monitor/queries";
import { AuthSetup } from "@/domains/settings/components/auth-setup";
import { CommandPalette } from "@/domains/settings/components/command-palette";
import { SettingsDialog } from "@/domains/settings/components/settings-dialog";
import { useAuthStatusQuery, useModelsQuery } from "@/domains/settings/queries";
import { useKeyboardShortcuts } from "@/shared/hooks/use-keyboard-shortcuts";
import { useIsMobile } from "@/shared/hooks/use-mobile";
import { useTheme } from "@/shared/theme/use-theme";
import { ToastProvider, useToast } from "@/shared/ui/toast";
import { useAppStore } from "@/store";

// ── Session handoff cache ────────────────────────────────────────────────────

interface SessionSnapshot {
  scrollTop: number;
  promptText: string;
}

const SESSION_CACHE_MAX = 40;
export const sessionCache = new Map<string, SessionSnapshot>();

export function cacheSession(id: string, snapshot: SessionSnapshot) {
  if (sessionCache.size >= SESSION_CACHE_MAX) {
    const firstKey = sessionCache.keys().next().value;
    if (firstKey !== undefined) sessionCache.delete(firstKey);
  }
  sessionCache.set(id, snapshot);
}

// ── App ──────────────────────────────────────────────────────────────────────

export default function App() {
  return (
    <ToastProvider>
      <AppInner />
    </ToastProvider>
  );
}

function AppInner() {
  // ── Zustand store ─────────────────────────────────────────────────────────
  const selectedProject = useAppStore((s) => s.selectedProject);
  const setSelectedProject = useAppStore((s) => s.setSelectedProject);
  const selectedModel = useAppStore((s) => s.selectedModel);
  const setSelectedModel = useAppStore((s) => s.setSelectedModel);
  const dronePanelCollapsed = useAppStore((s) => s.dronePanelCollapsed);
  const toggleDronePanel = useAppStore((s) => s.toggleDronePanel);
  const settingsOpen = useAppStore((s) => s.settingsOpen);
  const setSettingsOpen = useAppStore((s) => s.setSettingsOpen);
  const commandPaletteOpen = useAppStore((s) => s.commandPaletteOpen);
  const setCommandPaletteOpen = useAppStore((s) => s.setCommandPaletteOpen);
  const activeSessionId = useAppStore((s) => s.activeSessionId);
  const setActiveSessionId = useAppStore((s) => s.setActiveSession);

  // Chat state from Zustand (populated by useChat's dispatchChat)
  const turns = useAppStore((s) => s.turns);
  const isStreaming = useAppStore((s) => s.isStreaming);
  const chatError = useAppStore((s) => s.error);
  const currentTurnId = useAppStore((s) => s.currentTurnId);
  const chatSession = useAppStore((s) => s.session);
  const contextUsage = useAppStore((s) => s.contextUsage);

  // ── TanStack Query ────────────────────────────────────────────────────────
  const { data: projects = [], connectionStatus } = useProjectsSSE();
  const { data: authStatus, isLoading: authLoading } = useAuthStatusQuery();
  const { data: models = [] } = useModelsQuery();
  const { data: allSessions = [] } = useSessionsQuery();

  // ── Mutations ─────────────────────────────────────────────────────────────
  const renameSessionMutation = useRenameSession();
  const deleteSessionMutation = useDeleteSession();

  // ── Chat streaming (dispatches to Zustand) ────────────────────────────────
  const { sendMessage, abort, createSession, resetSession } = useChat();

  // ── Other hooks ───────────────────────────────────────────────────────────
  const isMobile = useIsMobile();
  const { toast } = useToast();
  const { toggleTheme } = useTheme();

  // ── Default model selection ───────────────────────────────────────────────
  const defaultModel = (() => {
    if (models.length === 0) return null;
    const priority = ["opus", "sonnet", "haiku"];
    for (const tier of priority) {
      const match = models.find((m) => m.id.includes(tier));
      if (match) return match.id;
    }
    return models[0]?.id ?? null;
  })();

  useEffect(() => {
    if (!selectedModel && defaultModel) {
      setSelectedModel(defaultModel);
    }
  }, [selectedModel, defaultModel, setSelectedModel]);

  // ── Sessions filtered by project ──────────────────────────────────────────
  const filtered = selectedProject
    ? allSessions.filter((s) => s.cwd === selectedProject || s.cwd.startsWith(selectedProject))
    : allSessions;
  const sessions: SessionEntry[] = filtered.map((s) => ({
    id: s.id,
    title: s.title,
    createdAt: s.created_at,
    status: s.status,
    cwd: s.cwd,
  }));

  // ── Session actions ───────────────────────────────────────────────────────
  const addSession = useCallback(
    async (title: string) => {
      try {
        const cwd = selectedProject ?? projects[0]?.path ?? "/";
        const session = await createSession(cwd, selectedModel ?? undefined);
        setActiveSessionId(session.id);
        renameSessionMutation.mutate({ id: session.id, title });
        return session;
      } catch {
        return null;
      }
    },
    [
      selectedProject,
      projects,
      createSession,
      selectedModel,
      setActiveSessionId,
      renameSessionMutation,
    ],
  );

  const handleNewSession = useCallback(() => {
    addSession(`Session ${sessions.length + 1}`);
  }, [addSession, sessions.length]);

  const handleSelectSession = useCallback(
    (id: string) => {
      setActiveSessionId(id);
    },
    [setActiveSessionId],
  );

  const handleRenameSession = useCallback(
    (id: string, title: string) => {
      renameSessionMutation.mutate({ id, title });
      toast("Session renamed", "success");
    },
    [renameSessionMutation, toast],
  );

  const handleDeleteSession = useCallback(
    (id: string) => {
      deleteSessionMutation.mutate(id);
      if (activeSessionId === id) {
        setActiveSessionId(null);
      }
      toast("Session deleted", "success");
    },
    [deleteSessionMutation, activeSessionId, setActiveSessionId, toast],
  );

  const handleSend = useCallback(
    async (message: string, images?: ImageAttachment[]) => {
      // ── Slash command interception ──────────────────────────────────────
      if (message.startsWith("/")) {
        const parts = message.slice(1).split(/\s+/);
        const cmd = parts[0]?.toLowerCase();

        switch (cmd) {
          case "new":
            handleNewSession();
            return;
          case "clear":
            resetSession();
            toast("Conversation cleared", "info");
            return;
          case "compact":
            toast("Compact is not yet implemented", "info");
            return;
          case "model": {
            const modelName = parts[1];
            if (modelName) {
              setSelectedModel(modelName);
              toast(`Model switched to ${modelName}`, "success");
            } else {
              toast("Usage: /model <name> (e.g. /model sonnet)", "info");
            }
            return;
          }
          case "steps":
            toast("Toggle steps via the steps button on each turn", "info");
            return;
          case "undo":
            toast("Undo is not yet implemented", "info");
            return;
          case "help":
            toast("Commands: /new, /clear, /compact, /model <name>, /steps, /undo, /help", "info");
            return;
        }
      }

      // ── Normal message send ────────────────────────────────────────────
      let session = chatSession;

      if (!session) {
        const title = message.slice(0, 50) || `Session ${sessions.length + 1}`;
        session = await addSession(title);
        if (!session) return;
      }

      if (turns.length === 0 && activeSessionId) {
        const newTitle = message.slice(0, 50);
        if (newTitle) {
          renameSessionMutation.mutate({ id: activeSessionId, title: newTitle });
        }
      }

      await sendMessage(message, session, selectedModel ?? undefined, images);
    },
    [
      chatSession,
      turns.length,
      activeSessionId,
      addSession,
      sessions.length,
      sendMessage,
      selectedModel,
      resetSession,
      toast,
      renameSessionMutation,
      setSelectedModel,
      handleNewSession,
    ],
  );

  // ── Keyboard shortcuts ────────────────────────────────────────────────────
  const handleClearConversation = useCallback(() => {
    if (!activeSessionId) return;
    if (window.confirm("Clear this conversation?")) {
      resetSession();
      toast("Conversation cleared", "info");
    }
  }, [activeSessionId, resetSession, toast]);

  const handleDeleteCurrentSession = useCallback(() => {
    if (!activeSessionId) return;
    if (window.confirm("Delete this session? This cannot be undone.")) {
      handleDeleteSession(activeSessionId);
    }
  }, [activeSessionId, handleDeleteSession]);

  useKeyboardShortcuts({
    onNewSession: handleNewSession,
    onOpenSettings: () => setSettingsOpen(true),
    onOpenCommandPalette: () => setCommandPaletteOpen(true),
    onDeleteSession: handleDeleteCurrentSession,
    onAbort: abort,
    onToggleDronePanel: toggleDronePanel,
    isStreaming,
  });

  // ── Project auto-select ───────────────────────────────────────────────────
  useEffect(() => {
    if (!selectedProject && projects[0]) {
      setSelectedProject(projects[0].path);
    }
  }, [projects, selectedProject, setSelectedProject]);

  // Reset session when switching projects
  useEffect(() => {
    if (!activeSessionId || !selectedProject) return;
    const sessionBelongs = sessions.some((s) => s.id === activeSessionId);
    if (!sessionBelongs) {
      setActiveSessionId(null);
      resetSession();
    }
  }, [selectedProject, activeSessionId, sessions, resetSession, setActiveSessionId]);

  const activeProject = projects.find((p) => p.path === selectedProject) ?? null;
  const drones = activeProject?.drones ?? [];

  // ── Mobile ────────────────────────────────────────────────────────────────
  if (isMobile) {
    return (
      <MobileNav
        projects={projects}
        selectedProject={selectedProject}
        onSelectProject={setSelectedProject}
        selectedDrone={null}
        onSelectDrone={() => {}}
        connectionStatus={connectionStatus}
        isMock={connectionStatus === "mock"}
      />
    );
  }

  // ── Desktop ───────────────────────────────────────────────────────────────
  return (
    <div data-component="app-root" className="flex h-screen overflow-hidden">
      <IconBar
        projects={projects}
        activeProject={selectedProject}
        onSelectProject={setSelectedProject}
        onOpenSettings={() => setSettingsOpen(true)}
      />

      <SessionPanel
        sessions={sessions}
        activeSessionId={activeSessionId}
        onSelectSession={handleSelectSession}
        onNewSession={handleNewSession}
        onRenameSession={handleRenameSession}
        onDeleteSession={handleDeleteSession}
      />

      <SettingsDialog
        open={settingsOpen}
        onOpenChange={setSettingsOpen}
        models={models}
        selectedModel={selectedModel ?? undefined}
        onModelChange={setSelectedModel}
      />

      <CommandPalette
        open={commandPaletteOpen}
        onOpenChange={setCommandPaletteOpen}
        sessions={sessions.map((s) => ({ id: s.id, title: s.title }))}
        onSelectSession={handleSelectSession}
        onNewSession={handleNewSession}
        onClearChat={handleClearConversation}
        onOpenSettings={() => setSettingsOpen(true)}
        onToggleTheme={toggleTheme}
        onRunCommand={handleSend}
      />

      <main data-component="main-content" className="flex-1 flex flex-col overflow-hidden">
        {!authLoading && authStatus && !authStatus.configured ? (
          <AuthSetup />
        ) : (
          <ChatLayout
            turns={turns}
            isStreaming={isStreaming}
            error={chatError}
            currentTurnId={currentTurnId}
            onSend={handleSend}
            onAbort={abort}
            hasSession={chatSession !== null}
            models={models}
            selectedModel={selectedModel ?? undefined}
            onModelChange={setSelectedModel}
            contextUsage={contextUsage}
          />
        )}
      </main>

      <DronePanel
        drones={drones}
        connectionStatus={connectionStatus}
        collapsed={dronePanelCollapsed}
        onToggleCollapse={toggleDronePanel}
      />
    </div>
  );
}
