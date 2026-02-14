import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "@/shared/api/client";
import { queryKeys } from "@/shared/api/query-keys";
import type { SessionMeta } from "./queries";

// ── Types ────────────────────────────────────────────────────────────────────

interface CreateSessionParams {
  cwd: string;
  model?: string;
  agent?: string;
  max_turns?: number;
}

interface CreateSessionResponse {
  id: string;
  status: string;
  cwd: string;
  createdAt: string;
}

interface RenameSessionParams {
  id: string;
  title: string;
}

// ── Mutations ────────────────────────────────────────────────────────────────

export function useCreateSession() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: CreateSessionParams) =>
      apiClient.post<CreateSessionResponse>("/api/chat/sessions", params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.sessions.all() });
    },
  });
}

export function useRenameSession() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, title }: RenameSessionParams) =>
      apiClient.patch<unknown>(`/api/chat/sessions/${id}`, { title }),
    onMutate: async ({ id, title }) => {
      await queryClient.cancelQueries({ queryKey: queryKeys.sessions.list() });
      const previous = queryClient.getQueryData<SessionMeta[]>(queryKeys.sessions.list());
      queryClient.setQueryData<SessionMeta[]>(queryKeys.sessions.list(), (old) =>
        old?.map((s) => (s.id === id ? { ...s, title } : s)),
      );
      return { previous };
    },
    onError: (_err, _vars, context) => {
      if (context?.previous) {
        queryClient.setQueryData(queryKeys.sessions.list(), context.previous);
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.sessions.all() });
    },
  });
}

export function useDeleteSession() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => apiClient.delete(`/api/chat/sessions/${id}`),
    onMutate: async (id) => {
      await queryClient.cancelQueries({ queryKey: queryKeys.sessions.list() });
      const previous = queryClient.getQueryData<SessionMeta[]>(queryKeys.sessions.list());
      queryClient.setQueryData<SessionMeta[]>(queryKeys.sessions.list(), (old) =>
        old?.filter((s) => s.id !== id),
      );
      return { previous };
    },
    onError: (_err, _vars, context) => {
      if (context?.previous) {
        queryClient.setQueryData(queryKeys.sessions.list(), context.previous);
      }
    },
    onSettled: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.sessions.all() });
    },
  });
}
