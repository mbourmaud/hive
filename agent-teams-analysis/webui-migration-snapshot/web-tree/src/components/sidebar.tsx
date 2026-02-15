import type { DroneInfo } from "@/types/api";
import { ThemeToggle } from "./theme-toggle";
import { DroneItem } from "./drone-item";
import { fmtCost } from "./constants";

interface SidebarProps {
  drones: DroneInfo[];
  selectedDrone: string | null;
  onSelectDrone: (name: string) => void;
  connectionStatus: "connected" | "disconnected" | "mock";
}

function sseIndicator(status: "connected" | "disconnected" | "mock") {
  const colors: Record<string, string> = {
    connected: "var(--green)",
    mock: "var(--yellow)",
    disconnected: "var(--red)",
  };
  const labels: Record<string, string> = {
    connected: "Live",
    mock: "Mock",
    disconnected: "Reconnecting...",
  };
  return { color: colors[status] || "var(--red)", label: labels[status] || "Unknown" };
}

export function Sidebar({ drones, selectedDrone, onSelectDrone, connectionStatus }: SidebarProps) {
  const activeCount = drones.filter((d) => d.liveness === "working").length;
  const totalCost = drones.reduce((sum, d) => sum + (d.cost?.total_usd || 0), 0);
  const sse = sseIndicator(connectionStatus);

  return (
    <div
      className="w-[300px] flex flex-col shrink-0 transition-colors"
      style={{
        background: "var(--surface)",
        borderRight: "1px solid var(--border)",
      }}
    >
      {/* Header */}
      <div className="p-5" style={{ borderBottom: "1px solid var(--border)" }}>
        <div className="flex items-center justify-between mb-3">
          <h1
            className="text-xl font-extrabold tracking-[3px]"
            style={{ color: "var(--accent)" }}
          >
            HIVE
          </h1>
          <ThemeToggle />
        </div>
        <div className="flex items-center gap-4 text-xs" style={{ color: "var(--text-muted)" }}>
          <span>
            Active: <strong style={{ color: "var(--text)" }}>{activeCount}</strong>
          </span>
          <span>
            Total: <strong style={{ color: "var(--text)" }}>{drones.length}</strong>
          </span>
          <span>
            Cost: <strong style={{ color: "var(--text)" }}>{fmtCost(totalCost)}</strong>
          </span>
        </div>
        <div className="flex items-center gap-2 mt-2">
          <div
            className="w-2 h-2 rounded-full"
            style={{ background: sse.color }}
          />
          <span className="text-xs" style={{ color: "var(--text-muted)" }}>
            {sse.label}
          </span>
        </div>
      </div>

      {/* Drone list */}
      <div className="flex-1 overflow-y-auto">
        {drones.length === 0 ? (
          <div className="p-5 text-sm" style={{ color: "var(--text-muted)" }}>
            No drones detected.
          </div>
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
