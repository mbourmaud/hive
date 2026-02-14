import { useState, useEffect } from "react";
import { cn } from "@/lib/utils";
import { Copy, Check, Monitor } from "lucide-react";
import type { Change } from "diff";
import { diffLines } from "diff";
import "./diff-viewer.css";

interface DiffViewerProps {
  oldString: string;
  newString: string;
  fileName?: string;
}

type ViewMode = "split" | "unified";

interface ProcessedLine {
  type: "add" | "remove" | "unchanged";
  oldLineNum: number | null;
  newLineNum: number | null;
  content: string;
}

const CONTEXT_LINES = 3;

export function DiffViewer({ oldString, newString, fileName }: DiffViewerProps) {
  const [viewMode, setViewMode] = useState<ViewMode>("split");
  const [copiedSide, setCopiedSide] = useState<"before" | "after" | null>(null);
  const [processedLines, setProcessedLines] = useState<ProcessedLine[]>([]);

  useEffect(() => {
    const changes: Change[] = diffLines(oldString, newString);
    const lines: ProcessedLine[] = [];
    let oldLineNum = 1;
    let newLineNum = 1;

    changes.forEach((change) => {
      const content = change.value;
      const lineCount = content.split("\n").filter((l) => l !== "").length;

      if (change.added) {
        for (let i = 0; i < lineCount; i++) {
          lines.push({
            type: "add",
            oldLineNum: null,
            newLineNum: newLineNum++,
            content: content.split("\n")[i] || "",
          });
        }
      } else if (change.removed) {
        for (let i = 0; i < lineCount; i++) {
          lines.push({
            type: "remove",
            oldLineNum: oldLineNum++,
            newLineNum: null,
            content: content.split("\n")[i] || "",
          });
        }
      } else {
        for (let i = 0; i < lineCount; i++) {
          lines.push({
            type: "unchanged",
            oldLineNum: oldLineNum++,
            newLineNum: newLineNum++,
            content: content.split("\n")[i] || "",
          });
        }
      }
    });

    setProcessedLines(lines);
  }, [oldString, newString]);

  const handleCopy = async (side: "before" | "after") => {
    const text = side === "before" ? oldString : newString;
    await navigator.clipboard.writeText(text);
    setCopiedSide(side);
    setTimeout(() => setCopiedSide(null), 2000);
  };

  const collapsedLines = collapseUnchanged(processedLines, CONTEXT_LINES);

  const additionCount = processedLines.filter((l) => l.type === "add").length;
  const deletionCount = processedLines.filter((l) => l.type === "remove").length;

  return (
    <div className="diff-viewer">
      {/* Toolbar */}
      <div className="diff-toolbar">
        <div className="flex items-center gap-2">
          {fileName && <span className="text-xs font-mono text-muted-foreground">{fileName}</span>}
          <span className="text-[11px] text-success">+{additionCount}</span>
          <span className="text-[11px] text-destructive">-{deletionCount}</span>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={() => setViewMode(viewMode === "split" ? "unified" : "split")}
            className={cn(
              "flex items-center gap-1.5 px-2 py-1 rounded text-[11px] font-medium transition-colors",
              "hover:bg-muted/50 text-muted-foreground hover:text-foreground"
            )}
            title={viewMode === "split" ? "Switch to unified view" : "Switch to split view"}
          >
            <Monitor className="w-3.5 h-3.5" />
            {viewMode === "split" ? "Split" : "Unified"}
          </button>
        </div>
      </div>

      {/* Diff content */}
      {viewMode === "split" ? (
        <div className="diff-split-view">
          {/* Before (left) */}
          <div className="diff-pane">
            <div className="diff-pane-header">
              <span className="text-xs font-medium text-muted-foreground">Before</span>
              <button
                onClick={() => handleCopy("before")}
                className={cn(
                  "flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium transition-colors",
                  copiedSide === "before"
                    ? "text-success"
                    : "hover:bg-muted/50 text-muted-foreground hover:text-foreground"
                )}
                title="Copy before content"
              >
                {copiedSide === "before" ? <Check className="w-3 h-3" /> : <Copy className="w-3 h-3" />}
              </button>
            </div>
            <div className="diff-code">
              {collapsedLines.map((item, idx) => {
                if (item.type === "ellipsis") {
                  return (
                    <div key={idx} className="diff-ellipsis">
                      <span className="diff-line-num"></span>
                      <span className="diff-line-content text-muted-foreground">⋯ {item.count} unchanged lines</span>
                    </div>
                  );
                }
                const line = item.line;
                if (line.type === "add") return null;
                return (
                  <div
                    key={idx}
                    className={cn("diff-line", line.type === "remove" && "diff-line-remove")}
                  >
                    <span className="diff-line-num">{line.oldLineNum ?? ""}</span>
                    <span className="diff-line-content">{line.content}</span>
                  </div>
                );
              })}
            </div>
          </div>

          {/* After (right) */}
          <div className="diff-pane">
            <div className="diff-pane-header">
              <span className="text-xs font-medium text-muted-foreground">After</span>
              <button
                onClick={() => handleCopy("after")}
                className={cn(
                  "flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] font-medium transition-colors",
                  copiedSide === "after"
                    ? "text-success"
                    : "hover:bg-muted/50 text-muted-foreground hover:text-foreground"
                )}
                title="Copy after content"
              >
                {copiedSide === "after" ? <Check className="w-3 h-3" /> : <Copy className="w-3 h-3" />}
              </button>
            </div>
            <div className="diff-code">
              {collapsedLines.map((item, idx) => {
                if (item.type === "ellipsis") {
                  return (
                    <div key={idx} className="diff-ellipsis">
                      <span className="diff-line-num"></span>
                      <span className="diff-line-content text-muted-foreground">⋯ {item.count} unchanged lines</span>
                    </div>
                  );
                }
                const line = item.line;
                if (line.type === "remove") return null;
                return (
                  <div
                    key={idx}
                    className={cn("diff-line", line.type === "add" && "diff-line-add")}
                  >
                    <span className="diff-line-num">{line.newLineNum ?? ""}</span>
                    <span className="diff-line-content">{line.content}</span>
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      ) : (
        <div className="diff-unified-view">
          <div className="diff-code">
            {collapsedLines.map((item, idx) => {
              if (item.type === "ellipsis") {
                return (
                  <div key={idx} className="diff-ellipsis">
                    <span className="diff-line-num"></span>
                    <span className="diff-line-num"></span>
                    <span className="diff-line-content text-muted-foreground">⋯ {item.count} unchanged lines</span>
                  </div>
                );
              }
              const line = item.line;
              return (
                <div
                  key={idx}
                  className={cn(
                    "diff-line",
                    line.type === "add" && "diff-line-add",
                    line.type === "remove" && "diff-line-remove"
                  )}
                >
                  <span className="diff-line-num">{line.oldLineNum ?? ""}</span>
                  <span className="diff-line-num">{line.newLineNum ?? ""}</span>
                  <span className="diff-line-content">{line.content}</span>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
}

type CollapsedItem =
  | { type: "line"; line: ProcessedLine }
  | { type: "ellipsis"; count: number };

function collapseUnchanged(lines: ProcessedLine[], contextLines: number): CollapsedItem[] {
  const result: CollapsedItem[] = [];
  let unchangedBuffer: ProcessedLine[] = [];

  const flushBuffer = () => {
    if (unchangedBuffer.length === 0) return;
    const needsCollapse = unchangedBuffer.length > contextLines * 2;
    if (needsCollapse) {
      for (const item of unchangedBuffer.slice(0, contextLines)) {
        result.push({ type: "line", line: item });
      }
      const hiddenCount = unchangedBuffer.length - contextLines * 2;
      result.push({ type: "ellipsis", count: hiddenCount });
      for (const item of unchangedBuffer.slice(-contextLines)) {
        result.push({ type: "line", line: item });
      }
    } else {
      for (const item of unchangedBuffer) {
        result.push({ type: "line", line: item });
      }
    }
    unchangedBuffer = [];
  };

  for (const line of lines) {
    if (line.type === "unchanged") {
      unchangedBuffer.push(line);
    } else {
      flushBuffer();
      result.push({ type: "line", line });
    }
  }

  // Flush remaining
  if (unchangedBuffer.length > 0) {
    const needsCollapse = unchangedBuffer.length > contextLines * 2;
    if (needsCollapse) {
      for (const item of unchangedBuffer.slice(0, contextLines)) {
        result.push({ type: "line", line: item });
      }
      const hiddenCount = unchangedBuffer.length - contextLines;
      result.push({ type: "ellipsis", count: hiddenCount });
    } else {
      for (const item of unchangedBuffer) {
        result.push({ type: "line", line: item });
      }
    }
  }

  return result;
}
