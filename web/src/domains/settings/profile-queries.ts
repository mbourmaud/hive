import { queryOptions, useQuery } from "@tanstack/react-query";
import { apiClient } from "@/shared/api/client";
import { queryKeys } from "@/shared/api/query-keys";
import type { ActiveProfile, ProfileInfo } from "./types";

export const profilesQueryOptions = queryOptions({
  queryKey: queryKeys.profiles.all(),
  queryFn: () => apiClient.get<ProfileInfo[]>("/api/profiles"),
  staleTime: 10_000,
});

export const activeProfileQueryOptions = queryOptions({
  queryKey: queryKeys.profiles.active(),
  queryFn: () => apiClient.get<ActiveProfile>("/api/profiles/active"),
  staleTime: 10_000,
});

export function useProfilesQuery() {
  return useQuery(profilesQueryOptions);
}

export function useActiveProfileQuery() {
  return useQuery(activeProfileQueryOptions);
}
