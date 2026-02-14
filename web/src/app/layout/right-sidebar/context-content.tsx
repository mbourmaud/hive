import { useMemo, useState } from "react";
import { MessageList } from "@/domains/chat/components/context-panel/message-list";
import {
  computeBreakdownSegments,
  computeSessionStats,
  formatTokens,
} from "@/domains/chat/components/context-panel/stats";
import type { ChatSession, ChatTurn, ContextUsage } from "@/domains/chat/types";
import "./context-content.css";

// ── Types ────────────────────────────────────────────────────────────────────

interface ContextContentProps {
  turns: ChatTurn[];
  contextUsage: ContextUsage | null;
  session: ChatSession | null;
  selectedModel?: string;
}

// ── Component ────────────────────────────────────────────────────────────────

export function ContextContent({
  turns,
  contextUsage,
  session,
  selectedModel,
}: ContextContentProps) {
  const [allExpanded, setAllExpanded] = useState(false);

  const stats = useMemo(
    () => computeSessionStats(turns, contextUsage, selectedModel, session?.createdAt),
    [turns, contextUsage, selectedModel, session?.createdAt],
  );

  const segments = useMemo(() => computeBreakdownSegments(contextUsage), [contextUsage]);

  return (
    <>
      {/* Stats grid */}
      <div data-slot="context-panel-stats">
        {stats.map((stat) => (
          <div key={stat.label} data-slot="stat-item">
            <span data-slot="stat-label">{stat.label}</span>
            <span data-slot="stat-value">{stat.value}</span>
          </div>
        ))}
      </div>

      {/* Context breakdown bar */}
      {segments.length > 0 && (
        <div data-slot="context-panel-breakdown">
          <div data-slot="breakdown-bar">
            {segments.map((seg) => (
              <div
                key={seg.label}
                data-slot="breakdown-segment"
                style={{
                  width: `${seg.percent}%`,
                  background: seg.color,
                }}
                title={`${seg.label}: ${formatTokens(seg.tokens)} (${seg.percent.toFixed(1)}%)`}
              />
            ))}
          </div>
          <div data-slot="breakdown-legend">
            {segments.map((seg) => (
              <span key={seg.label} data-slot="legend-item">
                <span data-slot="legend-dot" style={{ background: seg.color }} />
                {seg.label} ({formatTokens(seg.tokens)})
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Raw messages section */}
      <div data-slot="context-panel-messages-header">
        <span data-slot="context-panel-messages-title">Raw Messages</span>
        <button
          type="button"
          data-slot="expand-all-btn"
          onClick={() => setAllExpanded((prev) => !prev)}
        >
          {allExpanded ? "Collapse all" : "Expand all"}
        </button>
      </div>

      <MessageList turns={turns} allExpanded={allExpanded} />
    </>
  );
}
