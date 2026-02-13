import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { ArrowUp, Square } from "lucide-react";
import type { TurnStatus } from "@/types/chat";
import "./prompt-input.css";

// ── Constants ────────────────────────────────────────────────────────────────

const MAX_HISTORY = 100;
const HISTORY_KEY = "hive-prompt-history";

// ── Types ────────────────────────────────────────────────────────────────────

interface PromptInputProps {
  onSend: (message: string) => void;
  onAbort: () => void;
  isStreaming: boolean;
  disabled?: boolean;
  error?: string | null;
  turnStatus?: TurnStatus | null;
  className?: string;
}

// ── History helpers ──────────────────────────────────────────────────────────

function loadHistory(): string[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) return parsed as string[];
    return [];
  } catch {
    return [];
  }
}

function saveHistory(history: string[]): void {
  try {
    localStorage.setItem(HISTORY_KEY, JSON.stringify(history.slice(-MAX_HISTORY)));
  } catch {
    // quota exceeded — silently ignore
  }
}

// ── Status text derivation ───────────────────────────────────────────────────

function deriveStatusText(
  isStreaming: boolean,
  turnStatus: TurnStatus | null | undefined,
  error: string | null | undefined,
): { text: string; variant: "ready" | "busy" | "error" } {
  if (error) {
    return { text: error, variant: "error" };
  }
  if (!isStreaming) {
    return { text: "Ready", variant: "ready" };
  }
  if (turnStatus === "pending") {
    return { text: "Thinking...", variant: "busy" };
  }
  return { text: "Running commands...", variant: "busy" };
}

// ── Component ────────────────────────────────────────────────────────────────

export function PromptInput({
  onSend,
  onAbort,
  isStreaming,
  disabled = false,
  error = null,
  turnStatus = null,
  className,
}: PromptInputProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const [value, setValue] = useState("");
  const [composing, setComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [draftValue, setDraftValue] = useState("");
  const historyRef = useRef<string[]>(loadHistory());

  // ── Auto-resize textarea ─────────────────────────────────────────────────

  useLayoutEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [value]);

  // ── Focus textarea on mount and when streaming ends ──────────────────────

  useEffect(() => {
    if (!isStreaming && !disabled) {
      textareaRef.current?.focus();
    }
  }, [isStreaming, disabled]);

  // ── Submit handler ───────────────────────────────────────────────────────

  const handleSubmit = useCallback(() => {
    const trimmed = value.trim();
    if (!trimmed || isStreaming || disabled) return;

    // Save to history
    const history = historyRef.current;
    if (history[history.length - 1] !== trimmed) {
      history.push(trimmed);
      if (history.length > MAX_HISTORY) {
        history.splice(0, history.length - MAX_HISTORY);
      }
      saveHistory(history);
    }

    setHistoryIndex(-1);
    setDraftValue("");
    setValue("");
    onSend(trimmed);
  }, [value, isStreaming, disabled, onSend]);

  // ── Key handler ──────────────────────────────────────────────────────────

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      // IME composition guard
      if (e.nativeEvent.isComposing || composing || e.keyCode === 229) {
        return;
      }

      // Escape to abort
      if (e.key === "Escape") {
        if (isStreaming) {
          e.preventDefault();
          onAbort();
        }
        return;
      }

      // Enter to submit (Shift+Enter for newline)
      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSubmit();
        return;
      }

      // Arrow Up at start of textarea → cycle history backward
      if (e.key === "ArrowUp") {
        const el = textareaRef.current;
        if (el && el.selectionStart === 0 && el.selectionEnd === 0) {
          e.preventDefault();
          const history = historyRef.current;
          if (history.length === 0) return;

          const newIndex =
            historyIndex === -1
              ? history.length - 1
              : Math.max(0, historyIndex - 1);

          if (historyIndex === -1) {
            setDraftValue(value);
          }

          setHistoryIndex(newIndex);
          setValue(history[newIndex] ?? "");
        }
        return;
      }

      // Arrow Down at end of textarea → cycle history forward
      if (e.key === "ArrowDown") {
        const el = textareaRef.current;
        if (
          el &&
          el.selectionStart === el.value.length &&
          el.selectionEnd === el.value.length &&
          historyIndex !== -1
        ) {
          e.preventDefault();
          const history = historyRef.current;
          const newIndex = historyIndex + 1;

          if (newIndex >= history.length) {
            setHistoryIndex(-1);
            setValue(draftValue);
          } else {
            setHistoryIndex(newIndex);
            setValue(history[newIndex] ?? "");
          }
        }
      }
    },
    [composing, isStreaming, onAbort, handleSubmit, historyIndex, value, draftValue],
  );

  // ── Status ───────────────────────────────────────────────────────────────

  const { text: statusText, variant: statusVariant } = deriveStatusText(
    isStreaming,
    turnStatus,
    error,
  );

  const canSubmit = value.trim().length > 0 && !isStreaming && !disabled;

  return (
    <div data-component="prompt-dock" className={className}>
      <div
        data-slot="prompt-input-container"
        className="mx-auto max-w-3xl px-4"
      >
        {/* Input area */}
        <div
          className={cn(
            "rounded-xl border border-border bg-card shadow-sm transition-colors",
            "focus-within:border-ring focus-within:ring-1 focus-within:ring-ring/30",
            disabled && "opacity-50",
          )}
        >
          <textarea
            ref={textareaRef}
            data-slot="prompt-textarea"
            value={value}
            onChange={(e) => {
              setValue(e.target.value);
              if (historyIndex !== -1) {
                setHistoryIndex(-1);
              }
            }}
            onKeyDown={handleKeyDown}
            onCompositionStart={() => setComposing(true)}
            onCompositionEnd={() => setComposing(false)}
            placeholder={isStreaming ? "Waiting for response..." : "Ask anything..."}
            disabled={disabled}
            rows={1}
            className={cn(
              "w-full bg-transparent px-4 pt-3 pb-2 text-sm text-foreground",
              "placeholder:text-muted-foreground",
              "disabled:cursor-not-allowed",
            )}
          />

          {/* Bottom bar: status + submit button */}
          <div
            data-slot="prompt-status"
            data-streaming={isStreaming ? "true" : "false"}
            className="flex items-center justify-between px-4 pb-2.5"
          >
            {/* Status indicator */}
            <div className="flex items-center gap-1.5 text-muted-foreground">
              <span
                data-slot="status-dot"
                className={cn(
                  "inline-block h-1.5 w-1.5 rounded-full",
                  statusVariant === "ready" && "bg-success",
                  statusVariant === "busy" && "bg-accent",
                  statusVariant === "error" && "bg-destructive",
                )}
              />
              <span className={cn(
                statusVariant === "error" && "text-destructive",
              )}>
                {statusText}
              </span>
            </div>

            {/* Submit / Stop button */}
            {isStreaming ? (
              <Button
                variant="ghost"
                size="icon"
                onClick={onAbort}
                className="h-7 w-7 text-muted-foreground hover:text-destructive"
                aria-label="Stop response"
              >
                <Square className="h-3.5 w-3.5 fill-current" />
              </Button>
            ) : (
              <Button
                variant="default"
                size="icon"
                onClick={handleSubmit}
                disabled={!canSubmit}
                className="h-7 w-7"
                aria-label="Send message"
              >
                <ArrowUp className="h-3.5 w-3.5" />
              </Button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
