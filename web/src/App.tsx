import { useState, useEffect, useCallback } from "react";
import { useProjects } from "@/hooks/use-projects";
import { useTheme } from "@/hooks/use-theme";
import { useIsMobile } from "@/hooks/use-mobile";
import { useChat } from "@/hooks/use-chat";
import { ProjectSidebar } from "@/components/project-sidebar";
import { DetailPanel } from "@/components/detail-panel";
import { MobileNav } from "@/components/mobile-nav";
import { AppSidebar } from "@/components/layout/app-sidebar";
import { ChatLayout } from "@/components/chat/chat-layout";
import type { AppMode } from "@/components/layout/mode-switcher";
import type { SessionEntry } from "@/components/layout/app-sidebar";

// ── Session handoff cache ────────────────────────────────────────────────────

interface SessionSnapshot {
  scrollTop: number
  promptText: string
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
  const { projects, connectionStatus } = useProjects();
  const [selectedProject, setSelectedProject] = useState<string | null>(null);
  const [selectedDrone, setSelectedDrone] = useState<string | null>(null);
  const [mode, setMode] = useState<AppMode>("chat");
  const isMobile = useIsMobile();
  useTheme();

  // ── Chat state ───────────────────────────────────────────────────────────

  const { state: chatState, sendMessage, abort, createSession } = useChat();
  const [sessions, setSessions] = useState<SessionEntry[]>([]);
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null);

  const addSession = useCallback(
    async (title: string): Promise<boolean> => {
      try {
        const cwd = projects[0]?.path ?? "/";
        const session = await createSession(cwd);
        const entry: SessionEntry = {
          id: session.id,
          title,
          createdAt: session.createdAt,
          status: session.status,
        };
        setSessions((prev) => [entry, ...prev]);
        setActiveSessionId(session.id);
        return true;
      } catch {
        return false;
      }
    },
    [projects, createSession],
  );

  const handleNewSession = useCallback(async () => {
    await addSession(`Session ${sessions.length + 1}`);
  }, [addSession, sessions.length]);

  const handleSelectSession = useCallback((id: string) => {
    setActiveSessionId(id);
  }, []);

  const handleSend = useCallback(
    async (message: string) => {
      if (!chatState.session) {
        const title = message.slice(0, 50) || `Session ${sessions.length + 1}`;
        const created = await addSession(title);
        if (!created) return;
      }

      if (chatState.turns.length === 0 && activeSessionId) {
        setSessions((prev) =>
          prev.map((s) =>
            s.id === activeSessionId
              ? { ...s, title: message.slice(0, 50) || s.title }
              : s,
          ),
        );
      }

      await sendMessage(message);
    },
    [chatState.session, chatState.turns.length, activeSessionId, addSession, sessions.length, sendMessage],
  );

  // ── Monitor state ────────────────────────────────────────────────────────

  const isMock = connectionStatus === "mock";

  // Auto-select first project
  useEffect(() => {
    if (projects.length > 0 && !selectedProject) {
      setSelectedProject(projects[0]!.path);
    }
  }, [projects, selectedProject]);

  // Reset drone selection when project changes
  useEffect(() => {
    setSelectedDrone(null);
  }, [selectedProject]);

  // Auto-select first drone
  const activeProject = projects.find((p) => p.path === selectedProject) ?? null;
  const drones = activeProject?.drones ?? [];

  useEffect(() => {
    if (drones.length === 1 && !selectedDrone) {
      setSelectedDrone(drones[0]!.name);
    }
  }, [drones, selectedDrone]);

  useEffect(() => {
    if (isMock && drones.length > 0 && !selectedDrone) {
      setSelectedDrone(drones[0]!.name);
    }
  }, [isMock, drones, selectedDrone]);

  const activeDrone = drones.find((d) => d.name === selectedDrone) ?? null;
  const showProjectSidebar = projects.length > 1;

  // ── Mobile ───────────────────────────────────────────────────────────────

  if (isMobile) {
    return (
      <MobileNav
        projects={projects}
        selectedProject={selectedProject}
        onSelectProject={setSelectedProject}
        selectedDrone={selectedDrone}
        onSelectDrone={setSelectedDrone}
        connectionStatus={connectionStatus}
        isMock={isMock}
      />
    );
  }

  // ── Desktop ──────────────────────────────────────────────────────────────

  return (
    <div data-component="app-root" className="flex h-screen overflow-hidden">
      {/* Multi-project sidebar (if applicable) */}
      {showProjectSidebar && mode === "monitor" && (
        <ProjectSidebar
          projects={projects}
          selectedProject={selectedProject}
          onSelectProject={setSelectedProject}
        />
      )}

      {/* App sidebar — dual mode */}
      <AppSidebar
        mode={mode}
        onModeChange={setMode}
        sessions={sessions}
        activeSessionId={activeSessionId}
        onSelectSession={handleSelectSession}
        onNewSession={handleNewSession}
        drones={drones}
        selectedDrone={selectedDrone}
        onSelectDrone={setSelectedDrone}
        connectionStatus={connectionStatus}
        projectName={activeProject?.name}
      />

      {/* Main content — mode dependent */}
      <main data-component="main-content" className="flex-1 flex flex-col overflow-hidden">
        {mode === "chat" ? (
          <ChatLayout
            turns={chatState.turns}
            isStreaming={chatState.isStreaming}
            error={chatState.error}
            currentTurnId={chatState.currentTurnId}
            onSend={handleSend}
            onAbort={abort}
            hasSession={chatState.session !== null}
          />
        ) : (
          <DetailPanel
            drone={activeDrone}
            isMock={isMock}
            projectPath={activeProject?.path}
          />
        )}
      </main>
    </div>
  );
}
