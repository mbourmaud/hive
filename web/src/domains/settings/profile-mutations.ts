import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "@/shared/api/client";
import { queryKeys } from "@/shared/api/query-keys";
import type { CreateProfileParams } from "./types";

export function useCreateProfile() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: CreateProfileParams) =>
      apiClient.post<unknown>("/api/profiles", params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.profiles.all() });
      queryClient.invalidateQueries({ queryKey: queryKeys.profiles.active() });
    },
  });
}

export function useActivateProfile() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (name: string) =>
      apiClient.post<unknown>("/api/profiles/activate", { name }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.profiles.all() });
      queryClient.invalidateQueries({ queryKey: queryKeys.profiles.active() });
      queryClient.invalidateQueries({ queryKey: queryKeys.auth.status() });
      queryClient.invalidateQueries({ queryKey: queryKeys.auth.models() });
    },
  });
}

export function useAwsSsoLogin() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (profile: string) =>
      apiClient.post<unknown>("/api/aws/sso-login", { profile }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.auth.status() });
      queryClient.invalidateQueries({ queryKey: queryKeys.auth.models() });
    },
  });
}

export function useDeleteProfile() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (name: string) => apiClient.delete(`/api/profiles/${name}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.profiles.all() });
      queryClient.invalidateQueries({ queryKey: queryKeys.profiles.active() });
      queryClient.invalidateQueries({ queryKey: queryKeys.auth.status() });
    },
  });
}
