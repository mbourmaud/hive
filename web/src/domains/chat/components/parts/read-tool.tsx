import { useState } from "react";
import { cn } from "@/lib/utils";
import { Copy, Check, FileText, FileCode, FileJson, FileType, File } from "lucide-react";

interface ReadToolProps {
  filePath: string;
  content: string;
  offset?: number;
  limit?: number;
  totalLines?: number;
}

function getFileIcon(filePath: string) {
  const ext = filePath.split(".").pop()?.toLowerCase();
  switch (ext) {
    case "json":
      return FileJson;
    case "ts":
    case "tsx":
    case "js":
    case "jsx":
    case "py":
    case "rs":
    case "go":
    case "java":
    case "c":
    case "cpp":
    case "h":
    case "hpp":
      return FileCode;
    case "txt":
    case "md":
      return FileText;
    case "css":
    case "scss":
    case "html":
      return FileType;
    default:
      return File;
  }
}

function getLanguageClass(filePath: string): string {
  const ext = filePath.split(".").pop()?.toLowerCase();
  switch (ext) {
    case "ts":
    case "tsx":
    case "js":
    case "jsx":
      return "language-typescript";
    case "py":
      return "language-python";
    case "rs":
      return "language-rust";
    case "json":
      return "language-json";
    case "css":
    case "scss":
      return "language-css";
    case "html":
      return "language-html";
    default:
      return "";
  }
}

export function ReadTool({ filePath, content, offset = 0, limit, totalLines }: ReadToolProps) {
  const [copied, setCopied] = useState(false);
  const FileIcon = getFileIcon(filePath);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const lines = content.split("\n");
  const startLine = offset + 1;
  const endLine = limit ? offset + limit : offset + lines.length;

  const shouldTruncate = lines.length > 70;
  const truncatedLines = shouldTruncate
    ? [...lines.slice(0, 50), `... ${lines.length - 70} lines hidden ...`, ...lines.slice(-20)]
    : lines;

  const gutterWidth = String(endLine).length;

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      {/* Toolbar */}
      <div className="flex items-center justify-between px-2 py-1.5 bg-surface-inset border-b border-border">
        <div className="flex items-center gap-2">
          <FileIcon className="w-3.5 h-3.5 text-accent" />
          <span className="text-[11px] font-mono text-muted-foreground truncate max-w-md" title={filePath}>
            {filePath}
          </span>
        </div>
        <div className="flex items-center gap-2">
          <span className="text-[10px] text-muted-foreground">
            {totalLines ? (
              <>
                Showing lines {startLine}-{endLine} of {totalLines} total
              </>
            ) : (
              <>
                {lines.length} {lines.length === 1 ? "line" : "lines"}
              </>
            )}
          </span>
          <button
            onClick={handleCopy}
            className={cn(
              "flex items-center gap-1.5 px-2 py-1 rounded text-[11px] font-medium transition-colors",
              copied
                ? "text-success"
                : "hover:bg-muted/50 text-muted-foreground hover:text-foreground"
            )}
            title="Copy content to clipboard"
          >
            {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
            {copied ? "Copied!" : "Copy"}
          </button>
        </div>
      </div>

      {/* File content */}
      <div className="max-h-[400px] overflow-y-auto font-mono text-xs leading-relaxed bg-surface-inset">
        <pre className={cn("m-0", getLanguageClass(filePath))}>
          {truncatedLines.map((line, i) => {
            const isHiddenMarker = line.startsWith("...");
            const lineNum = isHiddenMarker
              ? ""
              : shouldTruncate && i >= 50
                ? startLine + lines.length - (truncatedLines.length - i)
                : startLine + i;

            return (
              <div
                key={i}
                className={cn(
                  "flex hover:bg-muted/30 transition-colors",
                  isHiddenMarker && "bg-muted/20"
                )}
              >
                <span
                  className={cn(
                    "shrink-0 text-right pr-3 pl-2 py-px select-none border-r",
                    isHiddenMarker
                      ? "text-muted-foreground/60 border-border/30"
                      : "text-muted-foreground/40 border-border/50"
                  )}
                  style={{ minWidth: `${gutterWidth + 2}ch` }}
                >
                  {lineNum}
                </span>
                <span
                  className={cn(
                    "flex-1 px-3 py-px whitespace-pre-wrap break-all",
                    isHiddenMarker ? "text-muted-foreground italic text-center" : "text-foreground"
                  )}
                >
                  {line}
                </span>
              </div>
            );
          })}
        </pre>
      </div>
    </div>
  );
}
