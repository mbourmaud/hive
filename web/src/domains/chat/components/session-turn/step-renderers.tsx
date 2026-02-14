import "./steps.css";
import "./thinking.css";

import { Brain, Check, ChevronRight, Copy } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { cn } from "@/shared/lib/utils";
import type { AssistantPart, TextPart, ThinkingPart, ToolResultPart, ToolUsePart } from "../../types";
import { MarkdownRenderer } from "../markdown-renderer";
import { getToolComponent } from "../tool-registry";
import { ToolExpandedBody } from "./tool-bodies";
import { ToolIcon, formatToolDuration, registryKeyForTool, toolDisplayName, toolSubtitle } from "./tool-utils";

// ── Copy button (reusable) ───────────────────────────────────────────────────

export function CopyButton({ text, slot }: { text: string; slot: string }) {
  const [copied, setCopied] = useState(false);
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(text).then(() => {
      setCopied(true);
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
      timeoutRef.current = setTimeout(() => setCopied(false), 2000);
    });
  }, [text]);

  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);

  return (
    <button type="button" data-slot={slot} onClick={handleCopy} aria-label="Copy text">
      {copied ? <Check className="h-3 w-3 text-success" /> : <Copy className="h-3 w-3" />}
    </button>
  );
}

// ── Thinking part ────────────────────────────────────────────────────────────

function ThinkingPartRenderer({ part }: { part: ThinkingPart }) {
  const [expanded, setExpanded] = useState(false);
  const topicLabel = part.topic ?? "reasoning";

  return (
    <div data-slot="step-thinking">
      <button
        type="button"
        data-slot="step-thinking-header"
        onClick={() => setExpanded(!expanded)}
        aria-expanded={expanded}
      >
        <div data-slot="step-thinking-header-left">
          <Brain className="h-4 w-4 shrink-0 text-muted-foreground" />
          <span data-slot="step-thinking-title">Thinking</span>
          <span data-slot="step-thinking-topic">{topicLabel}</span>
        </div>
        <ChevronRight
          className={cn(
            "h-3.5 w-3.5 text-muted-foreground shrink-0 transition-transform duration-150",
            expanded && "rotate-90",
          )}
        />
      </button>

      {expanded && (
        <div data-slot="step-thinking-body">
          <MarkdownRenderer text={part.text} />
        </div>
      )}
    </div>
  );
}

// ── Tool collapsible part ────────────────────────────────────────────────────

function ToolPartDisplay({
  part,
  result,
}: {
  part: ToolUsePart;
  result: ToolResultPart | undefined;
}) {
  // Check registry for a dedicated renderer using the canonical tool name
  const RegisteredTool = getToolComponent(registryKeyForTool(part.name));

  // If a registered component exists, delegate rendering entirely to it
  if (RegisteredTool) {
    return (
      <div data-slot="step-tool" data-status={part.status}>
        <RegisteredTool input={part.input} output={result?.content} status={part.status} />
      </div>
    );
  }

  // Fallback: inline trigger + body for unregistered tools
  return <InlineToolPartDisplay part={part} result={result} />;
}

function InlineToolPartDisplay({
  part,
  result,
}: {
  part: ToolUsePart;
  result: ToolResultPart | undefined;
}) {
  const [expanded, setExpanded] = useState(false);
  const title = toolDisplayName(part.name);
  const subtitle = toolSubtitle(part.name, part.input);

  return (
    <div data-slot="step-tool" data-status={part.status}>
      <button
        type="button"
        data-slot="step-tool-trigger"
        onClick={() => setExpanded(!expanded)}
        aria-expanded={expanded}
      >
        <div data-slot="step-tool-trigger-left">
          <ToolIcon name={part.name} status={part.status} />
          <span data-slot="step-tool-title">{title}</span>
          {subtitle && <span data-slot="step-tool-subtitle">{subtitle}</span>}
          {part.duration != null && part.status !== "running" && (
            <span data-slot="step-tool-duration">{formatToolDuration(part.duration)}</span>
          )}
        </div>
        <ChevronRight
          className={cn(
            "h-3.5 w-3.5 text-muted-foreground shrink-0 transition-transform duration-150",
            expanded && "rotate-90",
          )}
        />
      </button>

      {expanded && (
        <div data-slot="step-tool-body">
          <ToolExpandedBody name={part.name} input={part.input} result={result} />
        </div>
      )}
    </div>
  );
}

// ── Text part in steps ───────────────────────────────────────────────────────

function StepTextPart({ part }: { part: TextPart }) {
  return (
    <div data-slot="step-text">
      <div data-slot="step-text-body">
        <CopyButton text={part.text} slot="step-text-copy" />
        <MarkdownRenderer text={part.text} />
      </div>
    </div>
  );
}

// ── Part renderer (dispatch) ─────────────────────────────────────────────────

interface PartRendererProps {
  part: AssistantPart;
  result: ToolResultPart | undefined;
}

export function PartRenderer({ part, result }: PartRendererProps) {
  if (part.type === "text") {
    return <StepTextPart part={part} />;
  }

  if (part.type === "thinking") {
    return <ThinkingPartRenderer part={part} />;
  }

  if (part.type === "tool_use") {
    return <ToolPartDisplay part={part} result={result} />;
  }

  return null;
}
