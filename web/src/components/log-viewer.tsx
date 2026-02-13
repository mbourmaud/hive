import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/utils";
import { Copy, Check, Code, FileText } from "lucide-react";

interface LogViewerProps {
  logs: string[];
  raw: boolean;
  onToggleRaw: () => void;
}

function lineColorClass(line: string): string {
  const lower = line.toLowerCase();
  if (lower.startsWith("[done]") || lower.includes("success") || lower.includes("completed")) return "text-success";
  if (lower.startsWith("[tool]")) return "text-accent";
  if (lower.startsWith("[init]")) return "text-muted-foreground/60";
  if (lower.includes("error") || lower.includes("fatal") || lower.includes("panic")) return "text-destructive";
  if (lower.includes("warn")) return "text-warning";
  if (lower.includes("debug") || lower.includes("trace")) return "text-muted-foreground/60";
  return "";
}

export function LogViewer({ logs, raw, onToggleRaw }: LogViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs]);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(logs.join("\n"));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const gutterWidth = logs.length > 0 ? String(logs.length).length : 1;

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      {/* Toolbar */}
      <div className="flex items-center justify-end gap-1 px-2 py-1.5 bg-surface-inset border-b border-border">
        <button
          onClick={onToggleRaw}
          className={cn(
            "flex items-center gap-1.5 px-2 py-1 rounded text-[11px] font-medium transition-colors",
            "hover:bg-muted/50 text-muted-foreground hover:text-foreground"
          )}
          title={raw ? "Switch to formatted view" : "Switch to raw JSON"}
        >
          {raw ? <FileText className="w-3.5 h-3.5" /> : <Code className="w-3.5 h-3.5" />}
          {raw ? "Pretty" : "Raw"}
        </button>
        <button
          onClick={handleCopy}
          className={cn(
            "flex items-center gap-1.5 px-2 py-1 rounded text-[11px] font-medium transition-colors",
            copied
              ? "text-success"
              : "hover:bg-muted/50 text-muted-foreground hover:text-foreground"
          )}
          title="Copy logs to clipboard"
        >
          {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
          {copied ? "Copied!" : "Copy"}
        </button>
      </div>

      {/* Log content */}
      <div
        ref={containerRef}
        className="max-h-[400px] overflow-y-auto font-mono text-xs leading-relaxed bg-surface-inset"
      >
        {logs.length === 0 ? (
          <div className="p-3 text-muted-foreground animate-pulse">Waiting for logs...</div>
        ) : (
          logs.map((line, i) => (
            <div
              key={i}
              className={cn(
                "flex hover:bg-muted/30 transition-colors",
                lineColorClass(line)
              )}
            >
              <span
                className="shrink-0 text-right pr-3 pl-2 py-px text-muted-foreground/40 select-none border-r border-border/50"
                style={{ minWidth: `${gutterWidth + 2}ch` }}
              >
                {i + 1}
              </span>
              <span className={cn("flex-1 px-3 py-px whitespace-pre-wrap break-all", !lineColorClass(line) && "text-muted-foreground")}>
                {line}
              </span>
            </div>
          ))
        )}
      </div>
    </div>
  );
}
