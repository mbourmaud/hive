import type { DroneInfo } from "@/types/api";
import { Progress } from "@/components/ui/progress";
import { fmtCost } from "./constants";
import { cn } from "@/lib/utils";

interface DroneItemProps {
  drone: DroneInfo;
  selected: boolean;
  onClick: () => void;
}

function livenessColor(liveness: string) {
  switch (liveness) {
    case "working": return "bg-honey";
    case "idle": return "bg-muted-foreground";
    case "completed": return "bg-success";
    case "dead": return "bg-destructive";
    case "stopped": return "bg-muted-foreground";
    default: return "bg-muted-foreground";
  }
}

export function DroneItem({ drone, selected, onClick }: DroneItemProps) {
  const [done, total] = drone.progress;
  const pct = total > 0 ? Math.round((done / total) * 100) : 0;

  return (
    <div
      onClick={onClick}
      className={cn(
        "flex items-center gap-3 px-4 py-3 min-h-[72px] cursor-pointer transition-colors duration-150 border-l-3",
        selected
          ? "bg-muted border-l-accent"
          : "border-l-accent/0 hover:border-l-accent/20 hover:bg-muted/50"
      )}
    >
      <div className="relative shrink-0">
        <div className={cn("w-2.5 h-2.5 rounded-full", livenessColor(drone.liveness))} />
        {drone.liveness === "working" && (
          <div className="absolute inset-0 rounded-full bg-honey animate-[pulse-ring_2s_ease-out_infinite]" />
        )}
      </div>
      <div className="flex-1 min-w-0">
        <div className="text-sm font-semibold text-foreground truncate">{drone.name}</div>
        <div className="flex items-center gap-3 mt-1">
          <span className="text-xs text-muted-foreground">{done}/{total} ({pct}%)</span>
          <span className="text-xs text-muted-foreground">{drone.elapsed}</span>
        </div>
        <Progress value={pct} className="h-1 mt-1.5" />
      </div>
      <span className="text-xs font-medium text-muted-foreground shrink-0">
        {fmtCost(drone.cost.total_usd)}
      </span>
    </div>
  );
}
