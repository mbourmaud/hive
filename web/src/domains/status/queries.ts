import { useQuery } from "@tanstack/react-query";
import { apiClient } from "@/shared/api/client";
import { queryKeys } from "@/shared/api/query-keys";
import type { OverallHealth, SystemStatus } from "./types";

export function useStatusQuery(enabled: boolean) {
  return useQuery({
    queryKey: queryKeys.status.all(),
    queryFn: () => apiClient.get<SystemStatus>("/api/status"),
    staleTime: 5_000,
    refetchInterval: enabled ? 5_000 : 30_000,
  });
}

export function deriveHealth(status: SystemStatus | undefined): OverallHealth {
  if (!status) return "unknown";

  // Error: auth not configured or expired
  if (!status.auth.configured || status.auth.expired) return "error";

  // Warning: any drone is stuck
  if (status.drones.some((d) => d.is_stuck)) return "warning";

  return "healthy";
}
