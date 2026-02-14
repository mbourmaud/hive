import { ArrowUp, ImageIcon, Square, X } from "lucide-react";
import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { EffortLevel } from "@/domains/settings/store";
import type { Model } from "@/domains/settings/types";
import { cn } from "@/shared/lib/utils";
import { Button } from "@/shared/ui/button";
import type { ContextUsage, ImageAttachment, SlashCommand, TurnStatus } from "../types";
import { ContextUsageIndicator } from "./context-usage";
import { EffortToggle } from "./effort-toggle";
import { ModelSelector } from "./model-selector";
import {
  type HistoryNavState,
  handleHistoryDown,
  handleHistoryUp,
  handleSlashPopoverKeys,
} from "./prompt-helpers";
import { SlashPopover } from "./slash-popover";
import "./prompt-input.css";

// ── Constants ────────────────────────────────────────────────────────────────

const MAX_HISTORY = 100;
const HISTORY_KEY = "hive-prompt-history";
const PLACEHOLDER_INTERVAL_MS = 6000;

const ROTATING_PLACEHOLDERS = [
  "Ask anything...",
  "Fix the failing test in auth.rs",
  "Explain how the event system works",
  "Add a dark mode toggle component",
  "Refactor this function to use async/await",
  "Write a migration for the users table",
  "What does this error mean?",
  "Help me debug the SSE connection",
  "Create a React hook for pagination",
  "Optimize this database query",
  "Add input validation to the form",
  "Review my PR for security issues",
  "Generate types from this API response",
  "Write unit tests for the parser",
];

const ACCEPTED_IMAGE_TYPES = [
  "image/png",
  "image/jpeg",
  "image/gif",
  "image/webp",
  "image/svg+xml",
];

// ── Types ────────────────────────────────────────────────────────────────────

interface PromptInputProps {
  onSend: (message: string, images?: ImageAttachment[]) => void;
  onAbort: () => void;
  isStreaming: boolean;
  disabled?: boolean;
  error?: string | null;
  turnStatus?: TurnStatus | null;
  className?: string;
  models?: Model[];
  selectedModel?: string;
  onModelChange?: (modelId: string) => void;
  contextUsage?: ContextUsage | null;
  effort?: EffortLevel;
  onEffortChange?: (effort: EffortLevel) => void;
}

// ── History helpers ──────────────────────────────────────────────────────────

function loadHistory(): string[] {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    if (Array.isArray(parsed) && parsed.every((item): item is string => typeof item === "string")) {
      return parsed;
    }
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

// ── Unique ID generator ─────────────────────────────────────────────────────

let idCounter = 0;
function uniqueId(prefix: string): string {
  idCounter += 1;
  return `${prefix}-${Date.now()}-${idCounter}`;
}

// ── File → ImageAttachment ──────────────────────────────────────────────────

function fileToAttachment(file: File): Promise<ImageAttachment> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const result = reader.result;
      if (typeof result !== "string") {
        reject(new Error(`Expected data URL string for ${file.name}`));
        return;
      }
      resolve({
        id: uniqueId("img"),
        dataUrl: result,
        mimeType: file.type,
        name: file.name,
      });
    };
    reader.onerror = () => reject(new Error(`Failed to read ${file.name}`));
    reader.readAsDataURL(file);
  });
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

// ── Extract plain text from contenteditable ─────────────────────────────────

function getPlainText(el: HTMLDivElement): string {
  // innerText respects line breaks from <br> and block elements
  return el.innerText ?? "";
}

function setPlainText(el: HTMLDivElement, text: string): void {
  el.textContent = text;
  // Move cursor to end
  if (text.length > 0) {
    const range = document.createRange();
    const sel = window.getSelection();
    range.selectNodeContents(el);
    range.collapse(false);
    sel?.removeAllRanges();
    sel?.addRange(range);
  }
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
  models,
  selectedModel,
  onModelChange,
  contextUsage,
  effort,
  onEffortChange,
}: PromptInputProps) {
  const editorRef = useRef<HTMLDivElement>(null);
  const [value, setValue] = useState("");
  const [composing, setComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [draftValue, setDraftValue] = useState("");
  const historyRef = useRef<string[]>(loadHistory());

  // Image attachments
  const [attachments, setAttachments] = useState<ImageAttachment[]>([]);
  const [isDragging, setIsDragging] = useState(false);
  const dragCounterRef = useRef(0);

  // Slash command popover
  const [slashVisible, setSlashVisible] = useState(false);
  const [slashQuery, setSlashQuery] = useState("");

  // Rotating placeholder
  const [placeholderIndex, setPlaceholderIndex] = useState(() =>
    Math.floor(Math.random() * ROTATING_PLACEHOLDERS.length),
  );

  // ── Rotating placeholder ──────────────────────────────────────────────────

  useEffect(() => {
    if (isStreaming || value.length > 0) return;

    const timer = setInterval(() => {
      setPlaceholderIndex((prev) => (prev + 1) % ROTATING_PLACEHOLDERS.length);
    }, PLACEHOLDER_INTERVAL_MS);

    return () => clearInterval(timer);
  }, [isStreaming, value]);

  const placeholder = isStreaming
    ? "Waiting for response..."
    : (ROTATING_PLACEHOLDERS[placeholderIndex] ?? "Ask anything...");

  // ── Auto-resize contenteditable ───────────────────────────────────────────

  useLayoutEffect(() => {
    const el = editorRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, []);

  // ── Focus on mount and when streaming ends ────────────────────────────────

  useEffect(() => {
    if (!isStreaming && !disabled) {
      editorRef.current?.focus();
    }
  }, [isStreaming, disabled]);

  // ── Sync value from contenteditable ───────────────────────────────────────

  const handleInput = useCallback(() => {
    const el = editorRef.current;
    if (!el) return;

    const text = getPlainText(el);
    setValue(text);

    if (historyIndex !== -1) {
      setHistoryIndex(-1);
    }

    // Slash command detection: starts with / at beginning
    if (text.startsWith("/") && !text.includes("\n")) {
      const query = text.slice(1);
      setSlashQuery(query);
      setSlashVisible(true);
    } else {
      setSlashVisible(false);
      setSlashQuery("");
    }
  }, [historyIndex]);

  // ── Submit handler ────────────────────────────────────────────────────────

  const handleSubmit = useCallback(() => {
    const trimmed = value.trim();
    if ((!trimmed && attachments.length === 0) || isStreaming || disabled) return;

    // Save to history
    if (trimmed) {
      const history = historyRef.current;
      if (history[history.length - 1] !== trimmed) {
        history.push(trimmed);
        if (history.length > MAX_HISTORY) {
          history.splice(0, history.length - MAX_HISTORY);
        }
        saveHistory(history);
      }
    }

    setHistoryIndex(-1);
    setDraftValue("");
    setValue("");
    setSlashVisible(false);
    setSlashQuery("");

    const el = editorRef.current;
    if (el) el.textContent = "";

    const imgs = attachments.length > 0 ? [...attachments] : undefined;
    setAttachments([]);
    onSend(trimmed, imgs);
  }, [value, attachments, isStreaming, disabled, onSend]);

  // ── Slash command selection ───────────────────────────────────────────────

  const handleSlashSelect = useCallback(
    (cmd: SlashCommand) => {
      setSlashVisible(false);
      setSlashQuery("");

      const el = editorRef.current;
      if (!el) return;

      // Custom commands: insert text and let user type argument
      if (cmd.type === "custom") {
        setPlainText(el, `/${cmd.name} `);
        setValue(`/${cmd.name} `);
        return;
      }

      // Commands that need arguments: insert text and let user type
      if (
        cmd.name === "model" ||
        cmd.name === "launch" ||
        cmd.name === "stop" ||
        cmd.name === "logs"
      ) {
        setPlainText(el, `/${cmd.name} `);
        setValue(`/${cmd.name} `);
        return;
      }

      // For other built-in commands, submit immediately
      setPlainText(el, "");
      setValue("");
      onSend(`/${cmd.name}`);
    },
    [onSend],
  );

  // ── Key handler ───────────────────────────────────────────────────────────

  const historyNavState: HistoryNavState = useMemo(
    () => ({
      historyRef,
      historyIndex,
      setHistoryIndex,
      value,
      setValue,
      draftValue,
      setDraftValue,
      setPlainText,
    }),
    [historyIndex, value, draftValue],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>) => {
      if (e.nativeEvent.isComposing || composing || e.keyCode === 229) return;
      if (handleSlashPopoverKeys(e, slashVisible, setSlashVisible)) return;

      if (e.key === "Escape") {
        if (isStreaming) {
          e.preventDefault();
          onAbort();
        }
        return;
      }

      if (e.key === "Enter" && !e.shiftKey) {
        e.preventDefault();
        handleSubmit();
        return;
      }

      if (handleHistoryUp(e, editorRef.current, historyNavState)) return;
      handleHistoryDown(e, editorRef.current, historyNavState);
    },
    [composing, isStreaming, onAbort, handleSubmit, historyNavState, slashVisible],
  );

  // ── Paste handler (images + plain text) ───────────────────────────────────

  const handlePaste = useCallback((e: React.ClipboardEvent<HTMLDivElement>) => {
    const items = e.clipboardData.items;
    const imageFiles: File[] = [];

    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      if (item && item.kind === "file" && ACCEPTED_IMAGE_TYPES.includes(item.type)) {
        const file = item.getAsFile();
        if (file) imageFiles.push(file);
      }
    }

    if (imageFiles.length > 0) {
      e.preventDefault();
      void Promise.all(imageFiles.map(fileToAttachment))
        .then((newAttachments) => {
          setAttachments((prev) => [...prev, ...newAttachments]);
        })
        .catch(() => {});
      return;
    }

    // For plain text, prevent rich-text paste
    const text = e.clipboardData.getData("text/plain");
    if (text) {
      e.preventDefault();
      document.execCommand("insertText", false, text);
    }
  }, []);

  // ── Drop handler ──────────────────────────────────────────────────────────

  const handleDragEnter = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current += 1;
    if (dragCounterRef.current === 1) {
      setIsDragging(true);
    }
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current -= 1;
    if (dragCounterRef.current === 0) {
      setIsDragging(false);
    }
  }, []);

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
  }, []);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current = 0;
    setIsDragging(false);

    const files = Array.from(e.dataTransfer.files).filter((f) =>
      ACCEPTED_IMAGE_TYPES.includes(f.type),
    );

    if (files.length > 0) {
      void Promise.all(files.map(fileToAttachment))
        .then((newAttachments) => {
          setAttachments((prev) => [...prev, ...newAttachments]);
        })
        .catch(() => {});
    }
  }, []);

  // ── Remove attachment ─────────────────────────────────────────────────────

  const removeAttachment = useCallback((id: string) => {
    setAttachments((prev) => prev.filter((a) => a.id !== id));
  }, []);

  // ── Status ────────────────────────────────────────────────────────────────

  const { text: statusText, variant: statusVariant } = deriveStatusText(
    isStreaming,
    turnStatus,
    error,
  );

  const canSubmit =
    (value.trim().length > 0 || attachments.length > 0) && !isStreaming && !disabled;

  return (
    // biome-ignore lint/a11y/noStaticElementInteractions: drag-drop container, not interactive
    <div
      data-component="prompt-dock"
      className={className}
      onDragEnter={handleDragEnter}
      onDragLeave={handleDragLeave}
      onDragOver={handleDragOver}
      onDrop={handleDrop}
    >
      <div data-slot="prompt-input-container" className="relative mx-auto max-w-3xl px-4">
        {/* Slash command popover */}
        <SlashPopover
          query={slashQuery}
          visible={slashVisible}
          onSelect={handleSlashSelect}
          onClose={() => setSlashVisible(false)}
          anchorRef={editorRef}
        />

        {/* Drag overlay */}
        {isDragging && (
          <div data-slot="drag-overlay">
            <ImageIcon className="h-6 w-6" />
            <span>Drop file to attach</span>
          </div>
        )}

        {/* Input area */}
        <div
          className={cn(
            "rounded-xl border border-border bg-card shadow-sm transition-colors",
            "focus-within:border-ring focus-within:ring-1 focus-within:ring-ring/30",
            disabled && "opacity-50",
            isDragging && "border-ring ring-2 ring-ring/30",
          )}
        >
          {/* Contenteditable editor */}
          <div data-slot="editor-wrapper" className="relative">
            {/* biome-ignore lint/a11y/useSemanticElements: contentEditable div requires role="textbox" */}
            <div
              ref={editorRef}
              data-slot="prompt-editor"
              contentEditable={!disabled}
              role="textbox"
              tabIndex={0}
              aria-multiline="true"
              aria-placeholder={placeholder}
              suppressContentEditableWarning
              onInput={handleInput}
              onKeyDown={handleKeyDown}
              onPaste={handlePaste}
              onCompositionStart={() => setComposing(true)}
              onCompositionEnd={() => {
                setComposing(false);
                // Sync value after composition ends
                handleInput();
              }}
              className={cn(
                "w-full bg-transparent px-4 pt-3 pb-2 text-sm text-foreground",
                "outline-none",
                disabled && "cursor-not-allowed",
              )}
            />
            {/* Placeholder overlay */}
            {value.length === 0 && (
              <div data-slot="editor-placeholder" aria-hidden="true">
                {placeholder}
              </div>
            )}
          </div>

          {/* Image attachment previews */}
          {attachments.length > 0 && (
            <div data-slot="attachment-bar">
              {attachments.map((att) => (
                <div key={att.id} data-slot="attachment-thumb">
                  <img
                    src={att.dataUrl}
                    alt={att.name}
                    className="h-full w-full object-cover rounded"
                  />
                  <button
                    type="button"
                    data-slot="attachment-remove"
                    onClick={() => removeAttachment(att.id)}
                    aria-label={`Remove ${att.name}`}
                  >
                    <X className="h-3 w-3" />
                  </button>
                </div>
              ))}
            </div>
          )}

          {/* Bottom bar: status + submit button */}
          <div
            data-slot="prompt-status"
            data-streaming={isStreaming ? "true" : "false"}
            className="flex items-center justify-between px-4 pb-2.5"
          >
            {/* Status + model selector */}
            <div className="flex items-center gap-2 text-muted-foreground">
              <div className="flex items-center gap-1.5">
                <span
                  data-slot="status-dot"
                  className={cn(
                    "inline-block h-1.5 w-1.5 rounded-full",
                    statusVariant === "ready" && "bg-success",
                    statusVariant === "busy" && "bg-accent",
                    statusVariant === "error" && "bg-destructive",
                  )}
                />
                <span className={cn(statusVariant === "error" && "text-destructive")}>
                  {statusText}
                </span>
              </div>
              {models && models.length > 0 && selectedModel && onModelChange && (
                <>
                  <span className="text-border">|</span>
                  <ModelSelector
                    models={models}
                    selected={selectedModel}
                    onChange={onModelChange}
                    disabled={isStreaming}
                  />
                </>
              )}
              {effort && onEffortChange && (
                <>
                  <span className="text-border">|</span>
                  <EffortToggle effort={effort} onChange={onEffortChange} disabled={isStreaming} />
                </>
              )}
              {contextUsage && (
                <>
                  <span className="text-border">|</span>
                  <ContextUsageIndicator usage={contextUsage} />
                </>
              )}
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
