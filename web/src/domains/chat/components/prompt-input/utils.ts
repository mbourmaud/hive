import type { TurnStatus } from "../../types";

// ── Constants ────────────────────────────────────────────────────────────────

export const PLACEHOLDER_INTERVAL_MS = 6000;

export const ROTATING_PLACEHOLDERS = [
  "Ask anything...",
  "Fix the failing test in auth.rs",
  "Explain how the event system works",
  "Add a dark mode toggle component",
  "Refactor this function to use async/await",
  "Write a migration for the users table",
  "What does this error mean?",
  "Help me debug the SSE connection",
  "Create a React hook for pagination",
  "Optimize this database query",
  "Add input validation to the form",
  "Review my PR for security issues",
  "Generate types from this API response",
  "Write unit tests for the parser",
];

// ── Status text derivation ───────────────────────────────────────────────────

export function deriveStatusText(
  isStreaming: boolean,
  turnStatus: TurnStatus | null | undefined,
  error: string | null | undefined,
): { text: string; variant: "ready" | "busy" | "error" } {
  if (error) {
    return { text: error, variant: "error" };
  }
  if (!isStreaming) {
    return { text: "Ready", variant: "ready" };
  }
  if (turnStatus === "pending") {
    return { text: "Thinking...", variant: "busy" };
  }
  return { text: "Running commands...", variant: "busy" };
}

// ── Extract plain text from contenteditable ─────────────────────────────────

export function getPlainText(el: HTMLDivElement): string {
  // innerText respects line breaks from <br> and block elements
  return el.innerText ?? "";
}

export function setPlainText(el: HTMLDivElement, text: string): void {
  el.textContent = text;
  // Move cursor to end
  if (text.length > 0) {
    const range = document.createRange();
    const sel = window.getSelection();
    range.selectNodeContents(el);
    range.collapse(false);
    sel?.removeAllRanges();
    sel?.addRange(range);
  }
}
