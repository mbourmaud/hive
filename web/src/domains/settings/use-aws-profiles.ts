import { useQuery } from "@tanstack/react-query";
import { apiClient } from "@/shared/api/client";
import { queryKeys } from "@/shared/api/query-keys";
import type { AwsProfileInfo } from "./types";

export function useAwsProfiles(enabled: boolean) {
  return useQuery({
    queryKey: queryKeys.aws.profiles(),
    queryFn: () => apiClient.get<AwsProfileInfo[]>("/api/aws/profiles"),
    enabled,
    staleTime: 30_000,
  });
}
