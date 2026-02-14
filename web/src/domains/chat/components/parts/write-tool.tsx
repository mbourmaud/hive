import { useState, useEffect } from "react";
import { FileText, Copy, Check } from "lucide-react";
import { cn } from "@/lib/utils";
import { getHighlighter, getThemeName, resolveLanguage } from "../shiki-highlighter";

interface WriteToolProps {
  filePath: string;
  content: string;
}

function getFileExtension(path: string): string {
  const match = path.match(/\.([^.]+)$/);
  return match?.[1] ?? "";
}

function formatFileSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} bytes`;
  const kb = bytes / 1024;
  if (kb < 1024) return `${kb.toFixed(1)} KB`;
  const mb = kb / 1024;
  return `${mb.toFixed(1)} MB`;
}

function getPathBreadcrumb(path: string): { parts: string[]; fileName: string } {
  const parts = path.split("/").filter(Boolean);
  const fileName = parts.pop() ?? "";
  return { parts, fileName };
}

export function WriteTool({ filePath, content }: WriteToolProps) {
  const [isCollapsed, setIsCollapsed] = useState(true);
  const [copied, setCopied] = useState(false);
  const [highlightedHtml, setHighlightedHtml] = useState<string>("");

  const extension = getFileExtension(filePath);
  const language = resolveLanguage(extension);
  const fileSize = new Blob([content]).size;
  const lineCount = content.split("\n").length;
  const { parts: pathParts, fileName } = getPathBreadcrumb(filePath);

  useEffect(() => {
    let mounted = true;

    const highlight = async () => {
      try {
        const highlighter = await getHighlighter();
        const theme = getThemeName();
        const html = highlighter.codeToHtml(content, {
          lang: language,
          theme,
        });
        if (mounted) {
          setHighlightedHtml(html);
        }
      } catch (error) {
        console.error("Failed to highlight code:", error);
      }
    };

    highlight();
    return () => {
      mounted = false;
    };
  }, [content, language]);

  const handleCopy = () => {
    navigator.clipboard.writeText(content);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <div className="border border-border rounded-lg overflow-hidden bg-card">
      {/* Header */}
      <div
        className="flex items-center gap-2 px-3 py-2 bg-card-header cursor-pointer select-none"
        onClick={() => setIsCollapsed(!isCollapsed)}
      >
        <FileText className="w-4 h-4 text-muted-foreground shrink-0" />
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-1 text-sm font-medium text-foreground">
            {pathParts.length > 0 && (
              <span className="text-muted-foreground truncate">
                {pathParts.join("/")}
                <span className="mx-1">/</span>
              </span>
            )}
            <span className="font-semibold truncate">{fileName}</span>
          </div>
          <div className="text-xs text-muted-foreground mt-0.5">
            {isCollapsed
              ? `Wrote ${lineCount} lines to ${filePath}`
              : formatFileSize(fileSize)}
          </div>
        </div>
        <button
          onClick={(e) => {
            e.stopPropagation();
            handleCopy();
          }}
          className={cn(
            "p-1.5 rounded hover:bg-muted transition-colors shrink-0",
            copied && "text-success"
          )}
          title="Copy content"
        >
          {copied ? <Check className="w-4 h-4" /> : <Copy className="w-4 h-4" />}
        </button>
      </div>

      {/* Content */}
      {!isCollapsed && (
        <div className="bg-surface-inset">
          <div className="overflow-x-auto">
            <div className="inline-block min-w-full">
              <div className="flex font-mono text-xs">
                {/* Line numbers */}
                <div className="select-none bg-muted/30 px-3 py-3 text-muted-foreground border-r border-border">
                  {Array.from({ length: lineCount }, (_, i) => (
                    <div key={i} className="leading-6 text-right">
                      {i + 1}
                    </div>
                  ))}
                </div>
                {/* Code */}
                <div className="flex-1 overflow-x-auto">
                  {highlightedHtml ? (
                    <div
                      className="[&>pre]:!bg-transparent [&>pre]:!p-3 [&>pre]:!m-0 [&>pre>code]:!bg-transparent"
                      dangerouslySetInnerHTML={{ __html: highlightedHtml }}
                    />
                  ) : (
                    <pre className="p-3 m-0">
                      <code>{content}</code>
                    </pre>
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
