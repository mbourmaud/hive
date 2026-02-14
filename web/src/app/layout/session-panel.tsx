import * as ContextMenu from "@radix-ui/react-context-menu";
import { GripVertical, Pencil, Plus, Trash2 } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import beeIcon from "@/assets/bee-icon.png";
import { useResizablePanel } from "@/shared/hooks/use-resizable-panel";
import "./session-panel.css";

// ── Constants ────────────────────────────────────────────────────────────────

const PANEL_MIN = 220;
const PANEL_MAX = 400;
const PANEL_DEFAULT = 260;
const COLLAPSE_THRESHOLD = 160;

// ── Types ────────────────────────────────────────────────────────────────────

export interface SessionEntry {
  id: string;
  title: string;
  createdAt: string;
  status: "idle" | "busy" | "completed" | "error";
  cwd: string;
}

interface SessionPanelProps {
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

// ── Session item with context menu ──────────────────────────────────────────

function SessionItem({
  session,
  isActive,
  onSelect,
  onRename,
  onDelete,
}: {
  session: SessionEntry;
  isActive: boolean;
  onSelect: () => void;
  onRename?: (title: string) => void;
  onDelete?: () => void;
}) {
  const [renaming, setRenaming] = useState(false);
  const [renameValue, setRenameValue] = useState(session.title);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (renaming) {
      inputRef.current?.focus();
      inputRef.current?.select();
    }
  }, [renaming]);

  const handleRenameSubmit = useCallback(() => {
    const trimmed = renameValue.trim();
    if (trimmed && trimmed !== session.title) {
      onRename?.(trimmed);
    }
    setRenaming(false);
  }, [renameValue, session.title, onRename]);

  const handleDeleteConfirm = useCallback(() => {
    if (window.confirm("Delete this session? This cannot be undone.")) {
      onDelete?.();
    }
  }, [onDelete]);

  const content = (
    <button
      type="button"
      onClick={onSelect}
      data-slot="sidebar-session-item"
      data-active={isActive || undefined}
    >
      <div className="flex items-center gap-2 min-w-0">
        <div
          className={`w-1.5 h-1.5 rounded-full shrink-0 ${
            session.status === "busy"
              ? "bg-accent"
              : session.status === "idle"
                ? "bg-success"
                : session.status === "error"
                  ? "bg-destructive"
                  : "bg-muted-foreground"
          }`}
        />
        {renaming ? (
          <input
            ref={inputRef}
            type="text"
            value={renameValue}
            onChange={(e) => setRenameValue(e.target.value)}
            onBlur={handleRenameSubmit}
            onKeyDown={(e) => {
              if (e.key === "Enter") handleRenameSubmit();
              if (e.key === "Escape") {
                setRenameValue(session.title);
                setRenaming(false);
              }
            }}
            onClick={(e) => e.stopPropagation()}
            className="flex-1 min-w-0 bg-transparent text-foreground text-sm outline-none border-b border-accent"
          />
        ) : (
          <span className="truncate">{session.title}</span>
        )}
      </div>
    </button>
  );

  if (!onRename && !onDelete) return content;

  return (
    <ContextMenu.Root>
      <ContextMenu.Trigger asChild>{content}</ContextMenu.Trigger>
      <ContextMenu.Portal>
        <ContextMenu.Content data-component="sidebar-context-menu">
          {onRename && (
            <ContextMenu.Item
              data-slot="sidebar-context-item"
              onSelect={() => {
                setRenameValue(session.title);
                setRenaming(true);
              }}
            >
              <Pencil className="h-3 w-3" />
              Rename
            </ContextMenu.Item>
          )}
          {onDelete && (
            <ContextMenu.Item
              data-slot="sidebar-context-item"
              data-destructive
              onSelect={handleDeleteConfirm}
            >
              <Trash2 className="h-3 w-3" />
              Delete
            </ContextMenu.Item>
          )}
        </ContextMenu.Content>
      </ContextMenu.Portal>
    </ContextMenu.Root>
  );
}

// ── Session Panel ────────────────────────────────────────────────────────────

export function SessionPanel({
  sessions,
  activeSessionId,
  onSelectSession,
  onNewSession,
  onRenameSession,
  onDeleteSession,
}: SessionPanelProps) {
  const { width, collapsed, onMouseDown } = useResizablePanel({
    minWidth: PANEL_MIN,
    maxWidth: PANEL_MAX,
    defaultWidth: PANEL_DEFAULT,
    collapseThreshold: COLLAPSE_THRESHOLD,
    side: "left",
  });

  const grouped = useMemo(() => groupSessionsByDate(sessions), [sessions]);

  if (collapsed) {
    return null;
  }

  return (
    <>
      <nav data-component="session-panel" style={{ width: `${width}px` }}>
        {/* Header */}
        <div data-slot="session-panel-header">
          <img src={beeIcon} alt="Hive" className="w-5 h-5 shrink-0" />
          <span className="text-sm font-extrabold tracking-wider text-accent font-mono">HIVE</span>
        </div>

        {/* Session list */}
        <div className="flex-1 overflow-y-auto">
          {/* New session button */}
          <button
            type="button"
            onClick={onNewSession}
            className="mx-3 mt-3 mb-2 w-[calc(100%-24px)] inline-flex items-center gap-2 rounded-lg border border-dashed border-border px-3 py-2 text-sm text-muted-foreground hover:text-foreground hover:border-foreground/20 transition-colors"
          >
            <Plus className="h-3.5 w-3.5" />
            New session
          </button>

          {sessions.length === 0 ? (
            <div className="px-3 py-4 text-xs text-muted-foreground">
              No sessions yet. Start a new one.
            </div>
          ) : (
            <div className="flex flex-col px-1.5">
              {grouped.map(({ group, items }) => (
                <div key={group}>
                  <div className="px-2.5 pt-3 pb-1 text-[11px] font-medium text-muted-foreground uppercase tracking-wider">
                    {group}
                  </div>
                  <div className="flex flex-col gap-0.5">
                    {items.map((session) => (
                      <SessionItem
                        key={session.id}
                        session={session}
                        isActive={activeSessionId === session.id}
                        onSelect={() => onSelectSession(session.id)}
                        onRename={
                          onRenameSession
                            ? (title) => onRenameSession(session.id, title)
                            : undefined
                        }
                        onDelete={onDeleteSession ? () => onDeleteSession(session.id) : undefined}
                      />
                    ))}
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </nav>

      {/* Drag handle */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: resize drag handle */}
      <div data-slot="session-panel-drag-handle" onMouseDown={onMouseDown}>
        <div className="absolute inset-y-0 -left-0.5 -right-0.5 flex items-center justify-center group">
          <GripVertical className="h-4 w-4 text-border opacity-0 group-hover:opacity-60 transition-opacity" />
        </div>
      </div>
    </>
  );
}
