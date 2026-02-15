import { useQuery } from "@tanstack/react-query";
import { apiClient } from "@/shared/api/client";
import type { FileDiff, GitStatus } from "./types";

export function useGitStatus(projectPath: string | null) {
  return useQuery({
    queryKey: ["git", "status", projectPath],
    queryFn: () =>
      // Safe: queryFn only called when enabled=true, which guards against null
      apiClient.get<GitStatus>(`/api/git/status?project_path=${encodeURIComponent(projectPath!)}`),
    enabled: projectPath !== null,
    refetchInterval: 1_000,
    staleTime: 500,
  });
}

export function useFileDiff(projectPath: string | null, filePath: string | null, staged: boolean) {
  return useQuery({
    queryKey: ["git", "diff", projectPath, filePath, staged],
    queryFn: () =>
      // Safe: queryFn only called when enabled=true, which guards against null
      apiClient.get<FileDiff>(
        `/api/git/diff?project_path=${encodeURIComponent(projectPath!)}&file=${encodeURIComponent(filePath!)}&staged=${staged}`,
      ),
    enabled: projectPath !== null && filePath !== null,
    staleTime: 1_000,
  });
}
