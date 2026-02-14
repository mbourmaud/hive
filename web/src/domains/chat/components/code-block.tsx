import { useEffect, useState, useRef } from "react";
import { Check, Copy } from "lucide-react";
import { getHighlighter, getThemeName, resolveLanguage } from "./shiki-highlighter";
import "./code-block.css";

interface CodeBlockProps {
  code: string;
  language?: string;
  lineNumbers?: boolean;
  maxHeight?: string;
  className?: string;
}

export function CodeBlock({
  code,
  language,
  lineNumbers = true,
  maxHeight,
  className = "",
}: CodeBlockProps) {
  const [html, setHtml] = useState<string>("");
  const [copied, setCopied] = useState(false);
  const [currentTheme, setCurrentTheme] = useState(getThemeName());
  const codeRef = useRef<HTMLDivElement>(null);

  const resolvedLang = resolveLanguage(language);

  useEffect(() => {
    let mounted = true;

    async function highlight() {
      try {
        const highlighter = await getHighlighter();
        const theme = getThemeName();

        if (!mounted) return;

        const highlighted = highlighter.codeToHtml(code, {
          lang: resolvedLang,
          theme,
        });

        setHtml(highlighted);
        setCurrentTheme(theme);
      } catch (error) {
        console.error("Failed to highlight code:", error);
        // Fallback to plain text
        setHtml(`<pre><code>${escapeHtml(code)}</code></pre>`);
      }
    }

    highlight();

    return () => {
      mounted = false;
    };
  }, [code, resolvedLang]);

  useEffect(() => {
    const observer = new MutationObserver((mutations) => {
      for (const mutation of mutations) {
        if (
          mutation.type === "attributes" &&
          mutation.attributeName === "data-theme"
        ) {
          const newTheme = getThemeName();
          if (newTheme !== currentTheme) {
            setCurrentTheme(newTheme);
            // Re-highlight with new theme
            getHighlighter().then((highlighter) => {
              const highlighted = highlighter.codeToHtml(code, {
                lang: resolvedLang,
                theme: newTheme,
              });
              setHtml(highlighted);
            });
          }
        }
      }
    });

    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ["data-theme"],
    });

    return () => observer.disconnect();
  }, [code, resolvedLang, currentTheme]);

  const handleCopy = async () => {
    await navigator.clipboard.writeText(code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const lines = code.split("\n");
  const showLineNumbers = lineNumbers && lines.length > 1;

  return (
    <div className={`code-block ${className}`} style={{ maxHeight }}>
      <div className="code-block-header">
        {resolvedLang !== "text" && (
          <span className="code-block-language">{resolvedLang}</span>
        )}
        <button
          onClick={handleCopy}
          className="code-block-copy"
          title="Copy code"
          type="button"
        >
          {copied ? <Check size={16} /> : <Copy size={16} />}
        </button>
      </div>
      <div
        ref={codeRef}
        className={`code-block-content ${showLineNumbers ? "with-line-numbers" : ""}`}
        dangerouslySetInnerHTML={{ __html: html }}
      />
    </div>
  );
}

function escapeHtml(text: string): string {
  const div = document.createElement("div");
  div.textContent = text;
  return div.innerHTML;
}
