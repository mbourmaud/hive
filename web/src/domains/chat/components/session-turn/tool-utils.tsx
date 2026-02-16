import {
  BookOpen,
  FileCode,
  FileText,
  GitBranch,
  Globe,
  Loader2,
  Pencil,
  Search,
  Terminal,
} from "lucide-react";
import { cn } from "@/shared/lib/utils";
import type { ToolUsePart } from "../../types";

// ── Duration formatting ──────────────────────────────────────────────────────

export function formatDuration(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  if (totalSeconds < 60) return `${totalSeconds}s`;
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}m ${seconds}s`;
}

export function formatToolDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  const seconds = ms / 1000;
  if (seconds < 60) return `${seconds.toFixed(1)}s`;
  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.floor(seconds % 60);
  return `${minutes}m ${remainingSeconds}s`;
}

// ── Tool icon mapping ────────────────────────────────────────────────────────

export function ToolIcon({ name, status }: { name: string; status: ToolUsePart["status"] }) {
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

export function toolDisplayName(name: string): string {
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

// Map raw tool names to registry keys (must match registerTool() calls in parts/)
export function registryKeyForTool(name: string): string {
  const lower = name.toLowerCase();
  switch (lower) {
    case "read":
    case "readfile":
      return "Read";
    case "bash":
    case "execute":
    case "run":
      return "Bash";
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
      return "SendMessage";
    case "delegate":
      return "Delegate";
    case "webfetch":
      return "WebFetch";
    case "websearch":
      return "WebSearch";
    case "todowrite":
      return "TodoWrite";
    default:
      return name;
  }
}

interface SubtitleRule {
  tools: ReadonlySet<string>;
  keys: readonly string[];
  maxLen: number;
  format?: "filepath";
}

const SUBTITLE_RULES: SubtitleRule[] = [
  {
    tools: new Set(["read", "readfile", "edit", "write", "writefile"]),
    keys: ["file_path", "path"],
    maxLen: 0,
    format: "filepath",
  },
  { tools: new Set(["grep", "glob", "search"]), keys: ["pattern", "query"], maxLen: 40 },
  { tools: new Set(["bash", "execute", "run"]), keys: ["command"], maxLen: 50 },
  {
    tools: new Set(["task", "sendmessage", "delegate"]),
    keys: ["description", "subject", "prompt"],
    maxLen: 60,
  },
  { tools: new Set(["webfetch", "websearch"]), keys: ["url", "query"], maxLen: 50 },
];

function truncate(text: string, maxLen: number): string {
  if (maxLen <= 0 || text.length <= maxLen) return text;
  return `${text.slice(0, maxLen)}\u2026`;
}

export function toolSubtitle(name: string, input: Record<string, unknown>): string {
  const lower = name.toLowerCase();
  for (const rule of SUBTITLE_RULES) {
    if (!rule.tools.has(lower)) continue;
    for (const key of rule.keys) {
      const val = input[key];
      if (typeof val !== "string" || !val) continue;
      if (rule.format === "filepath") return val.split("/").slice(-2).join("/");
      return truncate(val, rule.maxLen);
    }
  }
  return "";
}
