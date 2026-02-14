import { useCallback, useEffect, useRef, useState } from "react";
import { apiClient } from "@/shared/api/client";
import type { SlashCommand } from "../types";
import "./slash-popover.css";

// ── Built-in commands ────────────────────────────────────────────────────────

const BUILTIN_COMMANDS: SlashCommand[] = [
  {
    name: "new",
    description: "Start a new session",
    shortcut: "Cmd+N",
    category: "session",
    type: "builtin",
  },
  {
    name: "sessions",
    description: "Browse and switch sessions",
    shortcut: "Cmd+E",
    category: "session",
    type: "builtin",
  },
  {
    name: "clear",
    description: "Clear conversation history",
    shortcut: "Ctrl+L",
    category: "session",
    type: "builtin",
  },
  { name: "compact", description: "Summarize and continue", category: "session", type: "builtin" },
  {
    name: "model",
    description: "Switch model",
    shortcut: "Cmd+'",
    category: "config",
    type: "builtin",
  },
  { name: "steps", description: "Toggle steps visibility", category: "view", type: "builtin" },
  { name: "undo", description: "Undo last message", category: "session", type: "builtin" },
  { name: "launch", description: "Launch a drone", category: "drone", type: "builtin" },
  { name: "status", description: "Show active drones", category: "drone", type: "builtin" },
  { name: "stop", description: "Stop a running drone", category: "drone", type: "builtin" },
  { name: "logs", description: "View drone activity", category: "drone", type: "builtin" },
  { name: "help", description: "Show available commands", category: "info", type: "builtin" },
];

// ── Fetch custom commands from API ──────────────────────────────────────────

interface ApiCommand {
  name: string;
  description: string;
  source: "project" | "user" | "tools";
}

function useCustomCommands(): SlashCommand[] {
  const [customs, setCustoms] = useState<SlashCommand[]>([]);

  useEffect(() => {
    let cancelled = false;

    apiClient
      .get<ApiCommand[]>("/api/commands")
      .then((data) => {
        if (cancelled) return;
        const cmds: SlashCommand[] = data.map((c) => ({
          name: c.name,
          description: c.description,
          type: "custom" as const,
          source: c.source,
        }));
        setCustoms(cmds);
      })
      .catch(() => {
        // Silently ignore — no custom commands available
      });

    return () => {
      cancelled = true;
    };
  }, []);

  return customs;
}

// ── Types ────────────────────────────────────────────────────────────────────

interface SlashPopoverProps {
  query: string;
  visible: boolean;
  onSelect: (command: SlashCommand) => void;
  onClose: () => void;
  anchorRef: React.RefObject<HTMLElement | null>;
}

// ── Component ────────────────────────────────────────────────────────────────

export function SlashPopover({ query, visible, onSelect, onClose, anchorRef }: SlashPopoverProps) {
  const [activeIndex, setActiveIndex] = useState(0);
  const listRef = useRef<HTMLDivElement>(null);
  const customCommands = useCustomCommands();

  // Merge: custom commands first, then built-in
  const allCommands = [...customCommands, ...BUILTIN_COMMANDS];

  // Filter commands based on query (text after /)
  const filtered = allCommands.filter((cmd) =>
    cmd.name.toLowerCase().startsWith(query.toLowerCase()),
  );

  // Reset active index when filter changes
  useEffect(() => {
    setActiveIndex(0);
  }, []);

  // Scroll active item into view
  useEffect(() => {
    if (!listRef.current) return;
    const activeEl = listRef.current.querySelector("[data-active='true']");
    if (activeEl) {
      activeEl.scrollIntoView({ block: "nearest" });
    }
  }, []);

  // Keyboard navigation
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (!visible || filtered.length === 0) return;

      if (e.key === "ArrowDown") {
        e.preventDefault();
        setActiveIndex((prev) => (prev + 1) % filtered.length);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setActiveIndex((prev) => (prev - 1 + filtered.length) % filtered.length);
      } else if (e.key === "Enter" || e.key === "Tab") {
        e.preventDefault();
        const cmd = filtered[activeIndex];
        if (cmd) onSelect(cmd);
      } else if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    },
    [visible, filtered, activeIndex, onSelect, onClose],
  );

  useEffect(() => {
    if (!visible) return;
    document.addEventListener("keydown", handleKeyDown, { capture: true });
    return () => document.removeEventListener("keydown", handleKeyDown, { capture: true });
  }, [visible, handleKeyDown]);

  // Close on outside click
  useEffect(() => {
    if (!visible) return;
    const handler = (e: MouseEvent) => {
      const target = e.target;
      if (!(target instanceof Node)) return;
      const popoverEl = listRef.current;
      const anchorEl = anchorRef.current;
      if (
        popoverEl &&
        !popoverEl.contains(target) &&
        anchorEl &&
        !anchorEl.contains(target)
      ) {
        onClose();
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [visible, onClose, anchorRef]);

  if (!visible || filtered.length === 0) return null;

  return (
    <div ref={listRef} data-component="slash-popover">
      <div data-slot="slash-popover-header">Commands</div>
      {filtered.map((cmd, idx) => (
        <button
          key={`${cmd.type ?? "builtin"}-${cmd.name}`}
          type="button"
          data-slot="slash-popover-item"
          data-active={idx === activeIndex ? "true" : "false"}
          onMouseEnter={() => setActiveIndex(idx)}
          onMouseDown={(e) => {
            e.preventDefault();
            onSelect(cmd);
          }}
        >
          <div data-slot="slash-popover-item-left">
            <span data-slot="slash-popover-name">/{cmd.name}</span>
            <span data-slot="slash-popover-desc">{cmd.description}</span>
          </div>
          <div data-slot="slash-popover-item-right">
            {cmd.type === "custom" && cmd.source && (
              <span data-slot="slash-popover-badge">
                {cmd.source === "tools" ? "Tool" : cmd.source === "project" ? "Project" : "Skill"}
              </span>
            )}
            {cmd.shortcut && <kbd data-slot="slash-popover-kbd">{cmd.shortcut}</kbd>}
          </div>
        </button>
      ))}
    </div>
  );
}

export { BUILTIN_COMMANDS as COMMANDS };
export type { SlashPopoverProps };
