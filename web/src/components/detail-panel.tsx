import type { DroneInfo } from "@/types/api";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { TaskList } from "./task-list";
import { TeamBar } from "./team-bar";
import { ChatMessages } from "./chat-messages";
import { CostCard } from "./cost-card";
import { LogViewer } from "./log-viewer";
import { fmtCost } from "./constants";
import { useLogs } from "@/hooks/use-logs";
import { useState } from "react";
import { ListChecks, Users, MessageSquare, Terminal, Square, Trash2, RotateCcw } from "lucide-react";
import beeIcon from "@/assets/bee-icon.png";
import { Button } from "@/components/ui/button";
import { useActions } from "@/hooks/use-actions";

interface DetailPanelProps {
  drone: DroneInfo | null;
  isMock: boolean;
  projectPath?: string;
}

function formatStatus(status: string) {
  return status.replace(/_/g, " ").replace(/inprogress/i, "in progress");
}

function statusVariant(liveness: string) {
  switch (liveness) {
    case "working": return "honey" as const;
    case "completed": return "success" as const;
    case "dead": return "destructive" as const;
    case "stopped": return "secondary" as const;
    default: return "outline" as const;
  }
}

function modelColorClass(model: string | null): string {
  if (!model) return "border-border text-foreground";
  if (model.includes("opus")) return "border-transparent bg-model-opus text-white";
  if (model.includes("haiku")) return "border-transparent bg-model-haiku text-white";
  return "border-transparent bg-model-sonnet text-white";
}

function shortModelName(model: string | null): string {
  if (!model) return "?";
  if (model.includes("opus")) return "Opus";
  if (model.includes("haiku")) return "Haiku";
  if (model.includes("sonnet")) return "Sonnet";
  return model;
}

function GitBranchIcon() {
  return (
    <svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor" className="shrink-0 opacity-70">
      <path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.5 2.5 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25Zm-6 0a.75.75 0 1 0 1.5 0 .75.75 0 0 0-1.5 0Zm8.25-.75a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5ZM4.25 12a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Z" />
    </svg>
  );
}

function SegmentedProgress({ done, total }: { done: number; total: number }) {
  if (total <= 0) return null;

  if (total <= 15) {
    return (
      <div className="flex gap-1 h-2.5">
        {Array.from({ length: total }, (_, i) => (
          <div
            key={i}
            className={`flex-1 rounded-full transition-all ${i < done ? "bg-success" : "bg-muted"}`}
          />
        ))}
      </div>
    );
  }

  const pct = Math.round((done / total) * 100);
  return (
    <div className="h-2.5 rounded-full overflow-hidden bg-muted">
      <div
        className="h-full rounded-full transition-all bg-success"
        style={{ width: `${pct}%` }}
      />
    </div>
  );
}

export function DetailPanel({ drone, isMock, projectPath }: DetailPanelProps) {
  const [rawLogs, setRawLogs] = useState(false);
  const { logs } = useLogs(drone?.name ?? null, isMock, projectPath, rawLogs);
  const { isTauri, stopDrone, cleanDrone } = useActions();

  const handleStop = async () => {
    if (!drone) return;
    try {
      await stopDrone(drone.name);
    } catch (err) {
      console.error("Failed to stop drone:", err);
    }
  };

  const handleClean = async () => {
    if (!drone) return;
    if (!confirm(`Are you sure you want to clean drone "${drone.name}"? This will remove the worktree.`)) {
      return;
    }
    try {
      await cleanDrone(drone.name);
    } catch (err) {
      console.error("Failed to clean drone:", err);
    }
  };

  const handleRestart = () => {
    alert("Restart coming soon!");
  };

  if (!drone) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center bg-background gap-4">
        <img src={beeIcon} alt="Hive" className="w-16 h-16 opacity-30" />
        <div className="text-center">
          <p className="text-lg font-medium text-muted-foreground">No drone selected</p>
          <p className="text-sm text-muted-foreground/60 mt-1">Pick a drone from the sidebar to view details</p>
        </div>
      </div>
    );
  }

  const [done, total] = drone.progress;
  const pct = total > 0 ? Math.round((done / total) * 100) : 0;

  return (
    <div key={drone.name} className="flex-1 flex flex-col overflow-hidden bg-background animate-fade-in">
      {/* Header */}
      <div className="h-14 flex items-center gap-2 sm:gap-3 px-4 sm:px-8 shrink-0 bg-card border-b border-border">
        <h2 className="text-sm sm:text-base font-bold text-foreground truncate">{drone.name}</h2>
        <Badge variant={statusVariant(drone.liveness)}>{formatStatus(drone.status)}</Badge>
        <Badge variant="outline" className={`hidden sm:inline-flex ${modelColorClass(drone.lead_model)}`}>{shortModelName(drone.lead_model)}</Badge>
        <Badge variant="outline" className="hidden sm:inline-flex items-center gap-1.5 font-mono text-accent border-accent/30">
          <GitBranchIcon />
          {drone.branch}
        </Badge>
        <Badge variant="outline" className="hidden sm:inline-flex text-accent">{fmtCost(drone.cost.total_usd)}</Badge>
        {isTauri && (
          <div className="flex items-center gap-1">
            <Button variant="destructive" size="sm" onClick={handleStop} className="h-7 px-2 text-xs">
              <Square className="w-3 h-3 mr-1" />
              Stop
            </Button>
            <Button variant="outline" size="sm" onClick={handleClean} className="h-7 px-2 text-xs">
              <Trash2 className="w-3 h-3 mr-1" />
              Clean
            </Button>
            <Button variant="outline" size="sm" onClick={handleRestart} className="h-7 px-2 text-xs">
              <RotateCcw className="w-3 h-3 mr-1" />
              Restart
            </Button>
          </div>
        )}
        <span className="ml-auto text-xs sm:text-sm text-muted-foreground shrink-0">{drone.elapsed}</span>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-4 sm:p-8">
        <div className="flex flex-col gap-6 max-w-5xl">
          {/* Hero: Progress + Cost merged */}
          <Card className="border-t-2 border-t-accent/60">
            <CardHeader className="pb-3 py-4">
              <CardTitle className="text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
                Progress
              </CardTitle>
            </CardHeader>
            <Separator />
            <CardContent className="pt-4 space-y-3">
              <div className="flex justify-between text-sm mb-2 text-muted-foreground">
                <span>{done}/{total} tasks</span>
                <span className="font-semibold text-success">{pct}%</span>
              </div>
              <SegmentedProgress done={done} total={total} />
              <div className="pt-1">
                <CostCard cost={drone.cost} />
              </div>
            </CardContent>
          </Card>

          {/* Tasks */}
          <Card>
            <CardHeader className="pb-3 py-4">
              <CardTitle className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
                <ListChecks className="w-3.5 h-3.5" />
                Tasks
              </CardTitle>
            </CardHeader>
            <Separator />
            <CardContent className="pt-4">
              <TaskList tasks={drone.tasks} members={drone.members} />
            </CardContent>
          </Card>

          {/* Team */}
          {drone.members.length > 0 && (
            <Card>
              <CardHeader className="pb-3 py-4">
                <CardTitle className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
                  <Users className="w-3.5 h-3.5" />
                  Team
                </CardTitle>
              </CardHeader>
              <Separator />
              <CardContent className="pt-4">
                <TeamBar members={drone.members} leadModel={drone.lead_model} droneLiveness={drone.liveness} />
              </CardContent>
            </Card>
          )}

          {/* Messages */}
          <Card>
            <CardHeader className="pb-3 py-4">
              <CardTitle className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
                <MessageSquare className="w-3.5 h-3.5" />
                Messages
              </CardTitle>
            </CardHeader>
            <Separator />
            <CardContent className="pt-4">
              <ChatMessages messages={drone.messages} members={drone.members} leadModel={drone.lead_model} />
            </CardContent>
          </Card>

          {/* Logs */}
          <Card>
            <CardHeader className="pb-3 py-4">
              <CardTitle className="flex items-center gap-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground">
                <Terminal className="w-3.5 h-3.5" />
                Live Logs
              </CardTitle>
            </CardHeader>
            <Separator />
            <CardContent className="pt-4">
              <LogViewer logs={logs} raw={rawLogs} onToggleRaw={() => setRawLogs(r => !r)} />
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
