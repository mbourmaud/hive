import * as Dialog from "@radix-ui/react-dialog";
import { Search } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { CommandItem } from "./use-command-items";
import { useCommandItems } from "./use-command-items";
import { fuzzyMatch, ShortcutKeys } from "./utils";
import "./command-palette.css";

// ── Types ────────────────────────────────────────────────────────────────────

export interface CommandPaletteProps {
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

  const allItems = useCommandItems({
    sessions,
    onSelectSession,
    onNewSession,
    onClearChat,
    onOpenSettings,
    onToggleTheme,
    onRunCommand,
  });

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

          <div ref={listRef} data-slot="cmd-results">
            {flatItems.length === 0 ? (
              <div data-slot="cmd-empty">
                {query ? `No results for "${query}"` : "No commands available"}
              </div>
            ) : (
              grouped.map((group) => {
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
