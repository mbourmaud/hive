import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { useAppStore } from "@/store";
import type { ImageAttachment, SlashCommand } from "../../types";
import type { HistoryNavState } from "../prompt-helpers";
import { handleHistoryDown, handleHistoryUp, handleSlashPopoverKeys } from "../prompt-helpers";
import { ACCEPTED_IMAGE_TYPES, fileToAttachment } from "./attachments";
import { loadHistory, MAX_HISTORY, saveHistory } from "./history";
import {
  getPlainText,
  PLACEHOLDER_INTERVAL_MS,
  ROTATING_PLACEHOLDERS,
  setPlainText,
} from "./utils";

// ── Placeholder hook ─────────────────────────────────────────────────────────

export function usePlaceholder(isStreaming: boolean, value: string, queueCount: number): string {
  const [placeholderIndex, setPlaceholderIndex] = useState(() =>
    Math.floor(Math.random() * ROTATING_PLACEHOLDERS.length),
  );

  useEffect(() => {
    if (isStreaming || value.length > 0) return;
    const timer = setInterval(() => {
      setPlaceholderIndex((prev) => (prev + 1) % ROTATING_PLACEHOLDERS.length);
    }, PLACEHOLDER_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [isStreaming, value]);

  if (isStreaming) {
    return queueCount > 0 ? "Type another message to queue..." : "Type to queue a message...";
  }
  return ROTATING_PLACEHOLDERS[placeholderIndex] ?? "Ask anything...";
}

// ── Editor state + handlers hook ─────────────────────────────────────────────

interface UseEditorOptions {
  onSend: (message: string, images?: ImageAttachment[]) => void;
  onAbort: () => void;
  isStreaming: boolean;
  disabled: boolean;
}

export function useEditor({ onSend, onAbort, isStreaming, disabled }: UseEditorOptions) {
  const editorRef = useRef<HTMLDivElement>(null);
  const [value, setValue] = useState("");
  const [composing, setComposing] = useState(false);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [draftValue, setDraftValue] = useState("");
  const historyRef = useRef<string[]>(loadHistory());

  // Slash command popover
  const [slashVisible, setSlashVisible] = useState(false);
  const [slashQuery, setSlashQuery] = useState("");

  // Image attachments
  const [attachments, setAttachments] = useState<ImageAttachment[]>([]);
  const [isDragging, setIsDragging] = useState(false);
  const dragCounterRef = useRef(0);

  // ── Sync prompt draft with store per-project cache ────────────────────
  const activeSessionId = useAppStore((s) => s.activeSessionId);
  const storePromptDraft = useAppStore((s) => s.promptDraft);
  const setPromptDraft = useAppStore((s) => s.setPromptDraft);

  // Restore draft from store when active session changes
  const prevSessionRef = useRef(activeSessionId);
  useEffect(() => {
    if (activeSessionId === prevSessionRef.current) return;
    prevSessionRef.current = activeSessionId;
    const el = editorRef.current;
    if (!el) return;
    setPlainText(el, storePromptDraft);
    setValue(storePromptDraft);
  }, [activeSessionId, storePromptDraft]);

  // Debounced sync editor → store
  const draftTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  useEffect(() => {
    if (draftTimerRef.current) clearTimeout(draftTimerRef.current);
    draftTimerRef.current = setTimeout(() => setPromptDraft(value), 300);
    return () => {
      if (draftTimerRef.current) clearTimeout(draftTimerRef.current);
    };
  }, [value, setPromptDraft]);

  // Auto-resize contenteditable
  useLayoutEffect(() => {
    const el = editorRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, []);

  // Focus on mount and when streaming ends
  useEffect(() => {
    if (!isStreaming && !disabled) editorRef.current?.focus();
  }, [isStreaming, disabled]);

  const handleInput = useCallback(() => {
    const el = editorRef.current;
    if (!el) return;
    const text = getPlainText(el);
    setValue(text);
    if (historyIndex !== -1) setHistoryIndex(-1);
    if (text.startsWith("/") && !text.includes("\n")) {
      setSlashQuery(text.slice(1));
      setSlashVisible(true);
    } else {
      setSlashVisible(false);
      setSlashQuery("");
    }
  }, [historyIndex]);

  const handleSubmit = useCallback(() => {
    const trimmed = value.trim();
    if ((!trimmed && attachments.length === 0) || disabled) return;
    if (trimmed) {
      const history = historyRef.current;
      if (history[history.length - 1] !== trimmed) {
        history.push(trimmed);
        if (history.length > MAX_HISTORY) history.splice(0, history.length - MAX_HISTORY);
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
  }, [value, attachments, disabled, onSend]);

  const handleSlashSelect = useCallback(
    (cmd: SlashCommand) => {
      setSlashVisible(false);
      setSlashQuery("");
      const el = editorRef.current;
      if (!el) return;
      if (cmd.type === "custom") {
        setPlainText(el, `/${cmd.name} `);
        setValue(`/${cmd.name} `);
        return;
      }
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
      setPlainText(el, "");
      setValue("");
      onSend(`/${cmd.name}`);
    },
    [onSend],
  );

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
        .then((a) => {
          setAttachments((prev) => [...prev, ...a]);
        })
        .catch(() => {});
      return;
    }
    const text = e.clipboardData.getData("text/plain");
    if (text) {
      e.preventDefault();
      document.execCommand("insertText", false, text);
    }
  }, []);

  const handleDragEnter = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current += 1;
    if (dragCounterRef.current === 1) setIsDragging(true);
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    dragCounterRef.current -= 1;
    if (dragCounterRef.current === 0) setIsDragging(false);
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
        .then((a) => {
          setAttachments((prev) => [...prev, ...a]);
        })
        .catch(() => {});
    }
  }, []);

  const removeAttachment = useCallback((id: string) => {
    setAttachments((prev) => prev.filter((a) => a.id !== id));
  }, []);

  return {
    editorRef,
    value,
    composing,
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
  };
}
