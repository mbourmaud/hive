import { useEffect, useRef } from "react";

// ── Types ────────────────────────────────────────────────────────────────────

export interface KeyBinding {
  /** Key value (e.g. "n", ",", "k", "Escape", "Backspace") */
  key: string;
  /** Requires Cmd (Mac) or Ctrl (Win/Linux) */
  mod?: boolean;
  /** Requires Shift */
  shift?: boolean;
  /** Handler to invoke when matched */
  handler: () => void;
  /**
   * Skip when focus is in an input/textarea/contenteditable.
   * Defaults to `true` — most shortcuts should not fire while editing.
   */
  ignoreEditing?: boolean;
}

// ── Helpers ──────────────────────────────────────────────────────────────────

export function isEditingElement(el: Element | null): boolean {
  if (!el) return false;
  if (el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement) return true;
  if (el instanceof HTMLElement && el.isContentEditable) return true;
  return false;
}

export function isDialogOpen(): boolean {
  return document.querySelector('[role="dialog"]') !== null;
}

/** Check whether a keyboard event matches a binding's key/modifier criteria. */
function matchesBinding(e: KeyboardEvent, binding: KeyBinding, meta: boolean): boolean {
  if (e.key !== binding.key) return false;
  if (binding.mod && !meta) return false;
  if (!binding.mod && meta) return false;
  if (binding.shift && !e.shiftKey) return false;
  if (binding.shift === false && e.shiftKey) return false;
  if (binding.ignoreEditing !== false && isEditingElement(document.activeElement)) return false;
  return true;
}

// ── Hook ─────────────────────────────────────────────────────────────────────

/**
 * Declarative keybind registry. Registers a single `keydown` listener on
 * `document` and iterates bindings to find the first match.
 *
 * Bindings are stored in a ref so the listener never needs to be re-attached.
 */
export function useKeybinds(bindings: KeyBinding[]): void {
  const bindingsRef = useRef(bindings);
  bindingsRef.current = bindings;

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.isComposing) return;
      const meta = e.metaKey || e.ctrlKey;

      for (const binding of bindingsRef.current) {
        if (matchesBinding(e, binding, meta)) {
          e.preventDefault();
          binding.handler();
          return;
        }
      }
    }

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);
}
