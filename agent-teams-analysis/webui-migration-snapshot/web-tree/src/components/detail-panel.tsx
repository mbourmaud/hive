import type { DroneInfo } from "@/types/api";
import { Badge } from "@/components/ui/badge";
import { TaskList } from "./task-list";
import { TeamBar } from "./team-bar";
import { ChatMessages } from "./chat-messages";
import { CostCard } from "./cost-card";
import { LogViewer } from "./log-viewer";
import { fmtCost } from "./constants";
import { useLogs } from "@/hooks/use-logs";

interface DetailPanelProps {
  drone: DroneInfo | null;
  isMock: boolean;
}

function statusVariant(liveness: string) {
  switch (liveness) {
    case "working": return "success" as const;
    case "completed": return "default" as const;
    case "dead": return "destructive" as const;
    case "stopped": return "secondary" as const;
    default: return "outline" as const;
  }
}

function SectionTitle({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex items-center gap-3 mb-3">
      <span
        className="text-xs font-semibold uppercase tracking-wider"
        style={{ color: "var(--text-muted)" }}
      >
        {children}
      </span>
      <div className="flex-1 h-px" style={{ background: "var(--border)" }} />
    </div>
  );
}

export function DetailPanel({ drone, isMock }: DetailPanelProps) {
  const { logs } = useLogs(drone?.name ?? null, isMock);

  if (!drone) {
    return (
      <div
        className="flex-1 flex items-center justify-center"
        style={{ background: "var(--bg)" }}
      >
        <span className="text-lg" style={{ color: "var(--text-muted)" }}>
          Select a drone from the sidebar
        </span>
      </div>
    );
  }

  const [done, total] = drone.progress;
  const pct = total > 0 ? Math.round((done / total) * 100) : 0;

  return (
    <div className="flex-1 flex flex-col overflow-hidden" style={{ background: "var(--bg)" }}>
      {/* Header */}
      <div
        className="flex items-center gap-3 px-6 py-4 shrink-0"
        style={{ borderBottom: "1px solid var(--border)" }}
      >
        <h2 className="text-lg font-bold" style={{ color: "var(--text)" }}>
          {drone.name}
        </h2>
        <Badge variant={statusVariant(drone.liveness)}>{drone.status}</Badge>
        <Badge variant="outline">{drone.lead_model || "?"}</Badge>
        <Badge variant="outline">{drone.branch}</Badge>
        <Badge variant="outline" style={{ color: "var(--accent)" }}>
          {fmtCost(drone.cost.total_usd)}
        </Badge>
        <span className="ml-auto text-sm" style={{ color: "var(--text-muted)" }}>
          {drone.elapsed}
        </span>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto px-6 py-4">
        <div className="flex flex-col gap-6 max-w-4xl">
          {/* Progress */}
          <div>
            <div className="flex justify-between text-sm mb-1.5" style={{ color: "var(--text-muted)" }}>
              <span>Tasks: {done}/{total}</span>
              <span>{pct}%</span>
            </div>
            <div className="h-2 rounded-full overflow-hidden" style={{ background: "var(--surface2)" }}>
              <div
                className="h-full rounded-full transition-all"
                style={{ width: `${pct}%`, background: "var(--accent)" }}
              />
            </div>
          </div>

          {/* Tasks */}
          <div>
            <SectionTitle>Tasks</SectionTitle>
            <TaskList tasks={drone.tasks} members={drone.members} />
          </div>

          {/* Team */}
          {drone.members.length > 0 && (
            <div>
              <SectionTitle>Team</SectionTitle>
              <TeamBar
                members={drone.members}
                leadModel={drone.lead_model}
                droneLiveness={drone.liveness}
              />
            </div>
          )}

          {/* Messages */}
          <div>
            <SectionTitle>Messages</SectionTitle>
            <ChatMessages
              messages={drone.messages}
              members={drone.members}
              leadModel={drone.lead_model}
            />
          </div>

          {/* Cost */}
          <div>
            <SectionTitle>Cost</SectionTitle>
            <CostCard cost={drone.cost} />
          </div>

          {/* Logs */}
          <div>
            <SectionTitle>Live Logs</SectionTitle>
            <LogViewer logs={logs} />
          </div>
        </div>
      </div>
    </div>
  );
}
