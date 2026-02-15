import { useCallback, useEffect, useRef } from "react";
import type { SessionEntry } from "@/domains/chat/components/sessions-modal";
import { useDeleteSession, useRenameSession } from "@/domains/chat/mutations";
import { useSessionsQuery } from "@/domains/chat/queries";
import type { ChatSession, SessionStatus, StreamEvent } from "@/domains/chat/types";
import { useChat } from "@/domains/chat/use-chat-stream";
import { apiClient } from "@/shared/api/client";
import { useAppStore } from "@/store";

// ── Helpers ─────────────────────────────────────────────────────────────────

const SESSION_STATUSES: ReadonlySet<string> = new Set<SessionStatus>([
  "idle",
  "busy",
  "completed",
  "error",
]);

function isSessionStatus(value: string | undefined): value is SessionStatus {
  return value !== undefined && SESSION_STATUSES.has(value);
}

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

// ── Hook ────────────────────────────────────────────────────────────────────

interface UseSessionManagerOptions {
  selectedProject: string | null;
  selectedModel: string | null;
  monitorProjects: { path: string }[];
  toast: (message: string, variant: "success" | "error" | "info") => void;
}

export function useSessionManager({
  selectedProject,
  selectedModel,
  monitorProjects,
  toast,
}: UseSessionManagerOptions) {
  const activeSessionId = useAppStore((s) => s.activeSessionId);
  const setActiveSessionId = useAppStore((s) => s.setActiveSession);
  const dispatchChat = useAppStore((s) => s.dispatchChat);

  const { data: allSessions = [], isLoading: sessionsLoading } = useSessionsQuery();
  const renameSessionMutation = useRenameSession();
  const deleteSessionMutation = useDeleteSession();
  const { sendMessage, abort, createSession, resetSession } = useChat();

  // ── Sessions filtered by project ──────────────────────────────────────
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

  // ── Session CRUD ──────────────────────────────────────────────────────
  const addSession = useCallback(
    async (title: string) => {
      try {
        const cwd = selectedProject ?? monitorProjects[0]?.path ?? "/";
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
      monitorProjects,
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
    (id: string) => setActiveSessionId(id),
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

  // ── Session replay ────────────────────────────────────────────────────
  const prevSessionIdRef = useRef<string | null>(null);

  useEffect(() => {
    if (!activeSessionId) {
      prevSessionIdRef.current = null;
      return;
    }

    if (activeSessionId === prevSessionIdRef.current) return;
    prevSessionIdRef.current = activeSessionId;

    const state = useAppStore.getState();
    if (state.session?.id === activeSessionId && state.turns.length > 0) return;

    const sessionMeta = allSessions.find((s) => s.id === activeSessionId);
    const session: ChatSession = {
      id: activeSessionId,
      status: isSessionStatus(sessionMeta?.status) ? sessionMeta.status : "idle",
      cwd: sessionMeta?.cwd ?? selectedProject ?? "/",
      createdAt: sessionMeta?.created_at ?? new Date().toISOString(),
    };

    apiClient
      .get<{
        events: unknown[];
        total_input_tokens?: number;
        total_output_tokens?: number;
      }>(`/api/chat/sessions/${activeSessionId}/history`)
      .then((res) => {
        const events = (res.events ?? []).filter(
          (e): e is StreamEvent =>
            typeof e === "object" && e !== null && "type" in e && typeof e.type === "string",
        );
        const tokenCounts =
          res.total_input_tokens || res.total_output_tokens
            ? {
                inputTokens: res.total_input_tokens ?? 0,
                outputTokens: res.total_output_tokens ?? 0,
              }
            : undefined;
        dispatchChat({ type: "REPLAY_HISTORY", session, events, tokenCounts });
      })
      .catch(() => {
        dispatchChat({ type: "SESSION_CREATED", session });
      });
  }, [activeSessionId, allSessions, selectedProject, dispatchChat]);

  // ── Auto-resume session ────────────────────────────────────────────────
  // Picks the most recent session for the current project when none is active.
  // Fires on mount AND on project switch (when activeSessionId becomes null).
  const prevProjectRef = useRef(selectedProject);

  useEffect(() => {
    // Reset when project changes so auto-resume can fire for new project
    if (selectedProject !== prevProjectRef.current) {
      prevProjectRef.current = selectedProject;
    }
  }, [selectedProject]);

  useEffect(() => {
    // Wait for sessions to load
    if (sessionsLoading) return;
    // Already have an active session
    if (activeSessionId) return;
    // No sessions for this project
    if (sessions.length === 0) return;
    // URL-based session selection takes priority
    const segments = window.location.pathname.split("/").filter(Boolean);
    if (segments.length >= 2) return;

    const mostRecent = sessions[0];
    if (mostRecent) {
      setActiveSessionId(mostRecent.id);
    }
  }, [sessionsLoading, activeSessionId, sessions, setActiveSessionId]);

  // ── Reset session when switching projects ─────────────────────────────
  // Guard: don't reset until sessions have loaded (prevents race on reload)
  useEffect(() => {
    if (sessionsLoading) return;
    if (!activeSessionId || !selectedProject) return;
    const sessionBelongs = sessions.some((s) => s.id === activeSessionId);
    if (!sessionBelongs) {
      setActiveSessionId(null);
      resetSession();
    }
  }, [
    sessionsLoading,
    selectedProject,
    activeSessionId,
    sessions,
    resetSession,
    setActiveSessionId,
  ]);

  return {
    sessions,
    sessionsLoading,
    activeSessionId,
    allSessions,
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
  };
}
