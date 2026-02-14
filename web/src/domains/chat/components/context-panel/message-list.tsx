import { ChevronRight } from "lucide-react";
import { useCallback, useMemo, useState } from "react";
import type { AssistantPart, ChatTurn } from "../../types";
import { CodeBlock } from "../code-block";
import "./message-list.css";

// ── Types ────────────────────────────────────────────────────────────────────

interface RawMessage {
  id: string;
  role: "user" | "assistant" | "tool";
  summary: string;
  json: unknown;
}

interface MessageListProps {
  turns: ChatTurn[];
  allExpanded: boolean;
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function summarizeUserMessage(text: string): string {
  if (text.length <= 80) return text;
  return `${text.slice(0, 77)}...`;
}

function summarizeAssistantPart(part: AssistantPart): string {
  switch (part.type) {
    case "text":
      return part.text.length <= 60 ? part.text : `${part.text.slice(0, 57)}...`;
    case "thinking":
      return `[thinking] ${part.topic ?? part.text.slice(0, 40)}`;
    case "tool_use":
      return `[tool] ${part.name}`;
    case "tool_result":
      return `[result] ${part.isError ? "error" : "ok"} — ${part.content.slice(0, 40)}`;
  }
}

function buildUserJson(turn: ChatTurn): unknown {
  return {
    role: "user",
    content: [{ type: "text", text: turn.userMessage }],
  };
}

function buildAssistantPartJson(part: AssistantPart): unknown {
  switch (part.type) {
    case "text":
      return { type: "text", text: part.text };
    case "thinking":
      return { type: "thinking", thinking: part.text };
    case "tool_use":
      return { type: "tool_use", id: part.id, name: part.name, input: part.input };
    case "tool_result":
      return {
        type: "tool_result",
        tool_use_id: part.toolUseId,
        content: part.content,
        is_error: part.isError,
      };
  }
}

function deriveMessages(turns: ChatTurn[]): RawMessage[] {
  const messages: RawMessage[] = [];

  for (const turn of turns) {
    if (turn.droneName) continue;

    messages.push({
      id: `${turn.id}-user`,
      role: "user",
      summary: summarizeUserMessage(turn.userMessage),
      json: buildUserJson(turn),
    });

    for (const part of turn.assistantParts) {
      const role = part.type === "tool_result" ? "tool" : "assistant";
      messages.push({
        id: `${turn.id}-${part.id}`,
        role,
        summary: summarizeAssistantPart(part),
        json: buildAssistantPartJson(part),
      });
    }
  }

  return messages;
}

// ── Message row ──────────────────────────────────────────────────────────────

interface MessageRowProps {
  message: RawMessage;
  expanded: boolean;
  onToggle: () => void;
}

function MessageRow({ message, expanded, onToggle }: MessageRowProps) {
  const jsonStr = useMemo(() => JSON.stringify(message.json, null, 2), [message.json]);

  return (
    <div data-slot="message-row">
      <button type="button" data-slot="message-row-header" onClick={onToggle}>
        <ChevronRight
          className="h-3 w-3"
          data-slot="message-chevron"
          data-expanded={expanded ? "" : undefined}
        />
        <span data-slot="message-role-badge" data-role={message.role}>
          {message.role}
        </span>
        <span data-slot="message-summary">{message.summary}</span>
      </button>
      {expanded && (
        <div data-slot="message-json-wrapper">
          <CodeBlock code={jsonStr} language="json" lineNumbers={false} maxHeight={350} />
        </div>
      )}
    </div>
  );
}

// ── Message list ─────────────────────────────────────────────────────────────

export function MessageList({ turns, allExpanded }: MessageListProps) {
  const messages = useMemo(() => deriveMessages(turns), [turns]);
  const [expandedIds, setExpandedIds] = useState<Set<string>>(new Set());

  const toggleMessage = useCallback((id: string) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }, []);

  if (messages.length === 0) {
    return (
      <div className="px-4 py-8 text-center text-xs text-muted-foreground">No messages yet</div>
    );
  }

  return (
    <div data-slot="context-panel-messages">
      {messages.map((msg) => (
        <MessageRow
          key={msg.id}
          message={msg}
          expanded={allExpanded || expandedIds.has(msg.id)}
          onToggle={() => toggleMessage(msg.id)}
        />
      ))}
    </div>
  );
}
