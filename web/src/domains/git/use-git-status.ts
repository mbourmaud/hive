import { useQuery } from "@tanstack/react-query";
import { apiClient } from "@/shared/api/client";
import type { FileDiff, GitStatus } from "./types";

export function useGitStatus(projectPath: string | null) {
  return useQuery({
    queryKey: ["git", "status", projectPath],
    queryFn: () => {
      if (projectPath === null) throw new Error("Cannot fetch git status without project path");
      return apiClient.get<GitStatus>(
        `/api/git/status?project_path=${encodeURIComponent(projectPath)}`,
      );
    },
    enabled: projectPath !== null,
    refetchInterval: 5_000,
    staleTime: 3_000,
  });
}

export function useFileDiff(projectPath: string | null, filePath: string | null, staged: boolean) {
  return useQuery({
    queryKey: ["git", "diff", projectPath, filePath, staged],
    queryFn: () => {
      if (projectPath === null || filePath === null)
        throw new Error("Cannot fetch diff without paths");
      return apiClient.get<FileDiff>(
        `/api/git/diff?project_path=${encodeURIComponent(projectPath)}&file=${encodeURIComponent(filePath)}&staged=${staged}`,
      );
    },
    enabled: projectPath !== null && filePath !== null,
    staleTime: 3_000,
  });
}
