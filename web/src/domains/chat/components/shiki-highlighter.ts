import { createHighlighterCore, type HighlighterCore } from "shiki/core";
import { createOnigurumaEngine } from "shiki/engine/oniguruma";

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

let highlighterPromise: Promise<HighlighterCore> | null = null;

export async function getHighlighter(): Promise<HighlighterCore> {
  if (highlighterPromise) {
    return highlighterPromise;
  }

  highlighterPromise = (async () => {
    const highlighter = await createHighlighterCore({
      themes: await Promise.all([
        import("shiki/themes/github-light.mjs"),
        import("shiki/themes/github-dark.mjs"),
      ]),
      langs: await Promise.all(
        LANGUAGES.map((lang) => import(`shiki/langs/${lang}.mjs`))
      ),
      engine: createOnigurumaEngine(import("shiki/wasm")),
    });

    return highlighter;
  })();

  return highlighterPromise;
}

export function getThemeName(): "github-light" | "github-dark" {
  const theme = document.documentElement.getAttribute("data-theme");
  return theme === "dark" ? "github-dark" : "github-light";
}

const LANGUAGE_EXTENSIONS: Record<string, string> = {
  ts: "typescript",
  tsx: "tsx",
  js: "javascript",
  jsx: "jsx",
  json: "json",
  css: "css",
  html: "html",
  htm: "html",
  rs: "rust",
  py: "python",
  go: "go",
  sh: "bash",
  bash: "bash",
  zsh: "bash",
  md: "markdown",
  yml: "yaml",
  yaml: "yaml",
  toml: "toml",
  sql: "sql",
  diff: "diff",
  patch: "diff",
};

export function resolveLanguage(lang: string | undefined): string {
  if (!lang) return "text";
  const normalized = lang.toLowerCase();
  return LANGUAGE_EXTENSIONS[normalized] ?? normalized;
}
