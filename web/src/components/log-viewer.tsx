import { useEffect, useRef } from "react";
import { cn } from "@/lib/utils";

interface LogViewerProps {
  logs: string[];
}

function lineColorClass(line: string): string {
  const lower = line.toLowerCase();
  if (lower.includes("error") || lower.includes("fatal") || lower.includes("panic")) return "text-destructive";
  if (lower.includes("warn")) return "text-warning";
  if (lower.includes("debug") || lower.includes("trace")) return "text-muted-foreground/60";
  if (lower.includes("success") || lower.includes("completed") || lower.includes("done")) return "text-success";
  return "";
}

export function LogViewer({ logs }: LogViewerProps) {
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [logs]);

  const gutterWidth = logs.length > 0 ? String(logs.length).length : 1;

  return (
    <div
      ref={containerRef}
      className="max-h-[400px] overflow-y-auto rounded-lg font-mono text-xs leading-relaxed bg-surface-inset border border-border"
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
  );
}
