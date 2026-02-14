import { useMemo } from "react";
import type {
  AssistantPart,
  ChatTurn,
  ThinkingPart,
  ToolResultPart,
  ToolUsePart,
} from "../types";

// ── Types ────────────────────────────────────────────────────────────────────

export interface TurnData {
  toolUseParts: ToolUsePart[];
  toolResultMap: Map<string, ToolResultPart>;
  thinkingParts: ThinkingPart[];
  stepsParts: AssistantPart[];
  stepsCount: number;
  summaryText: string | null;
  errorText: string | null;
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function findLastTextIndex(parts: AssistantPart[]): number {
  for (let i = parts.length - 1; i >= 0; i--) {
    if (parts[i]?.type === "text") return i;
  }
  return -1;
}

/** Single-pass: classify each part into the correct bucket. */
function classifyParts(parts: AssistantPart[], lastTextIdx: number) {
  const toolUseParts: ToolUsePart[] = [];
  const toolResultMap = new Map<string, ToolResultPart>();
  const thinkingParts: ThinkingPart[] = [];
  const stepsParts: AssistantPart[] = [];

  for (let i = 0; i < parts.length; i++) {
    const part = parts[i];
    if (!part) continue;

    switch (part.type) {
      case "tool_use":
        toolUseParts.push(part);
        stepsParts.push(part);
        break;
      case "tool_result":
        toolResultMap.set(part.toolUseId, part);
        break;
      case "thinking":
        thinkingParts.push(part);
        stepsParts.push(part);
        break;
      case "text":
        if (i !== lastTextIdx) stepsParts.push(part);
        break;
    }
  }

  return { toolUseParts, toolResultMap, thinkingParts, stepsParts };
}

/** Derive the turn-level error text (if any). */
function deriveTurnError(parts: AssistantPart[], turnStatus: string): string | null {
  if (turnStatus !== "error") return null;
  if (parts.some((p) => p.type === "tool_result" && p.isError)) return null;
  const lastText = [...parts].reverse().find((p) => p.type === "text");
  if (lastText && lastText.type === "text" && lastText.text.trim()) return null;
  return "An error occurred during this turn.";
}

// ── Hook ─────────────────────────────────────────────────────────────────────

/**
 * Consolidates derived turn data (previously 5 separate useMemo blocks).
 */
export function useTurnData(turn: ChatTurn): TurnData {
  return useMemo(() => {
    const parts = turn.assistantParts;
    const lastTextIdx = findLastTextIndex(parts);
    const { toolUseParts, toolResultMap, thinkingParts, stepsParts } = classifyParts(
      parts,
      lastTextIdx,
    );
    const stepsCount = toolUseParts.length + thinkingParts.length;

    let summaryText: string | null = null;
    if (lastTextIdx !== -1) {
      const lastPart = parts[lastTextIdx];
      if (lastPart?.type === "text") summaryText = lastPart.text;
    }

    const errorText = deriveTurnError(parts, turn.status);

    return { toolUseParts, toolResultMap, thinkingParts, stepsParts, stepsCount, summaryText, errorText };
  }, [turn.assistantParts, turn.status]);
}
