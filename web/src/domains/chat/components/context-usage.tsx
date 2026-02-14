import { useState } from "react";
import type { ContextUsage } from "../types";
import "./context-usage.css";

// ── Constants ────────────────────────────────────────────────────────────────

// Claude's context window (200k tokens total input capacity)
const MAX_CONTEXT_TOKENS = 200_000;

// ── Types ────────────────────────────────────────────────────────────────────

interface ContextUsageProps {
  usage: ContextUsage | null;
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function formatTokens(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}k`;
  return String(count);
}

function formatCost(usd: number): string {
  if (usd < 0.01) return `$${usd.toFixed(4)}`;
  return `$${usd.toFixed(2)}`;
}

function getUsageLevel(ratio: number): "low" | "medium" | "high" {
  if (ratio >= 0.8) return "high";
  if (ratio >= 0.5) return "medium";
  return "low";
}

// ── Component ────────────────────────────────────────────────────────────────

export function ContextUsageIndicator({ usage }: ContextUsageProps) {
  const [expanded, setExpanded] = useState(false);

  if (!usage) return null;

  // Context window usage = input tokens only (output tokens don't count against the 200k limit)
  // Cache read tokens are already included in inputTokens, so we don't double-count
  const totalUsed = usage.inputTokens;
  const ratio = Math.min(totalUsed / MAX_CONTEXT_TOKENS, 1);
  const level = getUsageLevel(ratio);
  const percent = Math.round(ratio * 100);

  return (
    <div data-component="context-usage" data-level={level}>
      <button
        type="button"
        data-slot="context-usage-toggle"
        onClick={() => setExpanded((prev) => !prev)}
        aria-expanded={expanded}
        aria-label="Toggle context usage breakdown"
      >
        <div data-slot="context-usage-bar">
          <div data-slot="context-usage-fill" style={{ transform: `scaleX(${ratio})` }} />
        </div>
        <span data-slot="context-usage-label">
          {formatTokens(totalUsed)} / {formatTokens(MAX_CONTEXT_TOKENS)} ({percent}%)
        </span>
      </button>

      {expanded && (
        <div data-slot="context-usage-breakdown">
          <div data-slot="context-usage-row">
            <span>Input tokens</span>
            <span>{formatTokens(usage.inputTokens)}</span>
          </div>
          <div data-slot="context-usage-row">
            <span>Output tokens</span>
            <span>{formatTokens(usage.outputTokens)}</span>
          </div>
          {usage.cacheReadTokens != null && (
            <div data-slot="context-usage-row">
              <span>Cache read</span>
              <span>{formatTokens(usage.cacheReadTokens)}</span>
            </div>
          )}
          {usage.cacheWriteTokens != null && (
            <div data-slot="context-usage-row">
              <span>Cache write</span>
              <span>{formatTokens(usage.cacheWriteTokens)}</span>
            </div>
          )}
          {usage.totalCost != null && (
            <div data-slot="context-usage-row" data-highlight>
              <span>Estimated cost</span>
              <span>{formatCost(usage.totalCost)}</span>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
