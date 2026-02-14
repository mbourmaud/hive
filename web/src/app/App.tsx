import { useCallback, useEffect, useMemo, useRef } from "react";
import { DronePanel } from "@/app/layout/drone-panel";
import { IconBar } from "@/app/layout/icon-bar";
import { MobileNav } from "@/app/layout/mobile-nav";
import { ChatLayout } from "@/domains/chat/components/chat-layout";
import type { SessionEntry } from "@/domains/chat/components/sessions-modal";
import { SessionsModal } from "@/domains/chat/components/sessions-modal";
import { useDeleteSession, useRenameSession } from "@/domains/chat/mutations";
import { useSessionsQuery } from "@/domains/chat/queries";
import { type SlashCommandContext, executeSlashCommand } from "@/domains/chat/slash-commands";
import type { ChatSession, ImageAttachment, SessionStatus, StreamEvent } from "@/domains/chat/types";
import { useChat } from "@/domains/chat/use-chat-stream";

const SESSION_STATUSES: ReadonlySet<string> = new Set<SessionStatus>(["idle", "busy", "completed", "error"]);
function isSessionStatus(value: string | undefined): value is SessionStatus {
  return value !== undefined && SESSION_STATUSES.has(value);
}
import { useProjectsSSE } from "@/domains/monitor/queries";
import { ContextBar } from "@/domains/projects/components/context-bar";
import { OnboardingWizard } from "@/domains/projects/components/onboarding-wizard";
import { useProjectRegistryQuery } from "@/domains/projects/queries";
import type { ProjectProfile } from "@/domains/projects/types";
import { useDetection } from "@/domains/projects/use-detection";
import { AuthSetup } from "@/domains/settings/components/auth-setup";
import { CommandPalette } from "@/domains/settings/components/command-palette";
import { SettingsDialog } from "@/domains/settings/components/settings-dialog";
import { useAuthStatusQuery, useModelsQuery } from "@/domains/settings/queries";
import { apiClient } from "@/shared/api/client";
import { useKeyboardShortcuts } from "@/shared/hooks/use-keyboard-shortcuts";
import { useIsMobile } from "@/shared/hooks/use-mobile";
import { useUrlState } from "@/shared/hooks/use-url-state";
import { THEMES, useTheme } from "@/shared/theme/use-theme";
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
  const sessionsModalOpen = useAppStore((s) => s.sessionsModalOpen);
  const setSessionsModalOpen = useAppStore((s) => s.setSessionsModalOpen);
  const activeSessionId = useAppStore((s) => s.activeSessionId);
  const setActiveSessionId = useAppStore((s) => s.setActiveSession);
  const effort = useAppStore((s) => s.effort);
  const setEffort = useAppStore((s) => s.setEffort);

  // Projects store
  const registryProjects = useAppStore((s) => s.registryProjects);
  const setRegistryProjects = useAppStore((s) => s.setRegistryProjects);
  const activeProjectContext = useAppStore((s) => s.activeProjectContext);
  const setActiveProjectContext = useAppStore((s) => s.setActiveProjectContext);
  const contextCacheTime = useAppStore((s) => s.contextCacheTime);
  const setContextCacheTime = useAppStore((s) => s.setContextCacheTime);
  const onboardingComplete = useAppStore((s) => s.onboardingComplete);
  const setOnboardingComplete = useAppStore((s) => s.setOnboardingComplete);

  // Chat state from Zustand (populated by useChat's dispatchChat)
  const turns = useAppStore((s) => s.turns);
  const isStreaming = useAppStore((s) => s.isStreaming);
  const chatError = useAppStore((s) => s.error);
  const currentTurnId = useAppStore((s) => s.currentTurnId);
  const chatSession = useAppStore((s) => s.session);
  const contextUsage = useAppStore((s) => s.contextUsage);

  // ── TanStack Query ────────────────────────────────────────────────────────
  const { data: projects = [], connectionStatus } = useProjectsSSE();
  useUrlState(projects);
  const { data: authStatus, isLoading: authLoading } = useAuthStatusQuery();
  const { data: models = [] } = useModelsQuery();
  const { data: allSessions = [] } = useSessionsQuery();
  const { data: registryData } = useProjectRegistryQuery();

  // ── Mutations ─────────────────────────────────────────────────────────────
  const renameSessionMutation = useRenameSession();
  const deleteSessionMutation = useDeleteSession();

  // ── Chat streaming (dispatches to Zustand) ────────────────────────────────
  const { sendMessage, abort, createSession, resetSession } = useChat();

  // ── Other hooks ───────────────────────────────────────────────────────────
  const isMobile = useIsMobile();
  const { toast } = useToast();
  const { toggleTheme, setThemeName } = useTheme();
  const { context: detectedContext, startDetection } = useDetection();

  // ── Sync registry data to store ────────────────────────────────────────────
  useEffect(() => {
    if (registryData && registryData.length > 0) {
      setRegistryProjects(registryData);
    }
  }, [registryData, setRegistryProjects]);

  // ── Auto-detect context on project switch ──────────────────────────────────
  const prevDetectProjectRef = useRef<string | null>(null);
  useEffect(() => {
    if (!selectedProject || selectedProject === prevDetectProjectRef.current) return;
    prevDetectProjectRef.current = selectedProject;

    // Skip if cached and < 5 minutes old
    const CACHE_TTL = 5 * 60 * 1000;
    if (activeProjectContext && contextCacheTime && Date.now() - contextCacheTime < CACHE_TTL) {
      return;
    }

    // Find registry project for this path
    const regProject = registryProjects.find((p) => p.path === selectedProject);
    if (regProject) {
      startDetection(regProject.id);
    }
  }, [selectedProject, registryProjects, activeProjectContext, contextCacheTime, startDetection]);

  // ── Cache detection results ─────────────────────────────────────────────────
  useEffect(() => {
    if (detectedContext) {
      setActiveProjectContext(detectedContext);
      setContextCacheTime(Date.now());
    }
  }, [detectedContext, setActiveProjectContext, setContextCacheTime]);

  // ── Onboarding completion handler ─────────────────────────────────────────
  const handleOnboardingComplete = useCallback(
    (project: ProjectProfile) => {
      setSelectedProject(project.path);
      if (project.color_theme) {
        const themeInfo = THEMES.find((t) => t.name === project.color_theme);
        if (themeInfo) {
          setThemeName(themeInfo.name);
        }
      }
      setOnboardingComplete(true);
    },
    [setSelectedProject, setThemeName, setOnboardingComplete],
  );

  // ── Add project handler (from icon bar) ─────────────────────────────────────
  const handleAddProject = useCallback(() => {
    setOnboardingComplete(false);
  }, [setOnboardingComplete]);

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

  const activeProject = projects.find((p) => p.path === selectedProject) ?? null;
  const drones = activeProject?.drones ?? [];
  const dispatchChat = useAppStore((s) => s.dispatchChat);

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
      dronePanelCollapsed,
      toggleDronePanel,
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
      dronePanelCollapsed,
      toggleDronePanel,
    ],
  );

  const handleSend = useCallback(
    async (message: string, images?: ImageAttachment[]) => {
      if (message.startsWith("/")) {
        const handled = await executeSlashCommand(message, commandCtx);
        if (handled) return;
      }

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
    [commandCtx, chatSession, turns.length, activeSessionId, addSession, sessions.length, sendMessage, selectedModel, renameSessionMutation],
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
    onOpenSessions: () => setSessionsModalOpen(true),
    onDeleteSession: handleDeleteCurrentSession,
    onAbort: abort,
    onToggleDronePanel: toggleDronePanel,
    isStreaming,
  });

  // ── Session replay: load history when activeSessionId changes ────────────
  const prevSessionIdRef = useRef<string | null>(null);

  useEffect(() => {
    if (!activeSessionId) {
      prevSessionIdRef.current = null;
      return;
    }

    // Skip if same session (avoids re-replaying on re-renders)
    if (activeSessionId === prevSessionIdRef.current) return;
    prevSessionIdRef.current = activeSessionId;

    // Don't replay if we already have turns for this session (live session)
    const state = useAppStore.getState();
    if (state.session?.id === activeSessionId && state.turns.length > 0) return;

    // Fetch history and dispatch REPLAY_HISTORY
    const sessionMeta = allSessions.find((s) => s.id === activeSessionId);
    const session: ChatSession = {
      id: activeSessionId,
      status: isSessionStatus(sessionMeta?.status) ? sessionMeta.status : "idle",
      cwd: sessionMeta?.cwd ?? selectedProject ?? "/",
      createdAt: sessionMeta?.created_at ?? new Date().toISOString(),
    };

    apiClient
      .get<{ events: unknown[] }>(`/api/chat/sessions/${activeSessionId}/history`)
      .then((res) => {
        const events = (res.events ?? []).filter(
          (e): e is StreamEvent =>
            typeof e === "object" &&
            e !== null &&
            "type" in e &&
            typeof e.type === "string",
        );
        dispatchChat({ type: "REPLAY_HISTORY", session, events });
      })
      .catch(() => {
        // No history found — just create an empty session
        dispatchChat({ type: "SESSION_CREATED", session });
      });
  }, [activeSessionId, allSessions, selectedProject, dispatchChat]);

  // ── Auto-continue: resume most recent session on startup ──────────────
  // Only auto-selects when the URL didn't specify a session (i.e. navigated to "/" or "/{project}")
  const autoResumedRef = useRef(false);

  useEffect(() => {
    if (autoResumedRef.current) return;
    if (allSessions.length === 0) return;
    autoResumedRef.current = true;

    // If we already have an activeSessionId (from URL or localStorage), use it
    if (activeSessionId) return;

    // Check if the URL specified a session — if so, the URL hook will handle it
    const segments = window.location.pathname.split("/").filter(Boolean);
    if (segments.length >= 2) return;

    // Otherwise auto-select the most recent session
    const mostRecent = allSessions[0];
    if (mostRecent) {
      setActiveSessionId(mostRecent.id);
    }
  }, [allSessions, activeSessionId, setActiveSessionId]);

  // Project auto-select is handled by useUrlState hook

  // Reset session when switching projects
  useEffect(() => {
    if (!activeSessionId || !selectedProject) return;
    const sessionBelongs = sessions.some((s) => s.id === activeSessionId);
    if (!sessionBelongs) {
      setActiveSessionId(null);
      resetSession();
    }
  }, [selectedProject, activeSessionId, sessions, resetSession, setActiveSessionId]);

  // ── Main content resolution (avoids nested ternaries) ───────────────────
  function renderMainContent() {
    if (!authLoading && authStatus && !authStatus.configured) {
      return <AuthSetup />;
    }
    if (registryProjects.length === 0 && !onboardingComplete) {
      return <OnboardingWizard onComplete={handleOnboardingComplete} />;
    }
    return (
      <>
        {activeProjectContext && <ContextBar context={activeProjectContext} />}
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
          effort={effort}
          onEffortChange={setEffort}
        />
      </>
    );
  }

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
        registryProjects={registryProjects}
        activeProject={selectedProject}
        onSelectProject={setSelectedProject}
        onAddProject={handleAddProject}
        onOpenSettings={() => setSettingsOpen(true)}
      />

      <SessionsModal
        open={sessionsModalOpen}
        onOpenChange={setSessionsModalOpen}
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
        {renderMainContent()}
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
