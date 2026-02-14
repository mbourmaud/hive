import { useCallback, useMemo, useState } from "react";
import { DronePanel } from "@/app/layout/drone-panel";
import { IconBar } from "@/app/layout/icon-bar";
import { MobileNav } from "@/app/layout/mobile-nav";
import { ChatLayout } from "@/domains/chat/components/chat-layout";
import { SessionsModal } from "@/domains/chat/components/sessions-modal";
import { type SlashCommandContext, executeSlashCommand } from "@/domains/chat/slash-commands";
import type { ImageAttachment } from "@/domains/chat/types";

import { useProjectsSSE } from "@/domains/monitor/queries";
import { ContextBar } from "@/domains/projects/components/context-bar";
import { OnboardingWizard } from "@/domains/projects/components/onboarding-wizard";
import { AuthSetup } from "@/domains/settings/components/auth-setup";
import { CommandPalette } from "@/domains/settings/components/command-palette";
import { SettingsDialog } from "@/domains/settings/components/settings-dialog";
import { useAuthStatusQuery, useModelsQuery } from "@/domains/settings/queries";
import { useKeyboardShortcuts } from "@/shared/hooks/use-keyboard-shortcuts";
import { useIsMobile } from "@/shared/hooks/use-mobile";
import { useUrlState } from "@/shared/hooks/use-url-state";
import { useTheme } from "@/shared/theme/use-theme";
import { ToastProvider, useToast } from "@/shared/ui/toast";
import { useAppStore } from "@/store";
import { useDefaultModel } from "./hooks/use-default-model";
import { useProjectDetection } from "./hooks/use-project-detection";
import { useSessionManager } from "./hooks/use-session-manager";

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
  const statusPopoverOpen = useAppStore((s) => s.statusPopoverOpen);
  const setStatusPopoverOpen = useAppStore((s) => s.setStatusPopoverOpen);
  const effort = useAppStore((s) => s.effort);
  const setEffort = useAppStore((s) => s.setEffort);

  // ── Project detection (registry sync, auto-detect, caching, onboarding) ──
  const { registryProjects, activeProjectContext, onboardingComplete, handleOnboardingComplete } =
    useProjectDetection();

  // Local UI state — explicit "add project" mode (not persisted)
  const [isAddingProject, setIsAddingProject] = useState(false);

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

  // ── Other hooks ───────────────────────────────────────────────────────────
  const isMobile = useIsMobile();
  const { toast } = useToast();
  const { toggleTheme } = useTheme();

  // ── Session manager (extracted hook) ────────────────────────────────────
  const {
    sessions,
    activeSessionId,
    addSession,
    handleNewSession,
    handleSelectSession,
    handleRenameSession,
    handleDeleteSession,
    sendMessage,
    abort,
    resetSession,
    renameSessionMutation,
    dispatchChat,
  } = useSessionManager({
    selectedProject,
    selectedModel,
    monitorProjects: projects,
    toast,
  });

  // ── Add project handler (from icon bar) ─────────────────────────────────────
  const handleAddProject = useCallback(() => setIsAddingProject(true), []);
  useDefaultModel(models);

  // ── Drones & slash commands ─────────────────────────────────────────────────
  const activeProject = projects.find((p) => p.path === selectedProject) ?? null;
  const drones = activeProject?.drones ?? [];

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
    onToggleStatus: () => setStatusPopoverOpen(!statusPopoverOpen),
    isStreaming,
  });

  // ── Main content resolution (avoids nested ternaries) ───────────────────
  function renderMainContent() {
    if (!authLoading && authStatus && !authStatus.configured) {
      return <AuthSetup />;
    }
    // Show wizard: first-time (no projects + not completed) OR user clicked "+"
    const showWizard =
      isAddingProject || (registryProjects.length === 0 && !onboardingComplete);
    if (showWizard) {
      return (
        <OnboardingWizard
          onComplete={(p) => { handleOnboardingComplete(p); setIsAddingProject(false); }}
          onCancel={registryProjects.length > 0 ? () => setIsAddingProject(false) : undefined}
        />
      );
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
        statusPopoverOpen={statusPopoverOpen}
        onStatusPopoverChange={setStatusPopoverOpen}
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
