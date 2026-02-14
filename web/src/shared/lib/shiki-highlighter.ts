import type { HighlighterCore } from "shiki";

let highlighter: HighlighterCore | null = null;
let initPromise: Promise<HighlighterCore> | null = null;

const LANGUAGES = [
  "typescript",
  "tsx",
  "javascript",
  "jsx",
  "json",
  "css",
  "html",
  "rust",
  "python",
  "go",
  "bash",
  "markdown",
  "yaml",
  "toml",
  "sql",
  "diff",
] as const;

const THEMES = ["github-light", "github-dark"] as const;

export async function getHighlighter(): Promise<HighlighterCore> {
  if (highlighter) return highlighter;
  if (initPromise) return initPromise;

  initPromise = (async () => {
    const { createHighlighter } = await import("shiki");
    const instance = await createHighlighter({
      themes: [...THEMES],
      langs: [...LANGUAGES],
    });
    highlighter = instance;
    return instance;
  })();

  return initPromise;
}

export function getThemeName(): "github-light" | "github-dark" {
  const attr = document.documentElement.getAttribute("data-theme");
  return attr === "light" ? "github-light" : "github-dark";
}

/** Supported language aliases for shiki */
const LANG_ALIASES: Record<string, string> = {
  ts: "typescript",
  tsx: "tsx",
  js: "javascript",
  jsx: "jsx",
  sh: "bash",
  shell: "bash",
  zsh: "bash",
  yml: "yaml",
  md: "markdown",
  rs: "rust",
  py: "python",
};

export function resolveLanguage(lang: string | undefined): string {
  if (!lang) return "text";
  const lower = lang.toLowerCase();
  const resolved = LANG_ALIASES[lower] ?? lower;
  const supported = LANGUAGES as readonly string[];
  return supported.includes(resolved) ? resolved : "text";
}
