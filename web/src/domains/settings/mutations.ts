import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "@/shared/api/client";
import { queryKeys } from "@/shared/api/query-keys";
import type { OAuthAuthorizeResponse } from "./types";

// ── Mutations ────────────────────────────────────────────────────────────────

export function useSetupApiKey() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (apiKey: string) => apiClient.post<unknown>("/api/auth/setup", { api_key: apiKey }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.auth.status() });
      queryClient.invalidateQueries({ queryKey: queryKeys.auth.models() });
    },
  });
}

export function useStartOAuth() {
  return useMutation({
    mutationFn: () => apiClient.get<OAuthAuthorizeResponse>("/api/auth/oauth/authorize"),
  });
}

export function useCompleteOAuth() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (code: string) => apiClient.post<unknown>("/api/auth/oauth/callback", { code }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.auth.status() });
      queryClient.invalidateQueries({ queryKey: queryKeys.auth.models() });
    },
  });
}

export function useLogout() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: () => apiClient.delete("/api/auth/logout"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.auth.status() });
    },
  });
}
