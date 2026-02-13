import { useEffect, useRef } from "react";

// ── Types ────────────────────────────────────────────────────────────────────

interface KeyboardShortcutHandlers {
  onNewSession: () => void;
  onOpenSettings: () => void;
  onOpenCommandPalette: () => void;
  onDeleteSession: () => void;
  onAbort: () => void;
  onToggleDronePanel?: () => void;
  isStreaming: boolean;
}

// ── Hook ─────────────────────────────────────────────────────────────────────

export function useKeyboardShortcuts({
  onNewSession,
  onOpenSettings,
  onOpenCommandPalette,
  onDeleteSession,
  onAbort,
  onToggleDronePanel,
  isStreaming,
}: KeyboardShortcutHandlers): void {
  const handlersRef = useRef({
    onNewSession,
    onOpenSettings,
    onOpenCommandPalette,
    onDeleteSession,
    onAbort,
    onToggleDronePanel,
    isStreaming,
  });

  handlersRef.current = {
    onNewSession,
    onOpenSettings,
    onOpenCommandPalette,
    onDeleteSession,
    onAbort,
    onToggleDronePanel,
    isStreaming,
  };

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      const meta = e.metaKey || e.ctrlKey;
      const h = handlersRef.current;

      // Escape — abort streaming or blur the editor for j/k navigation
      if (e.key === "Escape") {
        if (h.isStreaming) {
          e.preventDefault();
          h.onAbort();
          return;
        }
        // Blur editor so j/k navigation activates
        const active = document.activeElement;
        if (active instanceof HTMLElement && active.isContentEditable) {
          e.preventDefault();
          active.blur();
        }
        return;
      }

      if (!meta) return;

      // Cmd+N — new session
      if (e.key === "n" && !e.shiftKey) {
        e.preventDefault();
        h.onNewSession();
        return;
      }

      // Cmd+, — open settings
      if (e.key === ",") {
        e.preventDefault();
        h.onOpenSettings();
        return;
      }

      // Cmd+K — open command palette
      if (e.key === "k" && !e.shiftKey) {
        e.preventDefault();
        h.onOpenCommandPalette();
        return;
      }

      // Cmd+B — toggle drone panel
      if (e.key === "b" && !e.shiftKey) {
        e.preventDefault();
        h.onToggleDronePanel?.();
        return;
      }

      // Cmd+Shift+Backspace — delete current session
      if (e.key === "Backspace" && e.shiftKey) {
        e.preventDefault();
        h.onDeleteSession();
        return;
      }
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);
}
