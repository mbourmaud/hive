import { ArrowDown } from "lucide-react";
import { useCallback, useEffect, useMemo, useState } from "react";
import beeIcon from "@/assets/bee-icon.png";
import type { EffortLevel } from "@/domains/settings/store";
import type { Model } from "@/domains/settings/types";
import { isDialogOpen, isEditingElement, useKeybinds } from "@/shared/hooks/use-keybinds";
import type { ChatTurn, ContextUsage, ImageAttachment } from "../../types";
import { DroneStatusCard } from "../drone-status-card";
import { PromptInput } from "../prompt-input";
import { SessionTurn } from "../session-turn";
import "../chat-layout.css";

import { SUGGESTION_PROMPTS } from "./constants";
import { useAutoScroll, useProgressiveRender, useScrollButtonVisibility } from "./hooks";

// ── Types ────────────────────────────────────────────────────────────────────

interface ChatLayoutProps {
  turns: ChatTurn[];
  isStreaming: boolean;
  error: string | null;
  currentTurnId: string | null;
  onSend: (message: string, images?: ImageAttachment[]) => void;
  onAbort: () => void;
  hasSession: boolean;
  models?: Model[];
  selectedModel?: string;
  onModelChange?: (modelId: string) => void;
  contextUsage?: ContextUsage | null;
  effort?: EffortLevel;
  onEffortChange?: (effort: EffortLevel) => void;
}

// ── Component ────────────────────────────────────────────────────────────────

export function ChatLayout({
  turns,
  isStreaming,
  error,
  currentTurnId,
  onSend,
  onAbort,
  hasSession,
  models,
  selectedModel,
  onModelChange,
  contextUsage,
  effort,
  onEffortChange,
}: ChatLayoutProps) {
  // Steps expansion state — track per-turn
  const [expandedSteps, setExpandedSteps] = useState<Set<string>>(new Set());

  const toggleSteps = useCallback((turnId: string) => {
    setExpandedSteps((prev) => {
      const next = new Set(prev);
      if (next.has(turnId)) {
        next.delete(turnId);
      } else {
        next.add(turnId);
      }
      return next;
    });
  }, []);

  // Auto-expand steps for the current streaming turn
  useEffect(() => {
    if (currentTurnId && isStreaming) {
      setExpandedSteps((prev) => {
        if (prev.has(currentTurnId)) return prev;
        const next = new Set(prev);
        next.add(currentTurnId);
        return next;
      });
    }
  }, [currentTurnId, isStreaming]);

  const { visibleTurns } = useProgressiveRender(turns);
  const { scrollRef, scrollToBottom } = useAutoScroll(turns, isStreaming);
  const { visible: showScrollBtn, newCount: newMessageCount } = useScrollButtonVisibility(
    scrollRef,
    turns.length,
  );

  // Current turn status for prompt input
  const currentTurn = turns.find((t) => t.id === currentTurnId);
  const turnStatus = currentTurn?.status ?? null;

  // ── j/k message navigation ──────────────────────────────────────────────
  const [focusedTurnIndex, setFocusedTurnIndex] = useState<number | null>(null);

  const focusPromptEditor = useCallback(() => {
    const editor = document.querySelector<HTMLElement>('[data-slot="prompt-editor"]');
    editor?.focus();
  }, []);

  useKeybinds(
    useMemo(
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
        {
          key: "Escape",
          handler: () => setFocusedTurnIndex(null),
          ignoreEditing: false,
        },
      ],
      [visibleTurns.length, focusPromptEditor],
    ),
  );

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

  if (!hasSession && turns.length === 0) {
    return (
      <div
        data-component="chat-view"
        className="flex-1 flex flex-col relative overflow-hidden bg-background"
      >
        <div className="flex-1 flex flex-col items-center justify-center gap-6 px-4">
          <img src={beeIcon} alt="Hive" data-slot="empty-state-bee" />
          <div className="text-center">
            <p className="text-lg font-medium text-foreground">What can I help you build?</p>
            <p className="text-sm text-muted-foreground mt-1">
              Ask anything. Claude Code will help you code, debug, and ship.
            </p>
          </div>
          <div data-slot="empty-state-suggestions">
            {SUGGESTION_PROMPTS.map((suggestion) => (
              <button
                key={suggestion.label}
                type="button"
                data-slot="empty-state-suggestion-card"
                onClick={() => onSend(suggestion.prompt)}
              >
                <span className="text-xs font-medium text-foreground">{suggestion.label}</span>
                <span className="text-[11px] text-muted-foreground mt-0.5">{suggestion.hint}</span>
              </button>
            ))}
          </div>
          <div className="flex items-center gap-4 text-[11px] text-muted-foreground/60">
            <span>
              <kbd className="px-1 py-0.5 rounded bg-muted border border-border text-[10px] font-mono">
                Enter
              </kbd>{" "}
              to send
            </span>
            <span>
              <kbd className="px-1 py-0.5 rounded bg-muted border border-border text-[10px] font-mono">
                Cmd+N
              </kbd>{" "}
              new session
            </span>
            <span>
              <kbd className="px-1 py-0.5 rounded bg-muted border border-border text-[10px] font-mono">
                Esc
              </kbd>{" "}
              to stop
            </span>
          </div>
        </div>
        <PromptInput
          onSend={onSend}
          onAbort={onAbort}
          isStreaming={isStreaming}
          error={error}
          turnStatus={turnStatus}
          models={models}
          selectedModel={selectedModel}
          onModelChange={onModelChange}
          contextUsage={contextUsage}
          effort={effort}
          onEffortChange={onEffortChange}
        />
      </div>
    );
  }

  return (
    <div
      data-component="chat-view"
      className="flex-1 flex flex-col relative overflow-hidden bg-background"
    >
      <div ref={scrollRef} data-slot="message-list" className="flex-1 overflow-y-auto">
        <div
          className="max-w-[900px] mx-auto px-4 sm:px-6 pt-4 pb-[calc(var(--prompt-height,8rem)+64px)]"
          data-slot="message-list-inner"
        >
          {visibleTurns.map((turn, idx) =>
            turn.droneName ? (
              <DroneStatusCard key={turn.id} droneName={turn.droneName} prompt={turn.userMessage} />
            ) : (
              <SessionTurn
                key={turn.id}
                turn={turn}
                isLast={idx === visibleTurns.length - 1}
                stepsExpanded={expandedSteps.has(turn.id)}
                onToggleSteps={() => toggleSteps(turn.id)}
                isFocused={focusedTurnIndex === idx}
              />
            ),
          )}
        </div>
      </div>

      <button
        type="button"
        data-slot="scroll-to-bottom"
        data-visible={showScrollBtn ? "" : undefined}
        onClick={() => scrollToBottom(true)}
        aria-hidden={!showScrollBtn}
      >
        <ArrowDown className="h-3.5 w-3.5" />
        {newMessageCount > 0 ? `${newMessageCount} new` : "Scroll to bottom"}
      </button>

      <PromptInput
        onSend={onSend}
        onAbort={onAbort}
        isStreaming={isStreaming}
        disabled={false}
        error={error}
        turnStatus={turnStatus}
        models={models}
        selectedModel={selectedModel}
        onModelChange={onModelChange}
        contextUsage={contextUsage}
        effort={effort}
        onEffortChange={onEffortChange}
      />
    </div>
  );
}
