import type { ChatTurn, ContextUsage } from "../../types";

// ── Constants ────────────────────────────────────────────────────────────────

const MAX_CONTEXT_TOKENS = 200_000;

// ── Types ────────────────────────────────────────────────────────────────────

export interface StatItem {
  label: string;
  value: string;
}

export interface BreakdownSegment {
  label: string;
  tokens: number;
  percent: number;
  color: string;
}

// ── Helpers ──────────────────────────────────────────────────────────────────

export function formatTokens(count: number): string {
  if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`;
  if (count >= 1_000) return `${(count / 1_000).toFixed(1)}k`;
  return String(count);
}

export function formatCost(usd: number): string {
  if (usd < 0.01) return `$${usd.toFixed(4)}`;
  return `$${usd.toFixed(2)}`;
}

function formatDate(timestamp: number | string): string {
  const date = typeof timestamp === "string" ? new Date(timestamp) : new Date(timestamp);
  return date.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

// ── Stat computation ─────────────────────────────────────────────────────────

export function computeSessionStats(
  turns: ChatTurn[],
  usage: ContextUsage | null,
  model: string | undefined,
  sessionCreatedAt: string | undefined,
): StatItem[] {
  const userCount = turns.filter((t) => !t.droneName).length;
  const assistantCount = turns.filter((t) => !t.droneName && t.assistantParts.length > 0).length;
  const lastTurn = turns[turns.length - 1];
  const lastActivity = lastTurn ? lastTurn.startedAt + (lastTurn.duration ?? 0) : undefined;

  const stats: StatItem[] = [
    { label: "Model", value: model ?? "Unknown" },
    { label: "Messages", value: `${userCount} user / ${assistantCount} assistant` },
  ];

  if (usage) {
    const total = usage.inputTokens + usage.outputTokens;
    const ratio = Math.min(usage.inputTokens / MAX_CONTEXT_TOKENS, 1);
    stats.push(
      { label: "Total Tokens", value: formatTokens(total) },
      { label: "Usage", value: `${Math.round(ratio * 100)}%` },
      { label: "Input Tokens", value: formatTokens(usage.inputTokens) },
      { label: "Output Tokens", value: formatTokens(usage.outputTokens) },
    );
    if (usage.cacheReadTokens != null) {
      stats.push({ label: "Cache Read", value: formatTokens(usage.cacheReadTokens) });
    }
    if (usage.cacheWriteTokens != null) {
      stats.push({ label: "Cache Write", value: formatTokens(usage.cacheWriteTokens) });
    }
    if (usage.totalCost != null) {
      stats.push({ label: "Cost", value: formatCost(usage.totalCost) });
    }
  }

  if (sessionCreatedAt) {
    stats.push({ label: "Created", value: formatDate(sessionCreatedAt) });
  }
  if (lastActivity) {
    stats.push({ label: "Last Activity", value: formatDate(lastActivity) });
  }

  return stats;
}

// ── Breakdown bar ────────────────────────────────────────────────────────────

export function computeBreakdownSegments(usage: ContextUsage | null): BreakdownSegment[] {
  if (!usage) return [];

  const total = usage.inputTokens + usage.outputTokens;
  if (total === 0) return [];

  const segments: BreakdownSegment[] = [];

  if (usage.cacheReadTokens) {
    segments.push({
      label: "Cache Read",
      tokens: usage.cacheReadTokens,
      percent: (usage.cacheReadTokens / total) * 100,
      color: "var(--color-accent)",
    });
  }

  if (usage.cacheWriteTokens) {
    segments.push({
      label: "Cache Write",
      tokens: usage.cacheWriteTokens,
      percent: (usage.cacheWriteTokens / total) * 100,
      color: "var(--color-warning)",
    });
  }

  const nonCacheInput =
    usage.inputTokens - (usage.cacheReadTokens ?? 0) - (usage.cacheWriteTokens ?? 0);
  if (nonCacheInput > 0) {
    segments.push({
      label: "Input",
      tokens: nonCacheInput,
      percent: (nonCacheInput / total) * 100,
      color: "var(--color-success)",
    });
  }

  segments.push({
    label: "Output",
    tokens: usage.outputTokens,
    percent: (usage.outputTokens / total) * 100,
    color: "var(--color-info, var(--color-ring))",
  });

  return segments;
}
