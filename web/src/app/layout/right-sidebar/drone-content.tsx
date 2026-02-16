import * as Collapsible from "@radix-ui/react-collapsible";
import { Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { TaskList } from "@/domains/monitor/components/task-list";
import { TeamBar } from "@/domains/monitor/components/team-bar";
import type { DroneInfo } from "@/domains/monitor/types";
import { fmtCost } from "@/shared/constants";
import { cn } from "@/shared/lib/utils";
import { Progress } from "@/shared/ui/progress";

// ── Types ────────────────────────────────────────────────────────────────────

interface DroneContentProps {
  drones: DroneInfo[];
  connectionStatus: "connected" | "disconnected" | "mock";
}

// ── SSE indicator ────────────────────────────────────────────────────────────

function SseIndicator({ status }: { status: string }) {
  const info = (() => {
    switch (status) {
      case "connected":
        return { dotClass: "bg-success", label: "Live" };
      case "mock":
        return { dotClass: "bg-warning", label: "Mock" };
      default:
        return { dotClass: "bg-destructive", label: "Offline" };
    }
  })();

  return (
    <div className="flex items-center gap-1.5">
      <div className={cn("w-1.5 h-1.5 rounded-full", info.dotClass)} />
      <span className="text-[11px] text-muted-foreground">{info.label}</span>
    </div>
  );
}

// ── Liveness color ──────────────────────────────────────────────────────────

function livenessColor(liveness: string) {
  switch (liveness) {
    case "working":
      return "bg-honey";
    case "idle":
      return "bg-muted-foreground";
    case "completed":
      return "bg-success";
    case "dead":
      return "bg-destructive";
    case "stopped":
      return "bg-muted-foreground";
    default:
      return "bg-muted-foreground";
  }
}

// ── Drone Content ───────────────────────────────────────────────────────────

const NON_CLEANABLE_STATES = new Set(["working", "cleaning"]);

export function DroneContent({ drones, connectionStatus }: DroneContentProps) {
  const [expandedDrone, setExpandedDrone] = useState<string | null>(null);
  const [cleaning, setCleaning] = useState<string | null>(null);

  // Clear cleaning state once the drone disappears from the list or transitions to "cleaning"
  useEffect(() => {
    if (cleaning && !drones.some((d) => d.name === cleaning)) {
      setCleaning(null);
    }
  }, [cleaning, drones]);

  const handleClean = useCallback(async (name: string) => {
    setCleaning(name);
    try {
      await fetch(`/api/drones/${encodeURIComponent(name)}/clean`, { method: "POST" });
    } catch {
      /* drone disappears on next SSE poll */
    }
    // Don't clear cleaning here — wait for drone to disappear from SSE
  }, []);

  return (
    <>
      {/* Drone list */}
      <div className="flex-1 overflow-y-auto">
        {drones.length === 0 ? (
          <div className="p-4 text-sm text-muted-foreground">No drones detected.</div>
        ) : (
          drones.map((drone) => {
            const isExpanded = expandedDrone === drone.name;
            return (
              <DroneListItem
                key={drone.name}
                drone={drone}
                isExpanded={isExpanded}
                onToggle={() => setExpandedDrone(isExpanded ? null : drone.name)}
                canClean={!NON_CLEANABLE_STATES.has(drone.liveness)}
                isCleaning={cleaning === drone.name}
                onClean={() => handleClean(drone.name)}
              />
            );
          })
        )}
      </div>

      {/* Footer */}
      <div data-slot="drone-panel-footer">
        <SseIndicator status={connectionStatus} />
        {drones.length > 0 && (
          <span className="text-[11px] text-muted-foreground">
            {fmtCost(drones.reduce((s, d) => s + d.cost.total_usd, 0))}
          </span>
        )}
      </div>
    </>
  );
}

// ── Drone list item ─────────────────────────────────────────────────────────

function DroneListItem({
  drone,
  isExpanded,
  onToggle,
  canClean,
  isCleaning,
  onClean,
}: {
  drone: DroneInfo;
  isExpanded: boolean;
  onToggle: () => void;
  canClean: boolean;
  isCleaning: boolean;
  onClean: () => void;
}) {
  const [done, total] = drone.progress;
  const pct = total > 0 ? Math.round((done / total) * 100) : 0;

  return (
    <Collapsible.Root open={isExpanded} onOpenChange={() => onToggle()}>
      <Collapsible.Trigger asChild>
        <button type="button" data-slot="drone-panel-item" data-expanded={isExpanded || undefined}>
          <div className="flex items-center gap-2.5">
            <div className="relative shrink-0">
              <div className={cn("w-2 h-2 rounded-full", livenessColor(drone.liveness))} />
              {drone.liveness === "working" && (
                <div className="absolute inset-0 rounded-full bg-honey animate-[pulse-ring_2s_ease-out_infinite]" />
              )}
            </div>
            <span className="text-sm font-semibold truncate flex-1">
              {drone.title ?? drone.name}
            </span>
            <span className="text-xs text-muted-foreground shrink-0">
              {done}/{total}
            </span>
            <span className="text-[11px] text-muted-foreground shrink-0">{drone.elapsed}</span>
          </div>
          {drone.description && (
            <p className="text-[11px] text-muted-foreground leading-snug mt-1 line-clamp-2">
              {drone.description}
            </p>
          )}
          <Progress value={pct} className="h-1 mt-1.5" />
        </button>
      </Collapsible.Trigger>

      <Collapsible.Content>
        <div data-slot="drone-panel-detail">
          <div className="flex items-center justify-between mb-2">
            <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wide">
              Cost
            </span>
            <span className="text-xs font-semibold text-accent">
              {fmtCost(drone.cost.total_usd)}
            </span>
          </div>

          {drone.tasks.length > 0 && (
            <div className="mb-2">
              <div className="text-[10px] font-medium text-muted-foreground uppercase tracking-wide mb-1">
                Tasks
              </div>
              <TaskList tasks={drone.tasks} members={drone.members} />
            </div>
          )}

          {drone.members.length > 0 && (
            <div>
              <div className="text-[10px] font-medium text-muted-foreground uppercase tracking-wide mb-1">
                Team
              </div>
              <TeamBar
                members={drone.members}
                leadModel={drone.lead_model}
                droneLiveness={drone.liveness}
              />
            </div>
          )}

          {canClean && (
            <div className="flex justify-end pt-3 mt-3 border-t border-sidebar-border">
              <button
                type="button"
                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-destructive/10 text-destructive text-xs font-semibold hover:bg-destructive/20 transition-colors disabled:opacity-50"
                onClick={(e) => {
                  e.stopPropagation();
                  onClean();
                }}
                disabled={isCleaning}
              >
                <Trash2 className="h-3 w-3" />
                {isCleaning ? "Cleaning..." : "Clean"}
              </button>
            </div>
          )}
        </div>
      </Collapsible.Content>
    </Collapsible.Root>
  );
}
