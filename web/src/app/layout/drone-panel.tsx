import * as Collapsible from "@radix-ui/react-collapsible";
import { GripVertical, PanelRightClose, PanelRightOpen } from "lucide-react";
import { useState } from "react";
import { TaskList } from "@/domains/monitor/components/task-list";
import { TeamBar } from "@/domains/monitor/components/team-bar";
import type { DroneInfo } from "@/domains/monitor/types";
import { fmtCost } from "@/shared/constants";
import { useResizablePanel } from "@/shared/hooks/use-resizable-panel";
import { cn } from "@/shared/lib/utils";
import { Progress } from "@/shared/ui/progress";
import "./drone-panel.css";

// ── Constants ────────────────────────────────────────────────────────────────

const PANEL_MIN = 260;
const PANEL_MAX = 420;
const PANEL_DEFAULT = 300;
const COLLAPSE_THRESHOLD = 200;

// ── Types ────────────────────────────────────────────────────────────────────

interface DronePanelProps {
  drones: DroneInfo[];
  connectionStatus: "connected" | "disconnected" | "mock";
  collapsed: boolean;
  onToggleCollapse: () => void;
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

// ── Drone Panel ─────────────────────────────────────────────────────────────

export function DronePanel({
  drones,
  connectionStatus,
  collapsed: externalCollapsed,
  onToggleCollapse,
}: DronePanelProps) {
  const {
    width,
    collapsed: resizeCollapsed,
    onMouseDown,
  } = useResizablePanel({
    minWidth: PANEL_MIN,
    maxWidth: PANEL_MAX,
    defaultWidth: PANEL_DEFAULT,
    collapseThreshold: COLLAPSE_THRESHOLD,
    side: "right",
  });

  const [expandedDrone, setExpandedDrone] = useState<string | null>(null);

  const isCollapsed = externalCollapsed || resizeCollapsed;

  if (isCollapsed) {
    return (
      <div className="flex flex-col items-center py-3 shrink-0 border-l border-sidebar-border bg-sidebar w-10">
        <button
          type="button"
          onClick={onToggleCollapse}
          className="h-8 w-8 flex items-center justify-center rounded-md text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
          title="Show drone panel"
        >
          <PanelRightOpen className="h-4 w-4" />
        </button>
      </div>
    );
  }

  return (
    <>
      {/* Drag handle */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: resize drag handle */}
      <div data-slot="drone-panel-drag-handle" onMouseDown={onMouseDown}>
        <div className="absolute inset-y-0 -left-0.5 -right-0.5 flex items-center justify-center group">
          <GripVertical className="h-4 w-4 text-border opacity-0 group-hover:opacity-60 transition-opacity" />
        </div>
      </div>

      <aside data-component="drone-panel" style={{ width: `${width}px` }}>
        {/* Header */}
        <div data-slot="drone-panel-header">
          <span className="text-sm font-semibold text-foreground">Drones</span>
          {drones.length > 0 && (
            <span className="inline-flex items-center justify-center h-5 min-w-[20px] px-1.5 rounded-full bg-accent/15 text-accent text-[11px] font-bold">
              {drones.length}
            </span>
          )}
          <button
            type="button"
            onClick={onToggleCollapse}
            className="ml-auto h-7 w-7 flex items-center justify-center rounded-md text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
            title="Hide drone panel"
          >
            <PanelRightClose className="h-3.5 w-3.5" />
          </button>
        </div>

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
      </aside>
    </>
  );
}

// ── Drone list item with expandable detail ──────────────────────────────────

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
            {/* Status dot */}
            <div className="relative shrink-0">
              <div className={cn("w-2 h-2 rounded-full", livenessColor(drone.liveness))} />
              {drone.liveness === "working" && (
                <div className="absolute inset-0 rounded-full bg-honey animate-[pulse-ring_2s_ease-out_infinite]" />
              )}
            </div>

            {/* Name */}
            <span className="text-sm font-semibold truncate flex-1">{drone.name}</span>

            {/* Progress fraction */}
            <span className="text-xs text-muted-foreground shrink-0">
              {done}/{total}
            </span>

            {/* Elapsed */}
            <span className="text-[11px] text-muted-foreground shrink-0">{drone.elapsed}</span>
          </div>

          {/* Progress bar */}
          <Progress value={pct} className="h-1 mt-2" />
        </button>
      </Collapsible.Trigger>

      <Collapsible.Content>
        <div data-slot="drone-panel-detail">
          {/* Cost */}
          <div className="flex items-center justify-between mb-3">
            <span className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide">
              Cost
            </span>
            <span className="text-xs font-semibold text-accent">
              {fmtCost(drone.cost.total_usd)}
            </span>
          </div>

          {/* Tasks */}
          {drone.tasks.length > 0 && (
            <div className="mb-3">
              <div className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide mb-1.5">
                Tasks
              </div>
              <TaskList tasks={drone.tasks} members={drone.members} />
            </div>
          )}

          {/* Team */}
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
