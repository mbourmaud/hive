import type { DroneInfo } from "@/types/api";
import { Progress } from "@/components/ui/progress";
import { fmtCost } from "./constants";

interface DroneItemProps {
  drone: DroneInfo;
  selected: boolean;
  onClick: () => void;
}

function livenessColor(liveness: string) {
  switch (liveness) {
    case "working": return "var(--green)";
    case "idle": return "var(--yellow)";
    case "completed": return "var(--accent)";
    case "dead": return "var(--red)";
    default: return "var(--text-muted)";
  }
}

function livenessShadow(liveness: string) {
  if (liveness === "working") return "0 0 6px rgba(34, 197, 94, 0.5)";
  return "none";
}

export function DroneItem({ drone, selected, onClick }: DroneItemProps) {
  const [done, total] = drone.progress;
  const pct = total > 0 ? Math.round((done / total) * 100) : 0;

  return (
    <div
      onClick={onClick}
      className="flex items-center gap-3 px-4 py-3 cursor-pointer transition-colors"
      style={{
        background: selected ? "var(--surface2)" : "transparent",
        borderLeft: selected ? "3px solid var(--accent)" : "3px solid transparent",
      }}
      onMouseEnter={(e) => {
        if (!selected) e.currentTarget.style.background = "var(--surface2)";
      }}
      onMouseLeave={(e) => {
        if (!selected) e.currentTarget.style.background = "transparent";
      }}
    >
      {/* Status dot */}
      <div
        className="w-2.5 h-2.5 rounded-full shrink-0"
        style={{
          background: livenessColor(drone.liveness),
          boxShadow: livenessShadow(drone.liveness),
        }}
      />

      {/* Info */}
      <div className="flex-1 min-w-0">
        <div className="text-sm font-semibold truncate" style={{ color: "var(--text)" }}>
          {drone.name}
        </div>
        <div className="flex items-center gap-3 mt-1">
          <span className="text-xs" style={{ color: "var(--text-muted)" }}>
            {done}/{total} ({pct}%)
          </span>
          <span className="text-xs" style={{ color: "var(--text-muted)" }}>
            {drone.elapsed}
          </span>
        </div>
        <Progress value={pct} className="h-1 mt-1.5" />
      </div>

      {/* Cost */}
      <span className="text-xs font-medium shrink-0" style={{ color: "var(--text-muted)" }}>
        {fmtCost(drone.cost.total_usd)}
      </span>
    </div>
  );
}
