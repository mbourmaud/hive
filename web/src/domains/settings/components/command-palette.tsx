import * as Dialog from "@radix-ui/react-dialog";
import {
  HelpCircle,
  Layers,
  MessageSquare,
  Plus,
  Search,
  Settings,
  Sun,
  Trash2,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "./command-palette.css";

// ── Types ────────────────────────────────────────────────────────────────────

interface CommandItem {
  id: string;
  group: "actions" | "commands" | "sessions";
  icon: React.ReactNode;
  label: string;
  description: string;
  shortcut?: string[];
  onSelect: () => void;
}

interface CommandPaletteProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sessions: { id: string; title: string }[];
  onSelectSession: (id: string) => void;
  onNewSession: () => void;
  onClearChat: () => void;
  onOpenSettings: () => void;
  onToggleTheme: () => void;
  onRunCommand: (command: string) => void;
}

// ── Fuzzy match (case-insensitive substring) ────────────────────────────────

function fuzzyMatch(query: string, text: string): boolean {
  if (!query) return true;
  return text.toLowerCase().includes(query.toLowerCase());
}

// ── Shortcut display ────────────────────────────────────────────────────────

function ShortcutKeys({ keys }: { keys: string[] }) {
  return (
    <span data-slot="cmd-item-shortcut">
      {keys.map((k) => (
        <kbd key={k} data-slot="cmd-kbd">
          {k}
        </kbd>
      ))}
    </span>
  );
}

// ── Component ────────────────────────────────────────────────────────────────

export function CommandPalette({
  open,
  onOpenChange,
  sessions,
  onSelectSession,
  onNewSession,
  onClearChat,
  onOpenSettings,
  onToggleTheme,
  onRunCommand,
}: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Build full item list
  const allItems = useMemo((): CommandItem[] => {
    const actions: CommandItem[] = [
      {
        id: "action-new-session",
        group: "actions",
        icon: <Plus className="w-4 h-4" />,
        label: "New Session",
        description: "Start a new chat session",
        shortcut: ["\u2318", "N"],
        onSelect: onNewSession,
      },
      {
        id: "action-settings",
        group: "actions",
        icon: <Settings className="w-4 h-4" />,
        label: "Settings",
        description: "Open settings dialog",
        shortcut: ["\u2318", ","],
        onSelect: onOpenSettings,
      },
      {
        id: "action-toggle-theme",
        group: "actions",
        icon: <Sun className="w-4 h-4" />,
        label: "Toggle Theme",
        description: "Switch between light and dark mode",
        onSelect: onToggleTheme,
      },
      {
        id: "action-clear-chat",
        group: "actions",
        icon: <Trash2 className="w-4 h-4" />,
        label: "Clear Chat",
        description: "Clear the current conversation",
        onSelect: onClearChat,
      },
    ];

    const commands: CommandItem[] = [
      {
        id: "cmd-clear",
        group: "commands",
        icon: <Trash2 className="w-4 h-4" />,
        label: "/clear",
        description: "Clear conversation history",
        onSelect: () => onRunCommand("/clear"),
      },
      {
        id: "cmd-compact",
        group: "commands",
        icon: <Layers className="w-4 h-4" />,
        label: "/compact",
        description: "Summarize and continue",
        onSelect: () => onRunCommand("/compact"),
      },
      {
        id: "cmd-model",
        group: "commands",
        icon: <Zap className="w-4 h-4" />,
        label: "/model",
        description: "Switch model (e.g. /model sonnet)",
        onSelect: () => onRunCommand("/model"),
      },
      {
        id: "cmd-help",
        group: "commands",
        icon: <HelpCircle className="w-4 h-4" />,
        label: "/help",
        description: "Show available commands",
        onSelect: () => onRunCommand("/help"),
      },
    ];

    const sessionItems: CommandItem[] = sessions.map((s) => ({
      id: `session-${s.id}`,
      group: "sessions" as const,
      icon: <MessageSquare className="w-4 h-4" />,
      label: s.title,
      description: "Switch to session",
      onSelect: () => onSelectSession(s.id),
    }));

    return [...actions, ...commands, ...sessionItems];
  }, [
    sessions,
    onNewSession,
    onOpenSettings,
    onToggleTheme,
    onClearChat,
    onRunCommand,
    onSelectSession,
  ]);

  // Filter items by query
  const filtered = useMemo(() => {
    if (!query) return allItems;
    return allItems.filter(
      (item) => fuzzyMatch(query, item.label) || fuzzyMatch(query, item.description),
    );
  }, [allItems, query]);

  // Group filtered items
  const grouped = useMemo(() => {
    const groups: { key: string; label: string; items: CommandItem[] }[] = [];

    const actionItems = filtered.filter((i) => i.group === "actions");
    const commandItems = filtered.filter((i) => i.group === "commands");
    const sessionItems = filtered.filter((i) => i.group === "sessions");

    if (actionItems.length > 0)
      groups.push({ key: "actions", label: "Actions", items: actionItems });
    if (commandItems.length > 0)
      groups.push({ key: "commands", label: "Commands", items: commandItems });
    if (sessionItems.length > 0)
      groups.push({ key: "sessions", label: "Sessions", items: sessionItems });

    return groups;
  }, [filtered]);

  // Flat list for keyboard navigation
  const flatItems = useMemo(() => grouped.flatMap((g) => g.items), [grouped]);

  // Reset state when opening
  useEffect(() => {
    if (open) {
      setQuery("");
      setActiveIndex(0);
      // Focus input after animation frame
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  // Clamp activeIndex when filtered list changes
  useEffect(() => {
    setActiveIndex((prev) => Math.min(prev, Math.max(0, flatItems.length - 1)));
  }, [flatItems.length]);

  // Scroll active item into view
  useEffect(() => {
    if (!listRef.current) return;
    const activeEl = listRef.current.querySelector("[data-slot='cmd-item'][data-active]");
    if (activeEl) {
      activeEl.scrollIntoView({ block: "nearest" });
    }
  }, []);

  // Handle selection
  const handleSelect = useCallback(
    (item: CommandItem) => {
      onOpenChange(false);
      // Defer to allow dialog close animation
      requestAnimationFrame(() => item.onSelect());
    },
    [onOpenChange],
  );

  // Keyboard navigation
  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (flatItems.length === 0) return;

      if (e.key === "ArrowDown") {
        e.preventDefault();
        setActiveIndex((prev) => (prev + 1) % flatItems.length);
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setActiveIndex((prev) => (prev - 1 + flatItems.length) % flatItems.length);
      } else if (e.key === "Enter") {
        e.preventDefault();
        const item = flatItems[activeIndex];
        if (item) handleSelect(item);
      }
    },
    [flatItems, activeIndex, handleSelect],
  );

  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay data-component="command-palette-overlay" />
        <Dialog.Content
          data-component="command-palette"
          aria-describedby={undefined}
          aria-label="Command palette"
          onKeyDown={handleKeyDown}
        >
          <Dialog.Title className="sr-only">Command Palette</Dialog.Title>

          {/* Search input */}
          <div data-slot="cmd-search-wrapper">
            <Search data-slot="cmd-search-icon" className="w-4 h-4" />
            <input
              ref={inputRef}
              data-slot="cmd-search-input"
              type="text"
              placeholder="Search commands, sessions..."
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                setActiveIndex(0);
              }}
              autoComplete="off"
              spellCheck={false}
            />
          </div>

          {/* Results */}
          <div ref={listRef} data-slot="cmd-results">
            {flatItems.length === 0 ? (
              <div data-slot="cmd-empty">
                {query ? `No results for "${query}"` : "No commands available"}
              </div>
            ) : (
              grouped.map((group) => {
                // Calculate base index offset for this group
                let baseIndex = 0;
                for (const g of grouped) {
                  if (g.key === group.key) break;
                  baseIndex += g.items.length;
                }

                return (
                  <div key={group.key}>
                    <div data-slot="cmd-group-header">{group.label}</div>
                    {group.items.map((item, idx) => {
                      const globalIdx = baseIndex + idx;
                      return (
                        <button
                          key={item.id}
                          type="button"
                          data-slot="cmd-item"
                          data-active={globalIdx === activeIndex || undefined}
                          onMouseEnter={() => setActiveIndex(globalIdx)}
                          onMouseDown={(e) => {
                            e.preventDefault();
                            handleSelect(item);
                          }}
                        >
                          <span data-slot="cmd-item-icon">{item.icon}</span>
                          <span data-slot="cmd-item-content">
                            <span data-slot="cmd-item-label">{item.label}</span>
                            <span data-slot="cmd-item-desc">{item.description}</span>
                          </span>
                          {item.shortcut && <ShortcutKeys keys={item.shortcut} />}
                        </button>
                      );
                    })}
                  </div>
                );
              })
            )}
          </div>

          {/* Footer hints */}
          <div data-slot="cmd-footer">
            <span data-slot="cmd-footer-hint">
              <kbd data-slot="cmd-kbd">{"\u2191\u2193"}</kbd>
              navigate
            </span>
            <span data-slot="cmd-footer-hint">
              <kbd data-slot="cmd-kbd">{"\u23CE"}</kbd>
              select
            </span>
            <span data-slot="cmd-footer-hint">
              <kbd data-slot="cmd-kbd">esc</kbd>
              close
            </span>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}

export type { CommandPaletteProps };
