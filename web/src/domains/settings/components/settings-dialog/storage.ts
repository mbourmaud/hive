// ── Font family presets ──────────────────────────────────────────────────────

export interface FontPreset {
  id: string;
  label: string;
  value: string;
}

export const SANS_FONTS: FontPreset[] = [
  { id: "inter", label: "Inter", value: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif" },
  { id: "system", label: "System UI", value: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif" },
  { id: "geist", label: "Geist Sans", value: "'Geist', -apple-system, BlinkMacSystemFont, sans-serif" },
  { id: "dm-sans", label: "DM Sans", value: "'DM Sans', -apple-system, sans-serif" },
  { id: "space-grotesk", label: "Space Grotesk", value: "'Space Grotesk', -apple-system, sans-serif" },
  { id: "plus-jakarta", label: "Plus Jakarta Sans", value: "'Plus Jakarta Sans', -apple-system, sans-serif" },
];

export const HEADING_FONTS: FontPreset[] = [
  { id: "inherit", label: "Same as body", value: "var(--font-sans)" },
  { id: "inter", label: "Inter", value: "'Inter', -apple-system, BlinkMacSystemFont, sans-serif" },
  { id: "geist", label: "Geist Sans", value: "'Geist', -apple-system, sans-serif" },
  { id: "dm-sans", label: "DM Sans", value: "'DM Sans', -apple-system, sans-serif" },
  { id: "space-grotesk", label: "Space Grotesk", value: "'Space Grotesk', -apple-system, sans-serif" },
  { id: "plus-jakarta", label: "Plus Jakarta Sans", value: "'Plus Jakarta Sans', -apple-system, sans-serif" },
];

export const MONO_FONTS: FontPreset[] = [
  { id: "ibm-plex", label: "IBM Plex Mono", value: "'IBM Plex Mono', 'JetBrains Mono', 'SF Mono', 'Fira Code', ui-monospace, monospace" },
  { id: "jetbrains", label: "JetBrains Mono", value: "'JetBrains Mono', 'SF Mono', 'Fira Code', ui-monospace, monospace" },
  { id: "fira-code", label: "Fira Code", value: "'Fira Code', 'JetBrains Mono', ui-monospace, monospace" },
  { id: "geist-mono", label: "Geist Mono", value: "'Geist Mono', 'SF Mono', ui-monospace, monospace" },
  { id: "sf-mono", label: "SF Mono", value: "'SF Mono', 'Fira Code', ui-monospace, monospace" },
  { id: "cascadia", label: "Cascadia Code", value: "'Cascadia Code', 'Fira Code', ui-monospace, monospace" },
];

// ── Types ────────────────────────────────────────────────────────────────────

export interface AppSettings {
  fontSize: number;
  fontSans: string;
  fontHeading: string;
  fontMono: string;
}

// ── Constants ────────────────────────────────────────────────────────────────

const SETTINGS_KEY = "hive-settings";
export const DEFAULT_FONT_SIZE = 14;
export const MIN_FONT_SIZE = 12;
export const MAX_FONT_SIZE = 18;
const DEFAULT_SANS = "inter";
const DEFAULT_HEADING = "inherit";
const DEFAULT_MONO = "ibm-plex";

// ── localStorage helpers ────────────────────────────────────────────────────

export function loadSettings(): AppSettings {
  try {
    const raw = localStorage.getItem(SETTINGS_KEY);
    if (!raw) return defaults();
    const parsed = JSON.parse(raw) as Record<string, unknown>;
    const fontSize = typeof parsed.fontSize === "number" ? parsed.fontSize : DEFAULT_FONT_SIZE;
    const fontSans = typeof parsed.fontSans === "string" ? parsed.fontSans : DEFAULT_SANS;
    const fontHeading = typeof parsed.fontHeading === "string" ? parsed.fontHeading : DEFAULT_HEADING;
    const fontMono = typeof parsed.fontMono === "string" ? parsed.fontMono : DEFAULT_MONO;
    return {
      fontSize: Math.min(MAX_FONT_SIZE, Math.max(MIN_FONT_SIZE, fontSize)),
      fontSans,
      fontHeading,
      fontMono,
    };
  } catch {
    return defaults();
  }
}

export function saveSettings(settings: AppSettings): void {
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings));
  } catch {
    // quota exceeded
  }
}

function defaults(): AppSettings {
  return {
    fontSize: DEFAULT_FONT_SIZE,
    fontSans: DEFAULT_SANS,
    fontHeading: DEFAULT_HEADING,
    fontMono: DEFAULT_MONO,
  };
}

/** Resolve a font preset ID to its CSS font-family value. */
export function resolveFontValue(id: string, presets: FontPreset[]): string {
  const preset = presets.find((p) => p.id === id);
  if (preset) return preset.value;
  const fallback = presets[0];
  return fallback ? fallback.value : "sans-serif";
}
