import { queryOptions, useQuery } from "@tanstack/react-query";
import { apiClient } from "@/shared/api/client";
import { queryKeys } from "@/shared/api/query-keys";

// ── Types ────────────────────────────────────────────────────────────────────

export interface SessionMeta {
  id: string;
  title: string;
  cwd: string;
  status: "idle" | "busy" | "completed" | "error";
  created_at: string;
  updated_at: string;
}

interface SessionHistoryResponse {
  events: unknown[];
}

// ── Query Options ────────────────────────────────────────────────────────────

export const sessionsQueryOptions = queryOptions({
  queryKey: queryKeys.sessions.list(),
  queryFn: () => apiClient.get<SessionMeta[]>("/api/chat/sessions"),
  staleTime: 5_000,
  refetchInterval: 5_000,
});

export const sessionHistoryQueryOptions = (id: string) =>
  queryOptions({
    queryKey: queryKeys.sessions.history(id),
    queryFn: () =>
      apiClient
        .get<SessionHistoryResponse>(`/api/chat/sessions/${id}/history`)
        .then((r) => r.events),
    enabled: !!id,
  });

// ── Hooks ────────────────────────────────────────────────────────────────────

export function useSessionsQuery() {
  return useQuery(sessionsQueryOptions);
}

export function useSessionHistoryQuery(id: string) {
  return useQuery(sessionHistoryQueryOptions(id));
}

// ── Date grouping helpers ───────────────────────────────────────────────────

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
