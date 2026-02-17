import type { DroneInfo } from "@/domains/monitor/types";
import { fmtCost } from "@/shared/constants";
import { cn } from "@/shared/lib/utils";
import { Progress } from "@/shared/ui/progress";

interface DroneItemProps {
  drone: DroneInfo;
  selected: boolean;
  onClick: () => void;
}

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

function phaseBadge(phase: string | null) {
  if (!phase) return null;
  const config: Record<string, { label: string; cls: string }> = {
    dispatch: { label: "Dispatch", cls: "bg-blue-500/15 text-blue-400" },
    monitor: { label: "Working", cls: "bg-honey/15 text-honey" },
    verify: { label: "Verify", cls: "bg-purple-500/15 text-purple-400" },
    pr: { label: "PR", cls: "bg-green-500/15 text-green-400" },
    complete: { label: "Done", cls: "bg-success/15 text-success" },
    failed: { label: "Failed", cls: "bg-destructive/15 text-destructive" },
  };
  const c = config[phase];
  if (!c) return null;
  return (
    <span className={cn("text-[10px] font-semibold px-1.5 py-0.5 rounded", c.cls)}>
      {c.label}
    </span>
  );
}

export function DroneItem({ drone, selected, onClick }: DroneItemProps) {
  const [done, total] = drone.progress;
  const pct = total > 0 ? Math.round((done / total) * 100) : 0;

  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        "w-full text-left flex items-center gap-3 px-4 py-3 min-h-[72px] cursor-pointer transition-colors duration-150 border-l-3",
        selected
          ? "bg-muted border-l-accent"
          : "border-l-accent/0 hover:border-l-accent/20 hover:bg-muted/50",
      )}
    >
      <div className="relative shrink-0">
        <div className={cn("w-2.5 h-2.5 rounded-full", livenessColor(drone.liveness))} />
        {drone.liveness === "working" && (
          <div className="absolute inset-0 rounded-full bg-honey animate-[pulse-ring_2s_ease-out_infinite]" />
        )}
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-semibold text-foreground truncate">{drone.name}</span>
          {phaseBadge(drone.phase)}
        </div>
        <div className="flex items-center gap-3 mt-1">
          <span className="text-xs text-muted-foreground">
            {done}/{total} ({pct}%)
          </span>
          <span className="text-xs text-muted-foreground">{drone.elapsed}</span>
        </div>
        <Progress value={pct} className="h-1 mt-1.5" />
      </div>
      <span className="text-xs font-medium text-muted-foreground shrink-0">
        {fmtCost(drone.cost.total_usd)}
      </span>
    </button>
  );
}
