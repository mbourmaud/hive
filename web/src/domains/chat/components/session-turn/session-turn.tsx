import "./session-turn.css";
import "./summary.css";
import "./animations.css";

import { ChevronDown, Loader2 } from "lucide-react";
import { useState } from "react";
import { cn } from "@/shared/lib/utils";
import { useTurnData } from "../../hooks/use-turn-data";
import type { ChatTurn } from "../../types";
import { MarkdownRenderer } from "../markdown-renderer";

// Trigger side-effect registration of all parts/ renderers
import "../parts";

import { useElapsed, useDebouncedStatus, useStickyHeight } from "./hooks";
import { CopyButton, PartRenderer } from "./step-renderers";
import { formatDuration } from "./tool-utils";

// ── Constants ────────────────────────────────────────────────────────────────

const COLLAPSE_CHAR_THRESHOLD = 200;

// ── Sub-components ────────────────────────────────────────────────────────────

function StepsTrigger({
  stepsCount,
  stepsExpanded,
  isStreaming,
  statusLabel,
  displayDuration,
  onToggle,
}: {
  stepsCount: number;
  stepsExpanded: boolean;
  isStreaming: boolean;
  statusLabel: string;
  displayDuration: string | null;
  onToggle: () => void;
}) {
  if (stepsCount === 0) return null;

  const label = isStreaming
    ? `${stepsCount} ${stepsCount === 1 ? "step" : "steps"}`
    : stepsExpanded
      ? "hide steps"
      : "show steps";

  return (
    <button type="button" data-slot="steps-trigger" onClick={onToggle} aria-expanded={stepsExpanded}>
      <div data-slot="steps-trigger-left">
        {isStreaming ? (
          <Loader2 className="h-3.5 w-3.5 animate-spin text-accent" />
        ) : (
          <ChevronDown
            className={cn(
              "h-3.5 w-3.5 text-muted-foreground transition-transform duration-150",
              !stepsExpanded && "-rotate-90",
            )}
          />
        )}
        <span data-slot="steps-trigger-count">{label}</span>
        {isStreaming && <span data-slot="steps-trigger-status">{statusLabel}</span>}
      </div>
      {displayDuration && <span data-slot="steps-trigger-duration">{displayDuration}</span>}
    </button>
  );
}

const FINISH_REASON_LABELS: Record<string, string> = {
  error: "error",
  canceled: "canceled",
};

function TurnFinishInfo({
  turn,
  displayDuration,
}: {
  turn: ChatTurn;
  displayDuration: string | null;
}) {
  if (turn.status === "streaming" || turn.status === "pending") return null;

  const reasonLabel = turn.finishReason && turn.finishReason !== "end_turn"
    ? (FINISH_REASON_LABELS[turn.finishReason] ?? "max tokens")
    : null;

  return (
    <div data-slot="turn-finish-info">
      {turn.model && <span data-slot="turn-finish-model">{turn.model}</span>}
      {turn.model && displayDuration && <span data-slot="turn-finish-sep">&middot;</span>}
      {displayDuration && <span data-slot="turn-finish-duration">{displayDuration}</span>}
      {reasonLabel && (
        <span data-slot="turn-finish-badge" data-reason={turn.finishReason}>
          {reasonLabel}
        </span>
      )}
    </div>
  );
}

// ── SessionTurn ──────────────────────────────────────────────────────────────

interface SessionTurnProps {
  turn: ChatTurn;
  isLast: boolean;
  stepsExpanded: boolean;
  onToggleSteps: () => void;
  isFocused?: boolean;
}

export function SessionTurn({
  turn,
  isLast,
  stepsExpanded,
  isFocused,
  onToggleSteps,
}: SessionTurnProps) {
  const [userExpanded, setUserExpanded] = useState(false);

  const isStreaming = turn.status === "streaming";
  const elapsed = useElapsed(turn.startedAt, isStreaming);
  const statusLabel = useDebouncedStatus(turn.assistantParts);
  const [stickyRef, stickyHeight] = useStickyHeight();

  const canExpandUser = turn.userMessage.length > COLLAPSE_CHAR_THRESHOLD;
  const { toolResultMap, stepsCount, summaryText, errorText, stepsParts } = useTurnData(turn);

  let displayDuration: string | null = null;
  if (turn.duration !== null) {
    displayDuration = formatDuration(turn.duration);
  } else if (isStreaming) {
    displayDuration = formatDuration(elapsed);
  }

  return (
    <div
      data-component="session-turn"
      data-turn-id={turn.id}
      data-status={turn.status}
      data-last={isLast || undefined}
      data-focused={isFocused ? "" : undefined}
      style={{ "--session-turn-sticky-height": `${stickyHeight}px` } as React.CSSProperties}
    >
      <div data-slot="turn-content">
        <div data-slot="turn-sticky" ref={stickyRef}>
          <div
            data-slot="user-message"
            data-can-expand={canExpandUser || undefined}
            data-expanded={userExpanded || undefined}
          >
            <p>{turn.userMessage}</p>
            {canExpandUser && (
              <button
                type="button"
                data-slot="user-message-expand"
                onClick={() => setUserExpanded((prev) => !prev)}
                aria-expanded={userExpanded}
              >
                <ChevronDown
                  className={cn(
                    "h-3.5 w-3.5 transition-transform duration-150",
                    userExpanded && "rotate-180",
                  )}
                />
              </button>
            )}
            <CopyButton text={turn.userMessage} slot="user-message-copy" />
          </div>

          <StepsTrigger
            stepsCount={stepsCount}
            stepsExpanded={stepsExpanded}
            isStreaming={isStreaming}
            statusLabel={statusLabel}
            displayDuration={displayDuration}
            onToggle={onToggleSteps}
          />
        </div>

        {stepsCount > 0 && stepsExpanded && (
          <div data-slot="steps-content">
            {stepsParts.map((part) => (
              <PartRenderer
                key={part.id}
                part={part}
                result={part.type === "tool_use" ? toolResultMap.get(part.id) : undefined}
              />
            ))}
          </div>
        )}

        {summaryText && (
          <div
            data-slot={isStreaming ? "turn-summary" : "turn-summary-section"}
            data-streaming={isStreaming ? "true" : undefined}
          >
            {isStreaming ? (
              <MarkdownRenderer text={summaryText} />
            ) : (
              <div data-slot="turn-summary" data-fade={isLast || undefined}>
                <MarkdownRenderer text={summaryText} />
              </div>
            )}
          </div>
        )}

        {errorText && (
          <div data-slot="turn-error">
            <pre>{errorText}</pre>
          </div>
        )}

        <TurnFinishInfo turn={turn} displayDuration={displayDuration} />
      </div>
    </div>
  );
}
