import { useCallback, useEffect, useRef, useState } from "react";

// ── Types ────────────────────────────────────────────────────────────────────

export interface SessionMeta {
  id: string;
  title: string;
  cwd: string;
  status: "idle" | "busy" | "completed" | "error";
  created_at: string;
  updated_at: string;
}

export interface SessionHistoryResponse {
  events: unknown[];
}

interface UseSessionListResult {
  sessions: SessionMeta[];
  loading: boolean;
  error: string | null;
  refresh: () => void;
  deleteSession: (id: string) => Promise<void>;
  loadHistory: (id: string) => Promise<unknown[]>;
}

// ── Constants ────────────────────────────────────────────────────────────────

const POLL_INTERVAL_MS = 5_000;

// ── Hook ─────────────────────────────────────────────────────────────────────

export function useSessionList(baseUrl: string = ""): UseSessionListResult {
  const [sessions, setSessions] = useState<SessionMeta[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval> | undefined>(undefined);

  const fetchSessions = useCallback(async () => {
    try {
      const res = await fetch(`${baseUrl}/api/chat/sessions`);
      if (!res.ok) {
        throw new Error(`Failed to fetch sessions: ${res.status}`);
      }
      const data = (await res.json()) as SessionMeta[];
      setSessions(data);
      setError(null);
    } catch (err) {
      const msg = err instanceof Error ? err.message : "Unknown error";
      setError(msg);
    } finally {
      setLoading(false);
    }
  }, [baseUrl]);

  const deleteSession = useCallback(
    async (id: string) => {
      try {
        const res = await fetch(`${baseUrl}/api/chat/sessions/${id}`, {
          method: "DELETE",
        });
        if (!res.ok) {
          throw new Error(`Failed to delete session: ${res.status}`);
        }
        // Optimistic removal
        setSessions((prev) => prev.filter((s) => s.id !== id));
      } catch (err) {
        const msg = err instanceof Error ? err.message : "Unknown error";
        setError(msg);
      }
    },
    [baseUrl],
  );

  const loadHistory = useCallback(
    async (id: string): Promise<unknown[]> => {
      const res = await fetch(`${baseUrl}/api/chat/sessions/${id}/history`);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const data = (await res.json()) as SessionHistoryResponse;
      return data.events;
    },
    [baseUrl],
  );

  // Initial fetch + polling
  useEffect(() => {
    fetchSessions();
    intervalRef.current = setInterval(fetchSessions, POLL_INTERVAL_MS);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [fetchSessions]);

  return {
    sessions,
    loading,
    error,
    refresh: fetchSessions,
    deleteSession,
    loadHistory,
  };
}

// ── Date grouping helper ─────────────────────────────────────────────────────

export type DateGroup = "Today" | "Yesterday" | "This Week" | "Older";

export interface GroupedSessions {
  group: DateGroup;
  items: SessionMeta[];
}

export function groupSessionsByDate(sessions: SessionMeta[]): GroupedSessions[] {
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const yesterday = new Date(today.getTime() - 86_400_000);
  const weekAgo = new Date(today.getTime() - 7 * 86_400_000);

  const groups: Record<DateGroup, SessionMeta[]> = {
    Today: [],
    Yesterday: [],
    "This Week": [],
    Older: [],
  };

  for (const session of sessions) {
    const date = new Date(session.created_at);
    if (date >= today) {
      groups.Today.push(session);
    } else if (date >= yesterday) {
      groups.Yesterday.push(session);
    } else if (date >= weekAgo) {
      groups["This Week"].push(session);
    } else {
      groups.Older.push(session);
    }
  }

  const result: GroupedSessions[] = [];
  const order: DateGroup[] = ["Today", "Yesterday", "This Week", "Older"];
  for (const group of order) {
    if (groups[group].length > 0) {
      result.push({ group, items: groups[group] });
    }
  }

  return result;
}

// ── Relative time helper ────────────────────────────────────────────────────

export function relativeTime(dateStr: string): string {
  const date = new Date(dateStr);
  const now = Date.now();
  const diffMs = now - date.getTime();
  const diffSec = Math.floor(diffMs / 1000);
  const diffMin = Math.floor(diffSec / 60);
  const diffHr = Math.floor(diffMin / 60);
  const diffDay = Math.floor(diffHr / 24);

  if (diffSec < 60) return "just now";
  if (diffMin < 60) return `${diffMin}m ago`;
  if (diffHr < 24) return `${diffHr}h ago`;
  if (diffDay < 7) return `${diffDay}d ago`;
  return date.toLocaleDateString();
}
