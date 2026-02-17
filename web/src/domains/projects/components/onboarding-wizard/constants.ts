import { apiClient } from "@/shared/api/client";

export type WizardStep = 1 | 2 | 3;

export const STEP_HEADERS: Record<WizardStep, { title: string; subtitle: string }> = {
  1: { title: "Add your project", subtitle: "Configure your project to get started with Hive" },
  2: { title: "Setting up your project...", subtitle: "Detecting your project environment" },
  3: { title: "Ready!", subtitle: "Your project is configured and ready to go" },
};

// ── Folder picker (native dialog via backend) ───────────────────────────────

interface PickFolderResult {
  path: string | null;
  name: string | null;
}

export async function pickFolder(): Promise<PickFolderResult> {
  try {
    const result = await apiClient.get<PickFolderResult>("/api/pick-folder");
    return result;
  } catch {
    return { path: null, name: null };
  }
}
