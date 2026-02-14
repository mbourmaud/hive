// ── Cursor position helpers for contenteditable ──────────────────────────────

function getPlainText(el: HTMLDivElement): string {
  return el.innerText ?? "";
}

/**
 * Check whether the caret is at the very start of the editor (offset 0).
 */
export function isAtStartOfEditor(el: HTMLDivElement): boolean {
  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0) return false;

  const range = sel.getRangeAt(0);
  return (
    range.collapsed &&
    range.startOffset === 0 &&
    (range.startContainer === el || range.startContainer === el.firstChild)
  );
}

/**
 * Check whether the caret is at the very end of the editor.
 */
export function isAtEndOfEditor(el: HTMLDivElement): boolean {
  const sel = window.getSelection();
  if (!sel || sel.rangeCount === 0) return false;

  const range = sel.getRangeAt(0);
  const textLen = getPlainText(el).length;
  const node = range.startContainer;

  const atEnd =
    range.collapsed &&
    ((node === el && range.startOffset === el.childNodes.length) ||
      (node.nodeType === Node.TEXT_NODE &&
        range.startOffset === (node.textContent?.length ?? 0) &&
        !node.nextSibling));

  return atEnd || textLen === 0;
}

// ── Slash popover key handling ──────────────────────────────────────────────

/**
 * Returns `true` if the key event was consumed by the slash popover.
 */
export function handleSlashPopoverKeys(
  e: React.KeyboardEvent<HTMLDivElement>,
  slashVisible: boolean,
  setSlashVisible: (v: boolean) => void,
): boolean {
  if (!slashVisible) return false;

  if (
    e.key === "ArrowDown" ||
    e.key === "ArrowUp" ||
    e.key === "Tab" ||
    (e.key === "Enter" && !e.shiftKey)
  ) {
    // SlashPopover's document-level handler will handle these
    return true;
  }

  if (e.key === "Escape") {
    e.preventDefault();
    setSlashVisible(false);
    return true;
  }

  return false;
}

// ── History navigation ──────────────────────────────────────────────────────

export interface HistoryNavState {
  historyRef: React.RefObject<string[]>;
  historyIndex: number;
  setHistoryIndex: (idx: number) => void;
  value: string;
  setValue: (val: string) => void;
  draftValue: string;
  setDraftValue: (val: string) => void;
  setPlainText: (el: HTMLDivElement, text: string) => void;
}

/**
 * Handle ArrowUp at the start of the editor — cycle history backward.
 * Returns `true` if handled.
 */
export function handleHistoryUp(
  e: React.KeyboardEvent<HTMLDivElement>,
  el: HTMLDivElement | null,
  state: HistoryNavState,
): boolean {
  if (e.key !== "ArrowUp" || !el) return false;
  if (!isAtStartOfEditor(el)) return false;

  e.preventDefault();
  const history = state.historyRef.current;
  if (history.length === 0) return true;

  const newIndex =
    state.historyIndex === -1 ? history.length - 1 : Math.max(0, state.historyIndex - 1);

  if (state.historyIndex === -1) {
    state.setDraftValue(state.value);
  }

  state.setHistoryIndex(newIndex);
  const newVal = history[newIndex] ?? "";
  state.setValue(newVal);
  state.setPlainText(el, newVal);
  return true;
}

/**
 * Handle ArrowDown at the end of the editor — cycle history forward.
 * Returns `true` if handled.
 */
export function handleHistoryDown(
  e: React.KeyboardEvent<HTMLDivElement>,
  el: HTMLDivElement | null,
  state: HistoryNavState,
): boolean {
  if (e.key !== "ArrowDown" || !el) return false;
  if (state.historyIndex === -1) return false;
  if (!isAtEndOfEditor(el)) return false;

  e.preventDefault();
  const history = state.historyRef.current;
  const newIndex = state.historyIndex + 1;

  if (newIndex >= history.length) {
    state.setHistoryIndex(-1);
    state.setValue(state.draftValue);
    state.setPlainText(el, state.draftValue);
  } else {
    state.setHistoryIndex(newIndex);
    const newVal = history[newIndex] ?? "";
    state.setValue(newVal);
    state.setPlainText(el, newVal);
  }
  return true;
}
