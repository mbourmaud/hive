import "./session-turn.css";
import "./summary.css";
import "./animations.css";

import { CheckCircle2, ChevronDown, Loader2 } from "lucide-react";
import { useState } from "react";
import { cn } from "@/shared/lib/utils";
import { useAwsSsoLogin } from "@/domains/settings/profile-mutations";
import { useActiveProfileQuery } from "@/domains/settings/profile-queries";
import { useTurnData } from "../../hooks/use-turn-data";
import type { ChatTurn } from "../../types";
import { MarkdownRenderer } from "../markdown-renderer";

// Trigger side-effect registration of all parts/ renderers
import "../parts";

import { useDebouncedStatus, useElapsed, useStickyHeight } from "./hooks";
import { CopyButton, PartRenderer } from "./step-renderers";
import { formatDuration } from "./tool-utils";

// ── Constants ────────────────────────────────────────────────────────────────

const COLLAPSE_CHAR_THRESHOLD = 400;

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
    <button
      type="button"
      data-slot="steps-trigger"
      onClick={onToggle}
      aria-expanded={stepsExpanded}
    >
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
  aws_sso_expired: "SSO expired",
};

function SsoLoginBanner() {
  const { data: activeProfile } = useActiveProfileQuery();
  const ssoLogin = useAwsSsoLogin();
  const awsProfile = activeProfile?.name ?? "default";

  if (ssoLogin.isSuccess) {
    return (
      <div data-slot="turn-sso-success-banner">
        <CheckCircle2 className="h-5 w-5" />
        <div>
          <p data-slot="turn-sso-success-title">SSO authentication successful</p>
          <p data-slot="turn-sso-success-hint">You can now retry your message.</p>
        </div>
      </div>
    );
  }

  return (
    <div data-slot="turn-sso-banner">
      <p>AWS SSO session expired. Re-authenticate to continue.</p>
      <button
        type="button"
        data-slot="turn-sso-login-btn"
        disabled={ssoLogin.isPending}
        onClick={() => ssoLogin.mutate(awsProfile)}
      >
        {ssoLogin.isPending ? "Logging in..." : `aws sso login --profile ${awsProfile}`}
      </button>
      {ssoLogin.isError && (
        <p data-slot="turn-sso-error">
          SSO login failed. Run manually: aws sso login --profile {awsProfile}
        </p>
      )}
    </div>
  );
}

function TurnFinishInfo({
  turn,
  displayDuration,
}: {
  turn: ChatTurn;
  displayDuration: string | null;
}) {
  if (turn.status === "streaming" || turn.status === "pending") return null;

  const reasonLabel =
    turn.finishReason && turn.finishReason !== "end_turn"
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
            {turn.images && turn.images.length > 0 && (
              <div data-slot="user-message-images">
                {turn.images.map((img) => (
                  <img
                    key={img.id}
                    src={img.dataUrl}
                    alt={img.name}
                    data-slot="user-message-image"
                  />
                ))}
              </div>
            )}
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

        {turn.finishReason === "aws_sso_expired" && <SsoLoginBanner />}

        <TurnFinishInfo turn={turn} displayDuration={displayDuration} />
      </div>
    </div>
  );
}
