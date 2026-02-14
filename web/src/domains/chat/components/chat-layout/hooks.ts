import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  isDialogOpen,
  isEditingElement,
  type KeyBinding,
  useKeybinds,
} from "@/shared/hooks/use-keybinds";
import type { ChatTurn } from "../../types";
import { INITIAL_RENDER_COUNT, SCROLL_THRESHOLD } from "./constants";

// ── Auto-scroll hook ─────────────────────────────────────────────────────────

export function useAutoScroll(turns: ChatTurn[], isStreaming: boolean) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const isUserScrolling = useRef(false);
  const wasAtBottom = useRef(true);

  const isAtBottom = useCallback(() => {
    const el = scrollRef.current;
    if (!el) return true;
    return el.scrollHeight - el.scrollTop - el.clientHeight < SCROLL_THRESHOLD;
  }, []);

  const scrollToBottom = useCallback((smooth = true) => {
    const el = scrollRef.current;
    if (!el) return;
    el.scrollTo({
      top: el.scrollHeight,
      behavior: smooth ? "smooth" : "instant",
    });
    isUserScrolling.current = false;
    wasAtBottom.current = true;
  }, []);

  // Detect user scroll gestures
  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;

    const onWheel = () => {
      isUserScrolling.current = true;
    };
    const onTouchStart = () => {
      isUserScrolling.current = true;
    };
    const onScroll = () => {
      wasAtBottom.current = isAtBottom();
      if (wasAtBottom.current) {
        isUserScrolling.current = false;
      }
    };

    el.addEventListener("wheel", onWheel, { passive: true });
    el.addEventListener("touchstart", onTouchStart, { passive: true });
    el.addEventListener("scroll", onScroll, { passive: true });

    return () => {
      el.removeEventListener("wheel", onWheel);
      el.removeEventListener("touchstart", onTouchStart);
      el.removeEventListener("scroll", onScroll);
    };
  }, [isAtBottom]);

  // Derive a content fingerprint that changes when new content arrives
  const lastTurn = turns[turns.length - 1];
  const contentSignal = lastTurn
    ? `${turns.length}:${lastTurn.id}:${lastTurn.assistantParts.length}`
    : "0";

  // Auto-scroll on new content if user hasn't scrolled away
  useEffect(() => {
    if (!isUserScrolling.current && wasAtBottom.current) {
      scrollToBottom(false);
    }
  }, [scrollToBottom, contentSignal, isStreaming]);

  return { scrollRef, scrollToBottom };
}

// ── Progressive rendering hook ───────────────────────────────────────────────

export function useProgressiveRender(turns: ChatTurn[]) {
  const [renderCount, setRenderCount] = useState(INITIAL_RENDER_COUNT);

  // Reset when turns array changes drastically (new session)
  useEffect(() => {
    if (turns.length <= INITIAL_RENDER_COUNT) {
      setRenderCount(INITIAL_RENDER_COUNT);
    }
  }, [turns.length]);

  // Backfill older turns via requestIdleCallback
  useEffect(() => {
    if (renderCount >= turns.length) return;

    const id = requestIdleCallback(
      () => {
        setRenderCount((prev) => Math.min(prev + 10, turns.length));
      },
      { timeout: 500 },
    );

    return () => cancelIdleCallback(id);
  }, [renderCount, turns.length]);

  const visibleTurns = useMemo(() => {
    if (turns.length <= renderCount) return turns;
    const startIdx = Math.max(0, turns.length - renderCount);
    return turns.slice(startIdx);
  }, [turns, renderCount]);

  return { visibleTurns, isBackfilling: renderCount < turns.length };
}

// ── Scroll button state hook (separate to avoid re-renders) ──────────────────

export function useScrollButtonVisibility(
  scrollRef: React.RefObject<HTMLDivElement | null>,
  turnCount: number,
) {
  const [visible, setVisible] = useState(false);
  const [newCount, setNewCount] = useState(0);
  const turnCountWhenScrolledAway = useRef(turnCount);
  const wasVisible = useRef(false);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;

    const onScroll = () => {
      const atBottom = el.scrollHeight - el.scrollTop - el.clientHeight < SCROLL_THRESHOLD;
      const nowVisible = !atBottom;

      if (nowVisible && !wasVisible.current) {
        // User just scrolled away — snapshot the turn count
        turnCountWhenScrolledAway.current = turnCount;
      }

      if (!nowVisible) {
        // User returned to bottom — reset count
        setNewCount(0);
      }

      wasVisible.current = nowVisible;
      setVisible(nowVisible);
    };

    el.addEventListener("scroll", onScroll, { passive: true });
    return () => el.removeEventListener("scroll", onScroll);
  }, [scrollRef, turnCount]);

  // Update new count when turns arrive while scrolled away
  useEffect(() => {
    if (visible) {
      const delta = turnCount - turnCountWhenScrolledAway.current;
      if (delta > 0) {
        setNewCount(delta);
      }
    }
  }, [visible, turnCount]);

  return { visible, newCount };
}

// ── j/k message navigation hook ──────────────────────────────────────────────

export function useMessageNavigation(visibleTurns: ChatTurn[]) {
  const [focusedTurnIndex, setFocusedTurnIndex] = useState<number | null>(null);

  const focusPromptEditor = useCallback(() => {
    const editor = document.querySelector<HTMLElement>('[data-slot="prompt-editor"]');
    editor?.focus();
  }, []);

  const bindings: KeyBinding[] = useMemo(
    () => [
      {
        key: "j",
        handler: () =>
          setFocusedTurnIndex((prev) => {
            const max = visibleTurns.length - 1;
            if (prev === null) return 0;
            return Math.min(prev + 1, max);
          }),
      },
      {
        key: "ArrowDown",
        handler: () =>
          setFocusedTurnIndex((prev) => {
            const max = visibleTurns.length - 1;
            if (prev === null) return 0;
            return Math.min(prev + 1, max);
          }),
      },
      {
        key: "k",
        handler: () =>
          setFocusedTurnIndex((prev) => {
            if (prev === null) return visibleTurns.length - 1;
            return Math.max(prev - 1, 0);
          }),
      },
      {
        key: "ArrowUp",
        handler: () =>
          setFocusedTurnIndex((prev) => {
            if (prev === null) return visibleTurns.length - 1;
            return Math.max(prev - 1, 0);
          }),
      },
      {
        key: "i",
        handler: () => {
          setFocusedTurnIndex(null);
          focusPromptEditor();
        },
      },
      { key: "Escape", handler: () => setFocusedTurnIndex(null), ignoreEditing: false },
    ],
    [visibleTurns.length, focusPromptEditor],
  );

  useKeybinds(bindings);

  // Auto-focus prompt on printable keypress
  useEffect(() => {
    function handlePrintableKey(e: KeyboardEvent) {
      if (e.key.length !== 1 || e.ctrlKey || e.metaKey || e.altKey) return;
      if (isDialogOpen()) return;
      if (isEditingElement(document.activeElement)) return;
      focusPromptEditor();
    }
    document.addEventListener("keydown", handlePrintableKey);
    return () => document.removeEventListener("keydown", handlePrintableKey);
  }, [focusPromptEditor]);

  // Scroll focused turn into view
  useEffect(() => {
    if (focusedTurnIndex === null) return;
    const turn = visibleTurns[focusedTurnIndex];
    if (!turn) return;
    const el = document.querySelector<HTMLElement>(
      `[data-component="session-turn"][data-turn-id="${turn.id}"]`,
    );
    if (el) {
      el.scrollIntoView({ behavior: "smooth", block: "center" });
    }
  }, [focusedTurnIndex, visibleTurns]);

  return { focusedTurnIndex };
}
