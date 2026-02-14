import { useQuery } from "@tanstack/react-query";
import { apiClient } from "@/shared/api/client";
import { queryKeys } from "@/shared/api/query-keys";
import type { ProjectProfile } from "./types";

export function useProjectRegistryQuery() {
  return useQuery({
    queryKey: queryKeys.registry.all(),
    queryFn: () => apiClient.get<ProjectProfile[]>("/api/registry/projects"),
    staleTime: 30_000,
  });
}

export function useProjectProfileQuery(id: string | null) {
  return useQuery({
    queryKey: queryKeys.registry.detail(id ?? ""),
    queryFn: () => apiClient.get<ProjectProfile>(`/api/registry/projects/${id}`),
    enabled: id !== null,
  });
}
