// ── Custom Theme — import/export JSON themes ─────────────────────────────────

/**
 * JSON schema for importable themes.
 * Each key maps to a CSS custom property on :root / [data-theme].
 * Values must be valid CSS color strings (OKLCH preferred).
 */
export interface CustomThemeColors {
  background: string;
  foreground: string;
  card: string;
  "card-foreground": string;
  "card-header": string;
  popover: string;
  "popover-foreground": string;
  primary: string;
  "primary-foreground": string;
  secondary: string;
  "secondary-foreground": string;
  muted: string;
  "muted-foreground": string;
  accent: string;
  "accent-foreground": string;
  destructive: string;
  "destructive-foreground": string;
  border: string;
  input: string;
  ring: string;
  success: string;
  honey: string;
  warning: string;
  sidebar: string;
  "sidebar-border": string;
  "surface-inset": string;
  "surface-raised": string;
}

export interface CustomThemeFile {
  name: string;
  dark: CustomThemeColors;
  light: CustomThemeColors;
}

/** Stored custom theme includes a generated slug id */
export interface StoredCustomTheme extends CustomThemeFile {
  id: string;
}

// ── Validation ──────────────────────────────────────────────────────────────

const REQUIRED_KEYS: readonly (keyof CustomThemeColors)[] = [
  "background",
  "foreground",
  "card",
  "card-foreground",
  "card-header",
  "popover",
  "popover-foreground",
  "primary",
  "primary-foreground",
  "secondary",
  "secondary-foreground",
  "muted",
  "muted-foreground",
  "accent",
  "accent-foreground",
  "destructive",
  "destructive-foreground",
  "border",
  "input",
  "ring",
  "success",
  "honey",
  "warning",
  "sidebar",
  "sidebar-border",
  "surface-inset",
  "surface-raised",
] as const;

export function validateThemeFile(
  data: unknown,
): { ok: true; theme: CustomThemeFile } | { ok: false; error: string } {
  if (typeof data !== "object" || data === null) {
    return { ok: false, error: "Theme file must be a JSON object" };
  }

  const obj = data as Record<string, unknown>;

  if (typeof obj.name !== "string" || obj.name.trim().length === 0) {
    return { ok: false, error: "Theme must have a non-empty 'name' field" };
  }

  for (const variant of ["dark", "light"] as const) {
    if (typeof obj[variant] !== "object" || obj[variant] === null) {
      return { ok: false, error: `Theme must have a '${variant}' object with color values` };
    }

    const colors = obj[variant] as Record<string, unknown>;
    for (const key of REQUIRED_KEYS) {
      if (typeof colors[key] !== "string") {
        return { ok: false, error: `Missing or invalid color '${key}' in '${variant}' palette` };
      }
    }
  }

  return { ok: true, theme: data as CustomThemeFile };
}

// ── Helpers ─────────────────────────────────────────────────────────────────

export function slugify(name: string): string {
  return (
    name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-|-$/g, "")
      .slice(0, 32) || "custom"
  );
}

export function generateThemeId(name: string): string {
  return `custom-${slugify(name)}-${Date.now().toString(36)}`;
}

/** Apply a custom theme's colors as CSS custom properties on :root */
export function applyCustomThemeColors(colors: CustomThemeColors): void {
  const root = document.documentElement;
  for (const key of REQUIRED_KEYS) {
    root.style.setProperty(`--${key}`, colors[key]);
  }
}

/** Remove custom theme CSS custom properties from :root (reset to stylesheet) */
export function clearCustomThemeColors(): void {
  const root = document.documentElement;
  for (const key of REQUIRED_KEYS) {
    root.style.removeProperty(`--${key}`);
  }
}

/** Export the current theme as a CustomThemeFile JSON blob */
export function exportCurrentTheme(name: string): CustomThemeFile {
  const root = document.documentElement;
  const computedStyle = getComputedStyle(root);

  function readColors(): CustomThemeColors {
    const colors: Record<string, string> = {};
    for (const key of REQUIRED_KEYS) {
      colors[key] = computedStyle.getPropertyValue(`--${key}`).trim();
    }
    return colors as unknown as CustomThemeColors;
  }

  // Read current mode's colors
  const currentTheme = root.getAttribute("data-theme") ?? "dark";
  const currentColors = readColors();

  // We only have one set of active values; both dark and light get the current
  // The user can manually edit the JSON later if they want different light/dark
  return {
    name,
    dark: currentTheme === "dark" ? currentColors : currentColors,
    light: currentTheme === "light" ? currentColors : currentColors,
  };
}
