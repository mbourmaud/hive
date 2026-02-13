import { useEffect, useRef, useState, useCallback } from "react";
import { Check, Copy } from "lucide-react";
import { cn } from "@/lib/utils";
import { getHighlighter, getThemeName, resolveLanguage } from "@/lib/shiki-highlighter";
import "./code-block.css";

interface CodeBlockProps {
  code: string;
  language?: string;
  lineNumbers?: boolean;
  maxHeight?: number;
  className?: string;
}

export function CodeBlock({
  code,
  language,
  lineNumbers = true,
  maxHeight = 500,
  className,
}: CodeBlockProps) {
  const codeRef = useRef<HTMLDivElement>(null);
  const [highlighted, setHighlighted] = useState(false);
  const [copied, setCopied] = useState(false);
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const resolvedLang = resolveLanguage(language);
  const lines = code.split("\n");
  // Remove trailing empty line from the count (common in code blocks)
  const lineCount =
    lines.length > 1 && lines[lines.length - 1] === ""
      ? lines.length - 1
      : lines.length;

  useEffect(() => {
    let cancelled = false;

    async function highlight() {
      if (resolvedLang === "text") return;

      try {
        const hl = await getHighlighter();
        if (cancelled) return;

        const theme = getThemeName();
        const html = hl.codeToHtml(code, {
          lang: resolvedLang,
          theme,
        });

        if (cancelled || !codeRef.current) return;
        codeRef.current.innerHTML = html;
        setHighlighted(true);
      } catch {
        // Highlighting failed — keep raw <pre> as fallback
      }
    }

    setHighlighted(false);
    highlight();

    return () => {
      cancelled = true;
    };
  }, [code, resolvedLang]);

  // Re-highlight when theme changes
  useEffect(() => {
    if (!highlighted) return;

    const observer = new MutationObserver(async () => {
      try {
        const hl = await getHighlighter();
        const theme = getThemeName();
        const html = hl.codeToHtml(code, { lang: resolvedLang, theme });
        if (codeRef.current) {
          codeRef.current.innerHTML = html;
        }
      } catch {
        // Ignore theme-switch highlighting errors
      }
    });

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });

    return () => observer.disconnect();
  }, [highlighted, code, resolvedLang]);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(code).then(() => {
      setCopied(true);
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
      copyTimeoutRef.current = setTimeout(() => setCopied(false), 2000);
    });
  }, [code]);

  // Cleanup copy timeout
  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
    };
  }, []);

  const showLineNumbers = lineNumbers && lineCount > 1;

  return (
    <div
      data-component="code-block"
      className={cn("group", className)}
    >
      {resolvedLang !== "text" && (
        <span data-slot="code-block-lang">{resolvedLang}</span>
      )}

      <button
        data-slot="code-block-copy"
        type="button"
        onClick={handleCopy}
        className="inline-flex items-center justify-center rounded-md p-1.5 text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
        aria-label="Copy code"
      >
        {copied ? (
          <Check className="h-3.5 w-3.5 text-success" />
        ) : (
          <Copy className="h-3.5 w-3.5" />
        )}
      </button>

      <div
        data-slot="code-block-scroll"
        style={{ maxHeight: `${maxHeight}px` }}
      >
        {showLineNumbers ? (
          <div data-slot="code-block-table">
            <div data-slot="code-block-gutter">
              {Array.from({ length: lineCount }, (_, i) => (
                <div key={i}>{i + 1}</div>
              ))}
            </div>
            <div data-slot="code-block-lines" ref={codeRef}>
              <pre>
                <code>{code}</code>
              </pre>
            </div>
          </div>
        ) : (
          <div ref={codeRef}>
            <pre>
              <code>{code}</code>
            </pre>
          </div>
        )}
      </div>
    </div>
  );
}
