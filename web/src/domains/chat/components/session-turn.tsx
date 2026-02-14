import { useState } from "react";
import { cn } from "@/lib/utils";
import { Brain, ChevronDown } from "lucide-react";

interface ThinkingBlockProps {
  content: string;
  durationMs?: number;
}

export function ThinkingBlock({ content, durationMs }: ThinkingBlockProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const duration = durationMs ? formatDuration(durationMs) : null;

  return (
    <div className="rounded-lg border border-border overflow-hidden bg-surface-inset/50">
      {/* Header - collapsible trigger */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className={cn(
          "w-full flex items-center gap-2 px-3 py-2 hover:bg-muted/30 transition-colors text-left",
          isExpanded && "border-b border-border"
        )}
      >
        <Brain className="w-4 h-4 text-muted-foreground shrink-0" />
        <span className="flex-1 text-xs text-muted-foreground">
          {isExpanded ? "Thinking" : "Thinking..."}
        </span>
        {duration && (
          <span className="text-[11px] font-medium text-muted-foreground/60 shrink-0">
            {duration}
          </span>
        )}
        <ChevronDown
          className={cn(
            "w-4 h-4 text-muted-foreground transition-transform",
            isExpanded && "rotate-180"
          )}
        />
      </button>

      {/* Expanded content */}
      {isExpanded && (
        <div className="px-3 py-2 max-h-[400px] overflow-y-auto">
          <pre className="text-xs leading-relaxed whitespace-pre-wrap text-foreground/80 font-mono">
            {content}
          </pre>
        </div>
      )}
    </div>
  );
}

function formatDuration(ms: number): string {
  if (ms < 1000) {
    return `${ms}ms`;
  }
  const seconds = (ms / 1000).toFixed(1);
  return `${seconds}s`;
}
