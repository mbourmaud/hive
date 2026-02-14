import { ArrowUp, ImageIcon, Square, X } from "lucide-react";
import type { ChatMode, EffortLevel } from "@/domains/settings/store";
import type { Model } from "@/domains/settings/types";
import { cn } from "@/shared/lib/utils";
import { Button } from "@/shared/ui/button";
import type { ContextUsage, ImageAttachment, TurnStatus } from "../../types";
import { ContextUsageIndicator } from "../context-usage";
import { EffortToggle } from "../effort-toggle";
import { ModeToggle } from "../mode-toggle";
import { ModelSelector } from "../model-selector";
import { SlashPopover } from "../slash-popover";
import "../prompt-input.css";

import { useEditor, usePlaceholder } from "./hooks";
import { deriveStatusText } from "./utils";

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
  chatMode?: ChatMode;
  onModeChange?: (mode: ChatMode) => void;
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
  chatMode,
  onModeChange,
}: PromptInputProps) {
  const {
    editorRef,
    value,
    setComposing,
    attachments,
    isDragging,
    slashVisible,
    slashQuery,
    setSlashVisible,
    handleInput,
    handleSubmit,
    handleSlashSelect,
    handleKeyDown,
    handlePaste,
    handleDragEnter,
    handleDragLeave,
    handleDragOver,
    handleDrop,
    removeAttachment,
  } = useEditor({ onSend, onAbort, isStreaming, disabled });

  const placeholder = usePlaceholder(isStreaming, value);
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
        <SlashPopover
          query={slashQuery}
          visible={slashVisible}
          onSelect={handleSlashSelect}
          onClose={() => setSlashVisible(false)}
          anchorRef={editorRef}
        />

        {isDragging && (
          <div data-slot="drag-overlay">
            <ImageIcon className="h-6 w-6" />
            <span>Drop file to attach</span>
          </div>
        )}

        <div
          className={cn(
            "rounded-xl border border-border bg-card shadow-sm transition-colors",
            "focus-within:border-ring focus-within:ring-1 focus-within:ring-ring/30",
            disabled && "opacity-50",
            isDragging && "border-ring ring-2 ring-ring/30",
          )}
        >
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
                handleInput();
              }}
              className={cn(
                "w-full bg-transparent px-4 pt-3 pb-2 text-sm text-foreground",
                "outline-none",
                disabled && "cursor-not-allowed",
              )}
            />
            {value.length === 0 && (
              <div data-slot="editor-placeholder" aria-hidden="true">
                {placeholder}
              </div>
            )}
          </div>

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

          <div data-slot="prompt-toolbar" data-streaming={isStreaming ? "true" : "false"}>
            <div data-slot="toolbar-left">
              <StatusIndicator text={statusText} variant={statusVariant} />
              {chatMode && onModeChange && (
                <ModeToggle mode={chatMode} onChange={onModeChange} disabled={isStreaming} />
              )}
              {models && models.length > 0 && selectedModel && onModelChange && (
                <ModelSelector
                  models={models}
                  selected={selectedModel}
                  onChange={onModelChange}
                  disabled={isStreaming}
                />
              )}
              {effort && onEffortChange && (
                <EffortToggle effort={effort} onChange={onEffortChange} disabled={isStreaming} />
              )}
            </div>

            <div data-slot="toolbar-right">
              {contextUsage && <ContextUsageIndicator usage={contextUsage} />}

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
    </div>
  );
}

// ── Status indicator (extracted for clarity) ─────────────────────────────────

function StatusIndicator({ text, variant }: { text: string; variant: "ready" | "busy" | "error" }) {
  return (
    <div data-slot="status-indicator" data-variant={variant}>
      <span data-slot="status-dot" />
      <span className={cn(variant === "error" && "text-destructive")}>{text}</span>
    </div>
  );
}
