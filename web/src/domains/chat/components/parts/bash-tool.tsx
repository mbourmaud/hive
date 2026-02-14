import { useState } from "react";
import { cn } from "@/lib/utils";
import { Copy, Check, Terminal } from "lucide-react";

interface BashToolProps {
  command: string;
  output: string;
  exitCode: number;
  description?: string;
}

interface AnsiSegment {
  text: string;
  bold?: boolean;
  underline?: boolean;
  color?: string;
  bgColor?: string;
}

const ANSI_COLORS: Record<number, string> = {
  30: "text-[#000000]",
  31: "text-[#cd3131]",
  32: "text-[#0dbc79]",
  33: "text-[#e5e510]",
  34: "text-[#2472c8]",
  35: "text-[#bc3fbc]",
  36: "text-[#11a8cd]",
  37: "text-[#e5e5e5]",
  90: "text-[#666666]",
  91: "text-[#f14c4c]",
  92: "text-[#23d18b]",
  93: "text-[#f5f543]",
  94: "text-[#3b8eea]",
  95: "text-[#d670d6]",
  96: "text-[#29b8db]",
  97: "text-[#ffffff]",
};

const ANSI_BG_COLORS: Record<number, string> = {
  40: "bg-[#000000]",
  41: "bg-[#cd3131]",
  42: "bg-[#0dbc79]",
  43: "bg-[#e5e510]",
  44: "bg-[#2472c8]",
  45: "bg-[#bc3fbc]",
  46: "bg-[#11a8cd]",
  47: "bg-[#e5e5e5]",
};

function parseAnsi(text: string): AnsiSegment[] {
  const segments: AnsiSegment[] = [];
  const regex = /\x1b\[([0-9;]+)m/g;
  let lastIndex = 0;
  let currentStyle: Partial<AnsiSegment> = {};

  let match: RegExpExecArray | null;
  while ((match = regex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      const textContent = text.slice(lastIndex, match.index);
      if (textContent) {
        segments.push({ text: textContent, ...currentStyle });
      }
    }

    const codes = (match[1] ?? "").split(";").map(Number);
    for (const code of codes) {
      if (code === 0) {
        currentStyle = {};
      } else if (code === 1) {
        currentStyle.bold = true;
      } else if (code === 4) {
        currentStyle.underline = true;
      } else if (code >= 30 && code <= 37) {
        currentStyle.color = ANSI_COLORS[code];
      } else if (code >= 40 && code <= 47) {
        currentStyle.bgColor = ANSI_BG_COLORS[code];
      } else if (code >= 90 && code <= 97) {
        currentStyle.color = ANSI_COLORS[code];
      }
    }

    lastIndex = regex.lastIndex;
  }

  if (lastIndex < text.length) {
    segments.push({ text: text.slice(lastIndex), ...currentStyle });
  }

  return segments;
}

export function BashTool({ command, output, exitCode, description }: BashToolProps) {
  const [copied, setCopied] = useState(false);
  const truncatedCommand = command.length > 60 ? command.slice(0, 60) + "..." : command;

  const handleCopy = async () => {
    await navigator.clipboard.writeText(output);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const segments = parseAnsi(output);

  return (
    <div className="rounded-lg border border-border overflow-hidden">
      {/* Toolbar */}
      <div className="flex items-center justify-between px-2 py-1.5 bg-surface-inset border-b border-border">
        <div className="flex items-center gap-2">
          <Terminal className="w-3.5 h-3.5 text-muted-foreground" />
          <span className="text-[11px] font-mono text-muted-foreground" title={command}>
            {truncatedCommand}
          </span>
          <span
            className={cn(
              "px-1.5 py-0.5 rounded text-[10px] font-semibold",
              exitCode === 0
                ? "bg-success text-success-foreground"
                : "bg-destructive text-destructive-foreground"
            )}
          >
            exit {exitCode}
          </span>
        </div>
        <button
          onClick={handleCopy}
          className={cn(
            "flex items-center gap-1.5 px-2 py-1 rounded text-[11px] font-medium transition-colors",
            copied
              ? "text-success"
              : "hover:bg-muted/50 text-muted-foreground hover:text-foreground"
          )}
          title="Copy output to clipboard"
        >
          {copied ? <Check className="w-3.5 h-3.5" /> : <Copy className="w-3.5 h-3.5" />}
          {copied ? "Copied!" : "Copy"}
        </button>
      </div>

      {/* Description */}
      {description && (
        <div className="px-3 py-2 text-xs text-muted-foreground border-b border-border bg-card">
          {description}
        </div>
      )}

      {/* Output content */}
      <div className="max-h-[400px] overflow-y-auto font-mono text-xs leading-relaxed bg-surface-inset">
        {output.length === 0 ? (
          <div className="p-3 text-muted-foreground">No output</div>
        ) : (
          <div className="p-3 whitespace-pre-wrap break-all">
            {segments.map((segment, i) => (
              <span
                key={i}
                className={cn(
                  segment.color,
                  segment.bgColor,
                  segment.bold && "font-bold",
                  segment.underline && "underline",
                  !segment.color && "text-foreground"
                )}
              >
                {segment.text}
              </span>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
