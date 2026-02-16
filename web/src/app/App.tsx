import { useCallback, useMemo, useState } from "react";
import { IconBar } from "@/app/layout/icon-bar";
import { MobileNav } from "@/app/layout/mobile-nav";
import { RightSidebar } from "@/app/layout/right-sidebar/right-sidebar";
import { ChatLayout } from "@/domains/chat/components/chat-layout";
import { SessionsModal } from "@/domains/chat/components/sessions-modal";
import { executeSlashCommand, type SlashCommandContext } from "@/domains/chat/slash-commands";
import type { ImageAttachment } from "@/domains/chat/types";

import { useProjectsSSE } from "@/domains/monitor/queries";
import { ContextBar } from "@/domains/projects/components/context-bar";
import { ContextBarSkeleton } from "@/domains/projects/components/context-bar-skeleton";
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
  const selectedProject = useAppStore((s) => s.selectedProject);
  const setSelectedProject = useAppStore((s) => s.setSelectedProject);
  const selectedModel = useAppStore((s) => s.selectedModel);
  const setSelectedModel = useAppStore((s) => s.setSelectedModel);
  const rightSidebarTab = useAppStore((s) => s.rightSidebarTab);
  const rightSidebarCollapsed = useAppStore((s) => s.rightSidebarCollapsed);
  const setRightSidebarTab = useAppStore((s) => s.setRightSidebarTab);
  const toggleRightSidebar = useAppStore((s) => s.toggleRightSidebar);
  const openRightSidebar = useAppStore((s) => s.openRightSidebar);
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
  const chatMode = useAppStore((s) => s.chatMode);
  const setChatMode = useAppStore((s) => s.setChatMode);

  const { registryProjects, activeProjectContext, isDetecting, handleOnboardingComplete } =
    useProjectDetection();

  const [isAddingProject, setIsAddingProject] = useState(false);
  const turns = useAppStore((s) => s.turns);
  const isStreaming = useAppStore((s) => s.isStreaming);
  const chatError = useAppStore((s) => s.error);
  const currentTurnId = useAppStore((s) => s.currentTurnId);
  const chatSession = useAppStore((s) => s.session);
  const contextUsage = useAppStore((s) => s.contextUsage);
  const { data: projects = [], connectionStatus } = useProjectsSSE();
  useUrlState(projects);
  const { data: authStatus, isLoading: authLoading } = useAuthStatusQuery();
  const { data: models = [] } = useModelsQuery();
  const isMobile = useIsMobile();
  const { toast } = useToast();
  const { toggleTheme } = useTheme();
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

  const handleAddProject = useCallback(() => setIsAddingProject(true), []);
  useDefaultModel(models);
  const activeProject = projects.find((p) => p.path === selectedProject) ?? null;
  const drones = activeProject?.drones ?? [];

  const setActiveSessionId = useAppStore((s) => s.setActiveSession);
  const reloadSession = useCallback(
    (id: string) => {
      dispatchChat({ type: "SESSION_RESET" });
      setActiveSessionId(null);
      // Defer so the effect sees the null → id transition
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
      openRightSidebar,
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
          renameSessionMutation.mutate({ id: activeSessionId, title: newTitle });
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
      renameSessionMutation,
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

  useKeyboardShortcuts({
    onNewSession: handleNewSession,
    onOpenSettings: () => setSettingsOpen(true),
    onOpenCommandPalette: () => setCommandPaletteOpen(true),
    onOpenSessions: () => setSessionsModalOpen(true),
    onDeleteSession: handleDeleteCurrentSession,
    onAbort: abort,
    onToggleRightSidebar: toggleRightSidebar,
    onToggleStatus: () => setStatusPopoverOpen(!statusPopoverOpen),
    onOpenContextPanel: () => openRightSidebar("context"),
    isStreaming,
  });

  function renderMainContent() {
    if (!authLoading && authStatus && !authStatus.configured) return <AuthSetup />;
    const showWizard = isAddingProject || registryProjects.length === 0;
    if (showWizard) {
      return (
        <OnboardingWizard
          onComplete={(p) => {
            handleOnboardingComplete(p);
            setIsAddingProject(false);
          }}
          onCancel={registryProjects.length > 0 ? () => setIsAddingProject(false) : undefined}
        />
      );
    }
    return (
      <>
        {isDetecting && !activeProjectContext ? (
          <ContextBarSkeleton />
        ) : activeProjectContext ? (
          <ContextBar context={activeProjectContext} />
        ) : null}
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
          chatMode={chatMode}
          onModeChange={setChatMode}
        />
      </>
    );
  }

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

      <RightSidebar
        drones={drones}
        connectionStatus={connectionStatus}
        turns={turns}
        contextUsage={contextUsage ?? null}
        session={chatSession}
        selectedModel={selectedModel ?? undefined}
        activeTab={rightSidebarTab}
        collapsed={rightSidebarCollapsed}
        onTabChange={setRightSidebarTab}
        onToggleCollapse={toggleRightSidebar}
        onOpen={openRightSidebar}
      />
    </div>
  );
}
