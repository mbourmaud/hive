import { useMemo } from "react";
import { type KeyBinding, useKeybinds } from "./use-keybinds";

// ── Types ────────────────────────────────────────────────────────────────────

interface KeyboardShortcutHandlers {
  onNewSession: () => void;
  onOpenSettings: () => void;
  onOpenCommandPalette: () => void;
  onOpenSessions: () => void;
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
  onOpenSessions,
  onDeleteSession,
  onAbort,
  onToggleDronePanel,
  isStreaming,
}: KeyboardShortcutHandlers): void {
  const bindings: KeyBinding[] = useMemo(
    () => [
      {
        key: "Escape",
        handler: () => {
          if (isStreaming) {
            onAbort();
            return;
          }
          const active = document.activeElement;
          if (active instanceof HTMLElement && active.isContentEditable) {
            active.blur();
          }
        },
        ignoreEditing: false,
      },
      { key: "n", mod: true, handler: onNewSession },
      { key: ",", mod: true, handler: onOpenSettings },
      { key: "k", mod: true, handler: onOpenCommandPalette },
      { key: "e", mod: true, handler: onOpenSessions },
      { key: "b", mod: true, handler: () => onToggleDronePanel?.() },
      { key: "Backspace", mod: true, shift: true, handler: onDeleteSession },
    ],
    [
      onNewSession,
      onOpenSettings,
      onOpenCommandPalette,
      onOpenSessions,
      onDeleteSession,
      onAbort,
      onToggleDronePanel,
      isStreaming,
    ],
  );

  useKeybinds(bindings);
}
