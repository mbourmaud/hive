import "./session-turn.css";

import {
  BookOpen,
  Brain,
  Check,
  ChevronDown,
  ChevronRight,
  Copy,
  FileCode,
  FileText,
  GitBranch,
  Globe,
  Loader2,
  Pencil,
  Search,
  Terminal,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { cn } from "@/shared/lib/utils";
import type {
  AssistantPart,
  ChatTurn,
  TextPart,
  ThinkingPart,
  ToolResultPart,
  ToolUsePart,
} from "../types";
import { DiffViewer } from "./diff-viewer";
import { MarkdownRenderer } from "./markdown-renderer";

// ── Constants ────────────────────────────────────────────────────────────────

const COLLAPSE_CHAR_THRESHOLD = 200;
const STATUS_DEBOUNCE_MS = 2500;

// ── Status mapping ───────────────────────────────────────────────────────────

function computeStatusLabel(toolName: string): string {
  switch (toolName.toLowerCase()) {
    case "read":
    case "readfile":
      return "gathering context";
    case "grep":
    case "glob":
    case "search":
      return "searching codebase";
    case "edit":
    case "write":
    case "writefile":
      return "making edits";
    case "bash":
    case "execute":
    case "run":
      return "running commands";
    case "task":
    case "sendmessage":
    case "delegate":
      return "delegating";
    default:
      return "thinking";
  }
}

// ── Duration formatting ──────────────────────────────────────────────────────

function formatDuration(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}m ${seconds}s`;
}

function formatToolDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const seconds = ms / 1000;
  if (seconds < 60) return `${seconds.toFixed(1)}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.floor(seconds % 60);
  return `${minutes}m ${remainingSeconds}s`;
}

// ── Hook: live elapsed time ──────────────────────────────────────────────────

function useElapsed(startedAt: number, isActive: boolean): number {
  const [elapsed, setElapsed] = useState(() => (isActive ? Date.now() - startedAt : 0));

  useEffect(() => {
    if (!isActive) return;

    const tick = () => setElapsed(Date.now() - startedAt);
    tick();
    const id = setInterval(tick, 1000);
    return () => clearInterval(id);
  }, [startedAt, isActive]);

  return elapsed;
}

// ── Hook: debounced status label ─────────────────────────────────────────────

function useDebouncedStatus(parts: AssistantPart[]): string {
  const [label, setLabel] = useState("thinking");
  const lastChangeRef = useRef(0);
  const pendingRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    let lastToolName = "";
    for (let i = parts.length - 1; i >= 0; i--) {
      const part = parts[i];
      if (part?.type === "tool_use" && part.status === "running") {
        lastToolName = part.name;
        break;
      }
    }

    const newLabel = lastToolName ? computeStatusLabel(lastToolName) : "thinking";
    const now = Date.now();
    const timeSinceLastChange = now - lastChangeRef.current;

    if (pendingRef.current) {
      clearTimeout(pendingRef.current);
      pendingRef.current = null;
    }

    if (timeSinceLastChange >= STATUS_DEBOUNCE_MS) {
      setLabel(newLabel);
      lastChangeRef.current = now;
    } else {
      const delay = STATUS_DEBOUNCE_MS - timeSinceLastChange;
      pendingRef.current = setTimeout(() => {
        setLabel(newLabel);
        lastChangeRef.current = Date.now();
        pendingRef.current = null;
      }, delay);
    }

    return () => {
      if (pendingRef.current) {
        clearTimeout(pendingRef.current);
        pendingRef.current = null;
      }
    };
  }, [parts]);

  return label;
}

// ── Hook: sticky height tracking ─────────────────────────────────────────────

function useStickyHeight(): [React.RefObject<HTMLDivElement | null>, number] {
  const ref = useRef<HTMLDivElement>(null);
  const [height, setHeight] = useState(0);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setHeight(entry.contentRect.height);
      }
    });

    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return [ref, height];
}

// ── Tool icon mapping ────────────────────────────────────────────────────────

function ToolIcon({ name, status }: { name: string; status: ToolUsePart["status"] }) {
  const iconClass = cn(
    "h-4 w-4 shrink-0",
    status === "running" && "text-accent",
    status === "error" && "text-destructive",
    status === "completed" && "text-muted-foreground",
    status === "pending" && "text-muted-foreground opacity-40",
  );

  if (status === "running") {
    return <Loader2 className={cn(iconClass, "animate-spin")} />;
  }

  const lower = name.toLowerCase();

  if (lower === "read" || lower === "readfile" || lower === "view") {
    return <BookOpen className={iconClass} />;
  }
  if (lower === "bash" || lower === "execute" || lower === "run") {
    return <Terminal className={iconClass} />;
  }
  if (lower === "edit") {
    return <Pencil className={iconClass} />;
  }
  if (lower === "write" || lower === "writefile") {
    return <FileCode className={iconClass} />;
  }
  if (lower === "glob" || lower === "grep" || lower === "search") {
    return <Search className={iconClass} />;
  }
  if (lower === "task" || lower === "sendmessage" || lower === "delegate") {
    return <GitBranch className={iconClass} />;
  }
  if (lower === "webfetch" || lower === "websearch") {
    return <Globe className={iconClass} />;
  }

  return <FileText className={iconClass} />;
}

// ── Tool title + subtitle ────────────────────────────────────────────────────

function toolDisplayName(name: string): string {
  const lower = name.toLowerCase();
  switch (lower) {
    case "read":
    case "readfile":
      return "Read";
    case "bash":
    case "execute":
    case "run":
      return "Shell";
    case "edit":
      return "Edit";
    case "write":
    case "writefile":
      return "Write";
    case "glob":
      return "Glob";
    case "grep":
      return "Grep";
    case "search":
      return "Search";
    case "task":
      return "Task";
    case "sendmessage":
      return "Message";
    case "delegate":
      return "Delegate";
    case "webfetch":
      return "Fetch";
    case "websearch":
      return "Search";
    default:
      return name;
  }
}

function toolSubtitle(name: string, input: Record<string, unknown>): string {
  const lower = name.toLowerCase();

  if (["read", "readfile", "edit", "write", "writefile"].includes(lower)) {
    const filePath = input.file_path ?? input.path ?? "";
    if (typeof filePath === "string" && filePath) {
      const segments = filePath.split("/");
      return segments.slice(-2).join("/");
    }
  }

  if (["grep", "glob", "search"].includes(lower)) {
    const pattern = input.pattern ?? input.query ?? "";
    if (typeof pattern === "string" && pattern) {
      return pattern.length > 40 ? `${pattern.slice(0, 40)}\u2026` : pattern;
    }
  }

  if (lower === "bash" || lower === "execute" || lower === "run") {
    const cmd = input.command ?? "";
    if (typeof cmd === "string" && cmd) {
      return cmd.length > 50 ? `${cmd.slice(0, 50)}\u2026` : cmd;
    }
  }

  if (lower === "task" || lower === "sendmessage" || lower === "delegate") {
    const desc = input.description ?? input.subject ?? input.prompt ?? "";
    if (typeof desc === "string" && desc) {
      return desc.length > 60 ? `${desc.slice(0, 60)}\u2026` : desc;
    }
  }

  if (lower === "webfetch" || lower === "websearch") {
    const url = input.url ?? input.query ?? "";
    if (typeof url === "string" && url) {
      return url.length > 50 ? `${url.slice(0, 50)}\u2026` : url;
    }
  }

  return "";
}

// ── File extension → language mapping ───────────────────────────────────────

const EXT_LANG_MAP: Record<string, string> = {
  ts: "typescript",
  tsx: "tsx",
  js: "javascript",
  jsx: "jsx",
  rs: "rust",
  py: "python",
  rb: "ruby",
  go: "go",
  java: "java",
  kt: "kotlin",
  swift: "swift",
  c: "c",
  cpp: "cpp",
  h: "c",
  hpp: "cpp",
  cs: "csharp",
  css: "css",
  scss: "scss",
  html: "html",
  json: "json",
  yaml: "yaml",
  yml: "yaml",
  toml: "toml",
  md: "markdown",
  sh: "bash",
  bash: "bash",
  zsh: "bash",
  sql: "sql",
  xml: "xml",
  vue: "vue",
  svelte: "svelte",
  lua: "lua",
  zig: "zig",
};

function langFromPath(filePath: string): string {
  const ext = filePath.split(".").pop()?.toLowerCase() ?? "";
  return EXT_LANG_MAP[ext] ?? "";
}

// ── Tool-specific expanded body renderers ───────────────────────────────────

function BashToolBody({
  input,
  result,
}: {
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  const command = typeof input.command === "string" ? input.command : "";
  const content = result?.content ?? "";
  const truncated = content.length > 2000 ? `${content.slice(0, 2000)}\n... (truncated)` : content;

  return (
    <>
      {command && (
        <div data-slot="tool-body-command">
          <span data-slot="tool-body-command-prefix">$</span>
          <code>{command}</code>
        </div>
      )}
      {result && (
        <div data-slot="tool-body-result" data-error={result.isError || undefined}>
          <MarkdownRenderer text={`\`\`\`\n${truncated}\n\`\`\``} />
        </div>
      )}
    </>
  );
}

function ReadToolBody({
  input,
  result,
}: {
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  const filePath =
    typeof input.file_path === "string"
      ? input.file_path
      : typeof input.path === "string"
        ? input.path
        : "";
  const lang = langFromPath(filePath);
  const content = result?.content ?? "";
  const truncated = content.length > 2000 ? `${content.slice(0, 2000)}\n... (truncated)` : content;

  return (
    <>
      {filePath && (
        <div data-slot="tool-body-filepath">
          <code>{filePath}</code>
        </div>
      )}
      {result && (
        <div data-slot="tool-body-result" data-error={result.isError || undefined}>
          <MarkdownRenderer text={`\`\`\`${lang}\n${truncated}\n\`\`\``} />
        </div>
      )}
    </>
  );
}

function EditToolBody({
  input,
  result,
}: {
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  const filePath =
    typeof input.file_path === "string"
      ? input.file_path
      : typeof input.path === "string"
        ? input.path
        : "";
  const oldString = typeof input.old_string === "string" ? input.old_string : undefined;
  const newString = typeof input.new_string === "string" ? input.new_string : undefined;

  return (
    <>
      {filePath && (
        <div data-slot="tool-body-filepath">
          <code>{filePath}</code>
        </div>
      )}
      {oldString !== undefined && newString !== undefined ? (
        <DiffViewer oldText={oldString} newText={newString} filePath={filePath || undefined} />
      ) : (
        result && (
          <div data-slot="tool-body-result" data-error={result.isError || undefined}>
            <pre>
              <code>
                {result.content.length > 2000
                  ? `${result.content.slice(0, 2000)}\n... (truncated)`
                  : result.content}
              </code>
            </pre>
          </div>
        )
      )}
    </>
  );
}

function FileListToolBody({
  input,
  result,
}: {
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  const pattern =
    typeof input.pattern === "string"
      ? input.pattern
      : typeof input.query === "string"
        ? input.query
        : "";
  const content = result?.content ?? "";
  const lines = content.split("\n").filter((l) => l.trim().length > 0);
  const displayLines = lines.slice(0, 50);
  const remaining = lines.length - displayLines.length;

  return (
    <>
      {pattern && (
        <div data-slot="tool-body-filepath">
          <code>{pattern}</code>
        </div>
      )}
      {result && (
        <div data-slot="tool-body-result" data-error={result.isError || undefined}>
          <div data-slot="tool-body-filelist">
            {displayLines.map((line) => (
              <div key={line} data-slot="tool-body-filelist-item">
                {line}
              </div>
            ))}
            {remaining > 0 && (
              <div data-slot="tool-body-filelist-more">... and {remaining} more</div>
            )}
          </div>
        </div>
      )}
    </>
  );
}

function TaskToolBody({
  input,
  result,
}: {
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  const subject =
    typeof input.subject === "string"
      ? input.subject
      : typeof input.description === "string"
        ? input.description
        : "";
  const content = result?.content ?? "";
  const truncated = content.length > 500 ? `${content.slice(0, 500)}...` : content;

  return (
    <>
      {subject && (
        <div data-slot="tool-body-task-label">
          <span data-slot="tool-body-task-tree">&#x2514;</span>
          <span>{subject.length > 100 ? `${subject.slice(0, 100)}...` : subject}</span>
        </div>
      )}
      {result && truncated && (
        <div data-slot="tool-body-result" data-error={result.isError || undefined}>
          <pre>
            <code>{truncated}</code>
          </pre>
        </div>
      )}
    </>
  );
}

function DefaultToolBody({
  input,
  result,
}: {
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  return (
    <>
      <pre>
        <code>{JSON.stringify(input, null, 2)}</code>
      </pre>
      {result && (
        <div data-slot="tool-body-result" data-error={result.isError || undefined}>
          <pre>
            <code>
              {result.content.length > 2000
                ? `${result.content.slice(0, 2000)}\n... (truncated)`
                : result.content}
            </code>
          </pre>
        </div>
      )}
    </>
  );
}

function ToolExpandedBody({
  name,
  input,
  result,
}: {
  name: string;
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  const lower = name.toLowerCase();

  if (lower === "bash" || lower === "execute" || lower === "run") {
    return <BashToolBody input={input} result={result} />;
  }

  if (lower === "read" || lower === "readfile" || lower === "view") {
    return <ReadToolBody input={input} result={result} />;
  }

  if (lower === "edit" || lower === "write" || lower === "writefile") {
    return <EditToolBody input={input} result={result} />;
  }

  if (lower === "glob" || lower === "grep" || lower === "search") {
    return <FileListToolBody input={input} result={result} />;
  }

  if (lower === "task" || lower === "sendmessage" || lower === "delegate") {
    return <TaskToolBody input={input} result={result} />;
  }

  return <DefaultToolBody input={input} result={result} />;
}

// ── Copy button (reusable) ───────────────────────────────────────────────────

function CopyButton({ text, slot }: { text: string; slot: string }) {
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

function PartRenderer({ part, result }: PartRendererProps) {
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

  // ── Derived data ─────────────────────────────────────────────────────────

  const canExpandUser = turn.userMessage.length > COLLAPSE_CHAR_THRESHOLD;

  const toolUseParts = useMemo(
    () => turn.assistantParts.filter((p): p is ToolUsePart => p.type === "tool_use"),
    [turn.assistantParts],
  );

  const toolResultMap = useMemo(() => {
    const map = new Map<string, ToolResultPart>();
    for (const p of turn.assistantParts) {
      if (p.type === "tool_result") {
        map.set(p.toolUseId, p);
      }
    }
    return map;
  }, [turn.assistantParts]);

  const thinkingParts = useMemo(
    () => turn.assistantParts.filter((p): p is ThinkingPart => p.type === "thinking"),
    [turn.assistantParts],
  );

  const stepsCount = toolUseParts.length + thinkingParts.length;

  // Summary = last text part
  const summaryText = useMemo(() => {
    const lastTextIdx = findLastTextIndex(turn.assistantParts);
    if (lastTextIdx === -1) return null;
    const lastPart = turn.assistantParts[lastTextIdx];
    if (!lastPart || lastPart.type !== "text") return null;
    return lastPart.text;
  }, [turn.assistantParts]);

  let displayDuration: string | null = null;
  if (turn.duration !== null) {
    displayDuration = formatDuration(turn.duration);
  } else if (isStreaming) {
    displayDuration = formatDuration(elapsed);
  }

  // ── Error text ───────────────────────────────────────────────────────────

  const errorText = useMemo(() => {
    const errorResults = turn.assistantParts.filter(
      (p): p is ToolResultPart => p.type === "tool_result" && p.isError,
    );
    if (errorResults.length > 0) {
      return errorResults.map((r) => r.content).join("\n\n");
    }
    return turn.status === "error" ? "An error occurred during this turn." : null;
  }, [turn.assistantParts, turn.status]);

  // ── Steps content ──────────────────────────────────────────────────────

  const stepsParts = useMemo(() => {
    const lastTextIdx = findLastTextIndex(turn.assistantParts);
    const result: AssistantPart[] = [];
    for (let i = 0; i < turn.assistantParts.length; i++) {
      const part = turn.assistantParts[i];
      if (!part) continue;
      if (part.type === "tool_use" || part.type === "thinking") {
        result.push(part);
      } else if (part.type === "text" && i !== lastTextIdx) {
        result.push(part);
      }
    }
    return result;
  }, [turn.assistantParts]);

  // ── Render ───────────────────────────────────────────────────────────────

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
        {/* ── Sticky header: user message + steps toggle ─────────────── */}
        <div data-slot="turn-sticky" ref={stickyRef}>
          {/* User message card */}
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

          {/* Steps trigger */}
          {stepsCount > 0 && (
            <button
              type="button"
              data-slot="steps-trigger"
              onClick={onToggleSteps}
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
                <span data-slot="steps-trigger-count">
                  {isStreaming
                    ? `${stepsCount} ${stepsCount === 1 ? "step" : "steps"}`
                    : stepsExpanded
                      ? "hide steps"
                      : "show steps"}
                </span>
                {isStreaming && <span data-slot="steps-trigger-status">{statusLabel}</span>}
              </div>
              {displayDuration && <span data-slot="steps-trigger-duration">{displayDuration}</span>}
            </button>
          )}
        </div>

        {/* ── Steps (collapsible tool call list) ────────────────────── */}
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

        {/* ── Summary (final assistant text) ────────────────────────── */}
        {summaryText && !isStreaming && (
          <div data-slot="turn-summary-section">
            <div data-slot="turn-summary-header">
              <div data-slot="turn-summary-title-row">
                <span data-slot="turn-summary-title">Response</span>
                <CopyButton text={summaryText} slot="turn-summary-copy" />
              </div>
            </div>
            <div data-slot="turn-summary" data-fade={isLast || undefined}>
              <MarkdownRenderer text={summaryText} />
            </div>
          </div>
        )}

        {/* ── Streaming summary (live) ──────────────────────────────── */}
        {summaryText && isStreaming && (
          <div data-slot="turn-summary" data-streaming="true">
            <MarkdownRenderer text={summaryText} />
          </div>
        )}

        {/* ── Error ─────────────────────────────────────────────────── */}
        {errorText && (
          <div data-slot="turn-error">
            <pre>{errorText}</pre>
          </div>
        )}

        {/* ── Finish info: model + duration + finish reason badge ────── */}
        {!isStreaming && turn.status !== "pending" && (
          <div data-slot="turn-finish-info">
            {turn.model && <span data-slot="turn-finish-model">{turn.model}</span>}
            {turn.model && displayDuration && <span data-slot="turn-finish-sep">&middot;</span>}
            {displayDuration && <span data-slot="turn-finish-duration">{displayDuration}</span>}
            {turn.finishReason && turn.finishReason !== "end_turn" && (
              <span data-slot="turn-finish-badge" data-reason={turn.finishReason}>
                {turn.finishReason === "error"
                  ? "error"
                  : turn.finishReason === "canceled"
                    ? "canceled"
                    : "max tokens"}
              </span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function findLastTextIndex(parts: AssistantPart[]): number {
  for (let i = parts.length - 1; i >= 0; i--) {
    if (parts[i]?.type === "text") return i;
  }
  return -1;
}
