import * as Dialog from "@radix-ui/react-dialog";
import { MessageSquare, Search } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import "./sessions-modal.css";

// ── Types ────────────────────────────────────────────────────────────────────

export interface SessionEntry {
  id: string;
  title: string;
  createdAt: string;
  status: "idle" | "busy" | "completed" | "error";
  cwd: string;
}

interface SessionsModalProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  sessions: SessionEntry[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
  onNewSession: () => void;
  onRenameSession?: (id: string, title: string) => void;
  onDeleteSession?: (id: string) => void;
}

// ── Date grouping ────────────────────────────────────────────────────────────

type DateGroup = "Today" | "Yesterday" | "This Week" | "Older";

interface GroupedSessions {
  group: DateGroup;
  items: SessionEntry[];
}

function groupSessionsByDate(sessions: SessionEntry[]): GroupedSessions[] {
  const now = new Date();
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const yesterday = new Date(today.getTime() - 86_400_000);
  const weekAgo = new Date(today.getTime() - 7 * 86_400_000);

  const groups: Record<DateGroup, SessionEntry[]> = {
    Today: [],
    Yesterday: [],
    "This Week": [],
    Older: [],
  };

  for (const session of sessions) {
    const date = new Date(session.createdAt);
    if (date >= today) {
      groups.Today.push(session);
    } else if (date >= yesterday) {
      groups.Yesterday.push(session);
    } else if (date >= weekAgo) {
      groups["This Week"].push(session);
    } else {
      groups.Older.push(session);
    }
  }

  const result: GroupedSessions[] = [];
  const order: DateGroup[] = ["Today", "Yesterday", "This Week", "Older"];
  for (const group of order) {
    if (groups[group].length > 0) {
      result.push({ group, items: groups[group] });
    }
  }

  return result;
}

// ── Fuzzy match ──────────────────────────────────────────────────────────────

function fuzzyMatch(query: string, text: string): boolean {
  if (!query) return true;
  return text.toLowerCase().includes(query.toLowerCase());
}

// ── Relative time ────────────────────────────────────────────────────────────

function relativeTime(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60_000);

  if (diffMins < 1) return "just now";
  if (diffMins < 60) return `${diffMins}m ago`;

  const diffHours = Math.floor(diffMins / 60);
  if (diffHours < 24) return `${diffHours}h ago`;

  const diffDays = Math.floor(diffHours / 24);
  if (diffDays < 7) return `${diffDays}d ago`;

  return date.toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

// ── Component ────────────────────────────────────────────────────────────────

export function SessionsModal({
  open,
  onOpenChange,
  sessions,
  activeSessionId,
  onSelectSession,
  onNewSession,
}: SessionsModalProps) {
  const [query, setQuery] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  // Sessions are already ordered most recent first from the API
  const filtered = useMemo(() => {
    if (!query) return sessions;
    return sessions.filter((s) => fuzzyMatch(query, s.title) || fuzzyMatch(query, s.cwd));
  }, [sessions, query]);

  const grouped = useMemo(() => groupSessionsByDate(filtered), [filtered]);

  // Flat list for keyboard navigation
  const flatItems = useMemo(() => grouped.flatMap((g) => g.items), [grouped]);

  // Reset state on open
  useEffect(() => {
    if (open) {
      setQuery("");
      setActiveIndex(0);
      requestAnimationFrame(() => inputRef.current?.focus());
    }
  }, [open]);

  // Clamp active index
  useEffect(() => {
    setActiveIndex((prev) => Math.min(prev, Math.max(0, flatItems.length - 1)));
  }, [flatItems.length]);

  // Scroll active item into view
  useEffect(() => {
    if (!listRef.current) return;
    const activeEl = listRef.current.querySelector("[data-slot='sessions-item'][data-active]");
    if (activeEl) {
      activeEl.scrollIntoView({ block: "nearest" });
    }
  }, [activeIndex]);

  const handleSelect = useCallback(
    (session: SessionEntry) => {
      onOpenChange(false);
      requestAnimationFrame(() => onSelectSession(session.id));
    },
    [onOpenChange, onSelectSession],
  );

  const handleNewAndClose = useCallback(() => {
    onOpenChange(false);
    requestAnimationFrame(() => onNewSession());
  }, [onOpenChange, onNewSession]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        if (flatItems.length > 0) {
          setActiveIndex((prev) => (prev + 1) % flatItems.length);
        }
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        if (flatItems.length > 0) {
          setActiveIndex((prev) => (prev - 1 + flatItems.length) % flatItems.length);
        }
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
        <Dialog.Overlay data-component="sessions-modal-overlay" />
        <Dialog.Content
          data-component="sessions-modal"
          aria-describedby={undefined}
          aria-label="Sessions"
          onKeyDown={handleKeyDown}
        >
          <Dialog.Title className="sr-only">Sessions</Dialog.Title>

          {/* Search input */}
          <div data-slot="sessions-search-wrapper">
            <Search data-slot="sessions-search-icon" className="w-4 h-4" />
            <input
              ref={inputRef}
              data-slot="sessions-search-input"
              type="text"
              placeholder="Search sessions..."
              value={query}
              onChange={(e) => {
                setQuery(e.target.value);
                setActiveIndex(0);
              }}
              autoComplete="off"
              spellCheck={false}
            />
          </div>

          {/* Session list */}
          <div ref={listRef} data-slot="sessions-list">
            {flatItems.length === 0 ? (
              <div data-slot="sessions-empty">
                {query ? (
                  `No sessions matching "${query}"`
                ) : (
                  <button
                    type="button"
                    className="text-accent hover:underline"
                    onClick={handleNewAndClose}
                  >
                    Start a new session
                  </button>
                )}
              </div>
            ) : (
              grouped.map((group) => {
                let baseIndex = 0;
                for (const g of grouped) {
                  if (g.group === group.group) break;
                  baseIndex += g.items.length;
                }

                return (
                  <div key={group.group}>
                    <div data-slot="sessions-group-header">{group.group}</div>
                    {group.items.map((session, idx) => {
                      const globalIdx = baseIndex + idx;
                      const isCurrent = session.id === activeSessionId;
                      return (
                        <button
                          key={session.id}
                          type="button"
                          data-slot="sessions-item"
                          data-active={globalIdx === activeIndex || undefined}
                          data-current={isCurrent || undefined}
                          onMouseEnter={() => setActiveIndex(globalIdx)}
                          onMouseDown={(e) => {
                            e.preventDefault();
                            handleSelect(session);
                          }}
                        >
                          <MessageSquare className="w-4 h-4 shrink-0 text-muted-foreground" />
                          <span data-slot="sessions-item-content">
                            <span data-slot="sessions-item-title">{session.title}</span>
                            <span data-slot="sessions-item-meta">
                              {relativeTime(session.createdAt)}
                              {isCurrent ? " — current" : ""}
                            </span>
                          </span>
                          <span data-slot="sessions-item-dot" data-status={session.status} />
                        </button>
                      );
                    })}
                  </div>
                );
              })
            )}
          </div>

          {/* Footer */}
          <div data-slot="sessions-footer">
            <span data-slot="sessions-footer-hint">
              <kbd data-slot="sessions-kbd">{"\u2191\u2193"}</kbd>
              navigate
            </span>
            <span data-slot="sessions-footer-hint">
              <kbd data-slot="sessions-kbd">{"\u23CE"}</kbd>
              select
            </span>
            <span data-slot="sessions-footer-hint">
              <kbd data-slot="sessions-kbd">esc</kbd>
              close
            </span>
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
