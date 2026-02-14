// ── Constants ────────────────────────────────────────────────────────────────

export const MAX_HISTORY = 100;
export const HISTORY_KEY = "hive-prompt-history";

// ── History helpers ──────────────────────────────────────────────────────────

export function loadHistory(): string[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (Array.isArray(parsed) && parsed.every((item): item is string => typeof item === "string")) {
      return parsed;
    }
    return [];
  } catch {
    return [];
  }
}

export function saveHistory(history: string[]): void {
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(history.slice(-MAX_HISTORY)));
  } catch {
    // quota exceeded — silently ignore
  }
}
