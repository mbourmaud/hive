import type { DroneInfo } from "@/types/api";
import { ThemeToggle } from "./theme-toggle";
import { DroneItem } from "./drone-item";
import { cn } from "@/lib/utils";
import beeIcon from "@/assets/bee-icon.png";

interface SidebarProps {
  drones: DroneInfo[];
  selectedDrone: string | null;
  onSelectDrone: (name: string) => void;
  connectionStatus: "connected" | "disconnected" | "mock";
  projectName?: string;
  showBranding?: boolean;
}

function SseIndicator({ status }: { status: string }) {
  const isLive = status === "connected";
  const isMock = status === "mock";
  return (
    <div className="flex items-center gap-1.5">
      <div className={`w-1.5 h-1.5 rounded-full ${isLive ? "bg-success" : isMock ? "bg-warning" : "bg-destructive"}`} />
      <span className="text-[11px] text-muted-foreground">
        {isLive ? "Live" : isMock ? "Mock" : "Reconnecting..."}
      </span>
    </div>
  );
}

export function Sidebar({ drones, selectedDrone, onSelectDrone, connectionStatus, projectName, showBranding }: SidebarProps) {
  const hasProject = !showBranding && !!projectName;

  return (
    <div className={cn(
      "w-[300px] flex flex-col shrink-0 bg-sidebar border-r border-sidebar-border",
      hasProject && "border-l border-l-accent/15"
    )}>
      {/* Header — h-14 aligned with other panels */}
      <div className={cn(
        "h-14 flex items-center gap-3 px-4 shrink-0 border-b border-sidebar-border transition-colors duration-200",
        hasProject && ""
      )}>
        {showBranding ? (
          <>
            <img src={beeIcon} alt="Hive" className="w-7 h-7 shrink-0" />
            <span className="text-base font-black tracking-wider text-accent font-mono">
              HIVE
            </span>
          </>
        ) : (
          <div className="flex items-center gap-2 min-w-0">
            <div className="w-1.5 h-1.5 rounded-full bg-accent shrink-0" />
            <span className="text-sm font-semibold text-foreground truncate">{projectName || "Drones"}</span>
            <span className="text-xs text-muted-foreground shrink-0">
              ({drones.length})
            </span>
          </div>
        )}
        <div className="ml-auto flex items-center gap-3">
          <SseIndicator status={connectionStatus} />
          {showBranding && <ThemeToggle />}
        </div>
      </div>

      {/* Accent connection line at the top of drone list */}
      {hasProject && (
        <div className="h-px bg-gradient-to-r from-accent/20 via-accent/8 to-transparent" aria-hidden="true" />
      )}

      {/* Drone list */}
      <div className="flex-1 overflow-y-auto">
        {drones.length === 0 ? (
          <div className="p-5 text-sm text-muted-foreground">No drones detected.</div>
        ) : (
          drones.map((drone) => (
            <DroneItem
              key={drone.name}
              drone={drone}
              selected={selectedDrone === drone.name}
              onClick={() => onSelectDrone(drone.name)}
            />
          ))
        )}
      </div>
    </div>
  );
}
