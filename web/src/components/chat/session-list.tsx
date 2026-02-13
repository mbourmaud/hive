import { useState, useCallback, useEffect, useMemo, useRef } from "react";
import { Trash2 } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  groupSessionsByDate,
  relativeTime,
  type SessionMeta,
  type DateGroup,
} from "@/hooks/use-sessions";

// ── Types ────────────────────────────────────────────────────────────────────

interface SessionListProps {
  sessions: SessionMeta[];
  activeSessionId: string | null;
  onSelectSession: (id: string) => void;
  onDeleteSession: (id: string) => void;
}

// ── Status dot color ─────────────────────────────────────────────────────────

function statusDotClass(status: SessionMeta["status"]): string {
  switch (status) {
    case "busy":
      return "bg-accent";
    case "idle":
      return "bg-success";
    case "error":
      return "bg-destructive";
    default:
      return "bg-muted-foreground";
  }
}

// ── Group label ──────────────────────────────────────────────────────────────

function GroupLabel({ group }: { group: DateGroup }) {
  return (
    <div className="px-2.5 pt-3 pb-1">
      <span className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
        {group}
      </span>
    </div>
  );
}

// ── Session item ─────────────────────────────────────────────────────────────

interface SessionItemProps {
  session: SessionMeta;
  isActive: boolean;
  onSelect: () => void;
  onDelete: () => void;
}

function SessionItem({ session, isActive, onSelect, onDelete }: SessionItemProps) {
  const [hovered, setHovered] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const confirmTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (confirmTimeoutRef.current) clearTimeout(confirmTimeoutRef.current);
    };
  }, []);

  const handleDelete = useCallback(
    (e: React.MouseEvent) => {
      e.stopPropagation();
      if (confirmDelete) {
        onDelete();
        setConfirmDelete(false);
      } else {
        setConfirmDelete(true);
        if (confirmTimeoutRef.current) clearTimeout(confirmTimeoutRef.current);
        confirmTimeoutRef.current = setTimeout(() => setConfirmDelete(false), 3000);
      }
    },
    [confirmDelete, onDelete],
  );

  return (
    <button
      type="button"
      onClick={onSelect}
      onMouseEnter={() => setHovered(true)}
      onMouseLeave={() => {
        setHovered(false);
        setConfirmDelete(false);
      }}
      className={cn(
        "w-full text-left rounded-md px-2.5 py-2 text-sm transition-colors group",
        isActive
          ? "bg-accent/10 text-foreground"
          : "text-muted-foreground hover:bg-muted hover:text-foreground",
      )}
    >
      <div className="flex items-center gap-2 min-w-0">
        <div
          className={cn(
            "w-1.5 h-1.5 rounded-full shrink-0",
            statusDotClass(session.status),
          )}
        />
        <span className="flex-1 truncate">{session.title}</span>

        {hovered && (
          <button
            type="button"
            onClick={handleDelete}
            className={cn(
              "shrink-0 p-0.5 rounded transition-colors",
              confirmDelete
                ? "text-destructive hover:bg-destructive/10"
                : "text-muted-foreground hover:text-foreground",
            )}
            title={confirmDelete ? "Click again to confirm" : "Delete session"}
          >
            <Trash2 className="h-3 w-3" />
          </button>
        )}
      </div>

      <div className="flex items-center gap-1 mt-0.5 ml-3.5">
        <span className="text-[11px] text-muted-foreground/60">
          {relativeTime(session.updated_at)}
        </span>
      </div>
    </button>
  );
}

// ── Component ────────────────────────────────────────────────────────────────

export function SessionList({
  sessions,
  activeSessionId,
  onSelectSession,
  onDeleteSession,
}: SessionListProps) {
  const grouped = useMemo(() => groupSessionsByDate(sessions), [sessions]);

  if (sessions.length === 0) {
    return (
      <div className="px-3 py-4 text-xs text-muted-foreground">
        No sessions yet. Start a new one.
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-0.5 px-1.5">
      {grouped.map(({ group, items }) => (
        <div key={group}>
          <GroupLabel group={group} />
          {items.map((session) => (
            <SessionItem
              key={session.id}
              session={session}
              isActive={activeSessionId === session.id}
              onSelect={() => onSelectSession(session.id)}
              onDelete={() => onDeleteSession(session.id)}
            />
          ))}
        </div>
      ))}
    </div>
  );
}
