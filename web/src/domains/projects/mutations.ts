import { useMutation, useQueryClient } from "@tanstack/react-query";
import { apiClient } from "@/shared/api/client";
import { queryKeys } from "@/shared/api/query-keys";
import type { CreateProjectRequest, ProjectProfile, UpdateProjectRequest } from "./types";

export function useCreateProject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (params: CreateProjectRequest) =>
      apiClient.post<ProjectProfile>("/api/registry/projects", params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.registry.all() });
    },
  });
}

export function useUpdateProject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({ id, ...params }: UpdateProjectRequest & { id: string }) =>
      apiClient.put<ProjectProfile>(`/api/registry/projects/${id}`, params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.registry.all() });
    },
  });
}

export function useDeleteProject() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: string) => apiClient.delete(`/api/registry/projects/${id}`),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.registry.all() });
    },
  });
}

export function useUploadProjectImage() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: async ({ id, file }: { id: string; file: File }) => {
      const formData = new FormData();
      formData.append("image", file);
      const res = await fetch(`/api/registry/projects/${id}/image`, {
        method: "POST",
        body: formData,
      });
      if (!res.ok) throw new Error(await res.text());
      return apiClient.get<ProjectProfile>(`/api/registry/projects/${id}`);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.registry.all() });
    },
  });
}
