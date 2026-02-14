export type WizardStep = 1 | 2 | 3;

export const STEP_HEADERS: Record<WizardStep, { title: string; subtitle: string }> = {
  1: { title: "Add your project", subtitle: "Configure your project to get started with Hive" },
  2: { title: "Setting up your project...", subtitle: "Detecting your project environment" },
  3: { title: "Ready!", subtitle: "Your project is configured and ready to go" },
};

// ── Folder picker ───────────────────────────────────────────────────────────

interface DirectoryPickerWindow {
  showDirectoryPicker: () => Promise<FileSystemDirectoryHandle>;
}

function hasDirectoryPicker(w: Window): w is Window & DirectoryPickerWindow {
  return "showDirectoryPicker" in w;
}

export async function pickFolder(): Promise<string | null> {
  if (!hasDirectoryPicker(window)) return null;
  try {
    const handle = await window.showDirectoryPicker();
    return handle.name;
  } catch {
    // User cancelled
    return null;
  }
}
