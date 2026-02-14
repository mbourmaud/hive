import * as Collapsible from "@radix-ui/react-collapsible";
import { useState } from "react";
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

export function DroneContent({ drones, connectionStatus }: DroneContentProps) {
  const [expandedDrone, setExpandedDrone] = useState<string | null>(null);

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
}: {
  drone: DroneInfo;
  isExpanded: boolean;
  onToggle: () => void;
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
            <span className="text-sm font-semibold truncate flex-1">{drone.name}</span>
            <span className="text-xs text-muted-foreground shrink-0">
              {done}/{total}
            </span>
            <span className="text-[11px] text-muted-foreground shrink-0">{drone.elapsed}</span>
          </div>
          <Progress value={pct} className="h-1 mt-2" />
        </button>
      </Collapsible.Trigger>

      <Collapsible.Content>
        <div data-slot="drone-panel-detail">
          <div className="flex items-center justify-between mb-3">
            <span className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">
              Cost
            </span>
            <span className="text-xs font-semibold text-accent">
              {fmtCost(drone.cost.total_usd)}
            </span>
          </div>

          {drone.tasks.length > 0 && (
            <div className="mb-3">
              <div className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide mb-1.5">
                Tasks
              </div>
              <TaskList tasks={drone.tasks} members={drone.members} />
            </div>
          )}

          {drone.members.length > 0 && (
            <div>
              <div className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide mb-1.5">
                Team
              </div>
              <TeamBar
                members={drone.members}
                leadModel={drone.lead_model}
                droneLiveness={drone.liveness}
              />
            </div>
          )}
        </div>
      </Collapsible.Content>
    </Collapsible.Root>
  );
}
