import { queryOptions, useQuery } from "@tanstack/react-query";
import { apiClient } from "@/shared/api/client";
import { queryKeys } from "@/shared/api/query-keys";
import type { AuthStatus, Model } from "./types";

// ── Query Options ────────────────────────────────────────────────────────────

export const authStatusQueryOptions = queryOptions({
  queryKey: queryKeys.auth.status(),
  queryFn: () => apiClient.get<AuthStatus>("/api/auth/status"),
  staleTime: 30_000,
});

export const modelsQueryOptions = queryOptions({
  queryKey: queryKeys.auth.models(),
  queryFn: () => apiClient.get<Model[]>("/api/models"),
  staleTime: 60_000,
});

// ── Hooks ────────────────────────────────────────────────────────────────────

export function useAuthStatusQuery() {
  return useQuery(authStatusQueryOptions);
}

export function useModelsQuery() {
  return useQuery(modelsQueryOptions);
}
