// ── Types ────────────────────────────────────────────────────────────────────

export interface AppSettings {
  fontSize: number;
}

// ── Constants ────────────────────────────────────────────────────────────────

const SETTINGS_KEY = "hive-settings";
export const DEFAULT_FONT_SIZE = 14;
export const MIN_FONT_SIZE = 12;
export const MAX_FONT_SIZE = 18;

// ── localStorage helpers ────────────────────────────────────────────────────

export function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) return { fontSize: DEFAULT_FONT_SIZE };
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const fontSize = typeof parsed.fontSize === "number" ? parsed.fontSize : DEFAULT_FONT_SIZE;
    return { fontSize: Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, fontSize)) };
  } catch {
    return { fontSize: DEFAULT_FONT_SIZE };
  }
}

export function saveSettings(settings: AppSettings): void {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // quota exceeded
  }
}
