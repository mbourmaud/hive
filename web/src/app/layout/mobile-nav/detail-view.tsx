import { ListChecks, MessageSquare, Terminal, Users } from "lucide-react";
import { useState } from "react";
import { ChatMessages } from "@/domains/monitor/components/chat-messages";
import { LogViewer } from "@/domains/monitor/components/log-viewer";
import { TaskList } from "@/domains/monitor/components/task-list";
import { TeamBar } from "@/domains/monitor/components/team-bar";
import { useLogs } from "@/domains/monitor/hooks/use-logs";
import type { DroneInfo } from "@/domains/monitor/types";
import { fmtCost } from "@/shared/constants";
import { cn } from "@/shared/lib/utils";
import { Badge } from "@/shared/ui/badge";

type DetailTab = "tasks" | "team" | "chat" | "logs";

const TAB_CONFIG: { key: DetailTab; label: string; icon: typeof ListChecks }[] = [
  { key: "tasks", label: "Tasks", icon: ListChecks },
  { key: "team", label: "Team", icon: Users },
  { key: "chat", label: "Chat", icon: MessageSquare },
  { key: "logs", label: "Logs", icon: Terminal },
];

function statusVariant(liveness: string) {
  switch (liveness) {
    case "working":
      return "honey" as const;
    case "completed":
      return "success" as const;
    case "dead":
      return "destructive" as const;
    case "stopped":
      return "secondary" as const;
    default:
      return "outline" as const;
  }
}

function formatStatus(status: string) {
  return status.replace(/_/g, " ").replace(/inprogress/i, "in progress");
}

export function MobileDetailView({
  drone,
  isMock,
  projectPath,
}: {
  drone: DroneInfo;
  isMock: boolean;
  projectPath?: string;
}) {
  const [tab, setTab] = useState<DetailTab>("tasks");
  const [rawLogs, setRawLogs] = useState(false);
  const { logs } = useLogs(drone.name, isMock, projectPath, rawLogs);
  const [done, total] = drone.progress;
  const pct = total > 0 ? Math.round((done / total) * 100) : 0;

  return (
    <div className="flex flex-col h-full">
      {/* Progress summary bar */}
      <div className="px-4 py-3 bg-card border-b border-border">
        <div className="flex items-center gap-3 mb-2">
          <Badge variant={statusVariant(drone.liveness)}>{formatStatus(drone.status)}</Badge>
          <span className="text-xs text-muted-foreground">{drone.elapsed}</span>
          <span className="ml-auto text-xs font-medium text-accent">
            {fmtCost(drone.cost.total_usd)}
          </span>
        </div>
        <div className="flex items-center gap-3">
          <div className="flex-1 h-2 rounded-full overflow-hidden bg-muted">
            <div
              className="h-full rounded-full transition-all bg-success"
              style={{ width: `${pct}%` }}
            />
          </div>
          <span className="text-xs font-semibold text-success shrink-0">
            {done}/{total}
          </span>
        </div>
      </div>

      {/* Tab content */}
      <div className="flex-1 overflow-y-auto p-4 animate-fade-in">
        {tab === "tasks" && <TaskList tasks={drone.tasks} members={drone.members} />}
        {tab === "team" &&
          (drone.members.length > 0 ? (
            <TeamBar
              members={drone.members}
              leadModel={drone.lead_model}
              droneLiveness={drone.liveness}
            />
          ) : (
            <div className="text-sm text-muted-foreground py-3">No team members</div>
          ))}
        {tab === "chat" && (
          <ChatMessages
            messages={drone.messages}
            members={drone.members}
            leadModel={drone.lead_model}
          />
        )}
        {tab === "logs" && (
          <LogViewer logs={logs} raw={rawLogs} onToggleRaw={() => setRawLogs((r) => !r)} />
        )}
      </div>

      {/* Bottom tab bar */}
      <div className="shrink-0 border-t border-border bg-card safe-area-pb">
        <div className="flex">
          {TAB_CONFIG.map(({ key, label, icon: Icon }) => (
            <button
              type="button"
              key={key}
              onClick={() => setTab(key)}
              className={cn(
                "flex-1 flex flex-col items-center gap-1 py-2.5 text-[11px] font-medium transition-colors",
                tab === key ? "text-accent" : "text-muted-foreground",
              )}
            >
              <Icon className="w-5 h-5" />
              {label}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
