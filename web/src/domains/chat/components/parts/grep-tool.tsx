import { useState } from "react";
import { ChevronDown, ChevronRight, FileText } from "lucide-react";

interface GrepToolProps {
  pattern: string;
  results: string;
}

interface GrepMatch {
  filePath: string;
  lineNumber: number;
  content: string;
}

interface GroupedMatches {
  [filePath: string]: GrepMatch[];
}

function parseGrepResults(results: string): GroupedMatches {
  const lines = results.split("\n").filter(Boolean);
  const grouped: GroupedMatches = {};

  for (const line of lines) {
    const match = line.match(/^([^:]+):(\d+):(.*)$/);
    if (match) {
      const filePath = match[1];
      const lineNum = match[2];
      const content = match[3];

      if (!filePath || !lineNum || content === undefined) continue;

      const lineNumber = parseInt(lineNum, 10);

      const existing = grouped[filePath];
      if (existing) {
        existing.push({ filePath, lineNumber, content });
      } else {
        grouped[filePath] = [{ filePath, lineNumber, content }];
      }
    }
  }

  return grouped;
}

function highlightMatches(content: string, pattern: string): React.ReactNode {
  try {
    const regex = new RegExp(`(${pattern})`, "gi");
    const parts = content.split(regex);

    return parts.map((part, i) => {
      if (regex.test(part)) {
        return (
          <mark key={i} className="bg-accent/30 text-accent-foreground font-semibold rounded px-0.5">
            {part}
          </mark>
        );
      }
      return <span key={i}>{part}</span>;
    });
  } catch {
    return content;
  }
}

function FileGroup({ filePath, matches, pattern }: { filePath: string; matches: GrepMatch[]; pattern: string }) {
  const [isExpanded, setIsExpanded] = useState(true);

  return (
    <details open={isExpanded} onToggle={(e) => setIsExpanded((e.target as HTMLDetailsElement).open)}>
      <summary className="flex items-center gap-2 px-3 py-2 bg-card-header border-b border-border cursor-pointer hover:bg-muted/30 transition-colors select-none">
        {isExpanded ? (
          <ChevronDown className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        ) : (
          <ChevronRight className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
        )}
        <FileText className="w-3.5 h-3.5 text-accent shrink-0" />
        <span className="font-mono text-xs text-foreground flex-1 truncate" title={filePath}>
          {filePath}
        </span>
        <span className="px-1.5 py-0.5 bg-accent/20 text-accent text-[10px] font-semibold rounded shrink-0">
          {matches.length}
        </span>
      </summary>
      <div className="divide-y divide-border/50">
        {matches.map((match, i) => (
          <div key={i} className="flex hover:bg-muted/30 transition-colors">
            <span className="shrink-0 text-right px-3 py-1 text-[11px] text-muted-foreground/60 font-mono select-none border-r border-border/50 min-w-[4rem]">
              {match.lineNumber}
            </span>
            <div className="flex-1 px-3 py-1 font-mono text-xs whitespace-pre-wrap break-all">
              {highlightMatches(match.content, pattern)}
            </div>
          </div>
        ))}
      </div>
    </details>
  );
}

export function GrepTool({ pattern, results }: GrepToolProps) {
  const grouped = parseGrepResults(results);
  const fileCount = Object.keys(grouped).length;
  const totalMatches = Object.values(grouped).reduce((sum, matches) => sum + matches.length, 0);

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      {/* Header */}
      <div className="px-3 py-2 bg-surface-inset border-b border-border">
        <div className="flex items-center gap-2">
          <span className="text-xs font-mono text-muted-foreground">
            Pattern: <span className="text-accent font-semibold">{pattern}</span>
          </span>
          <span className="text-xs text-muted-foreground">•</span>
          <span className="text-xs text-muted-foreground">
            {totalMatches} {totalMatches === 1 ? "match" : "matches"} in {fileCount} {fileCount === 1 ? "file" : "files"}
          </span>
        </div>
      </div>

      {/* Results */}
      <div className="max-h-[400px] overflow-y-auto bg-surface-inset">
        {totalMatches === 0 ? (
          <div className="p-3 text-xs text-muted-foreground">No matches found</div>
        ) : (
          <div className="divide-y divide-border">
            {Object.entries(grouped).map(([filePath, matches]) => (
              <FileGroup key={filePath} filePath={filePath} matches={matches} pattern={pattern} />
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
