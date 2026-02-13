import { useState, useEffect } from "react";
import type { ProjectInfo, DroneInfo } from "@/types/api";
import { ThemeToggle } from "./theme-toggle";
import { DroneItem } from "./drone-item";
import { TaskList } from "./task-list";
import { TeamBar } from "./team-bar";
import { ChatMessages } from "./chat-messages";
import { LogViewer } from "./log-viewer";
import { Badge } from "@/components/ui/badge";
import { fmtCost } from "./constants";
import { useLogs } from "@/hooks/use-logs";
import { ListChecks, Users, MessageSquare, Terminal } from "lucide-react";
import { cn } from "@/lib/utils";

type Screen = "projects" | "drones" | "detail";
type DetailTab = "tasks" | "team" | "chat" | "logs";

interface MobileNavProps {
  projects: ProjectInfo[];
  selectedProject: string | null;
  onSelectProject: (path: string) => void;
  selectedDrone: string | null;
  onSelectDrone: (name: string) => void;
  connectionStatus: "connected" | "disconnected" | "mock";
  isMock: boolean;
}

export function MobileNav({
  projects,
  selectedProject,
  onSelectProject,
  selectedDrone,
  onSelectDrone,
  connectionStatus,
  isMock,
}: MobileNavProps) {
  const isSingleProject = projects.length <= 1;
  const [screen, setScreen] = useState<Screen>(isSingleProject ? "drones" : "projects");

  useEffect(() => {
    if (isSingleProject && screen === "projects") {
      setScreen("drones");
    }
  }, [isSingleProject, screen]);

  const activeProject = projects.find((p) => p.path === selectedProject) ?? projects[0] ?? null;
  const drones = activeProject?.drones ?? [];
  const activeDrone = drones.find((d) => d.name === selectedDrone) ?? null;

  const handleSelectProject = (path: string) => {
    onSelectProject(path);
    setScreen("drones");
  };

  const handleSelectDrone = (name: string) => {
    onSelectDrone(name);
    setScreen("detail");
  };

  const handleBack = () => {
    if (screen === "detail") {
      setScreen("drones");
    } else if (screen === "drones" && !isSingleProject) {
      setScreen("projects");
    }
  };

  const title = screen === "projects"
    ? "HIVE"
    : screen === "drones"
      ? activeProject?.name ?? "HIVE"
      : activeDrone?.name ?? "HIVE";

  const showBack = screen === "detail" || (screen === "drones" && !isSingleProject);

  return (
    <div className="flex flex-col h-[100dvh] bg-background">
      {/* Fixed header */}
      <div className="flex items-center gap-3 px-4 py-3 shrink-0 bg-card border-b border-border">
        {showBack && (
          <button
            onClick={handleBack}
            className="text-muted-foreground hover:text-foreground p-1 -ml-1"
          >
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M13 4L7 10L13 16" />
            </svg>
          </button>
        )}
        <h1 className="text-lg font-extrabold tracking-[2px] text-accent flex-1 truncate">
          {title}
        </h1>
        <SseIndicatorMobile status={connectionStatus} />
        <ThemeToggle />
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto">
        {screen === "projects" && (
          <MobileProjectList
            projects={projects}
            selectedProject={selectedProject}
            onSelectProject={handleSelectProject}
          />
        )}
        {screen === "drones" && (
          <MobileDroneList
            drones={drones}
            selectedDrone={selectedDrone}
            onSelectDrone={handleSelectDrone}
          />
        )}
        {screen === "detail" && activeDrone && (
          <MobileDetailView
            drone={activeDrone}
            isMock={isMock}
            projectPath={activeProject?.path}
          />
        )}
        {screen === "detail" && !activeDrone && (
          <div className="p-6 text-center text-muted-foreground">
            Select a drone
          </div>
        )}
      </div>
    </div>
  );
}

function SseIndicatorMobile({ status }: { status: string }) {
  const isLive = status === "connected";
  const isMock = status === "mock";
  return (
    <div className={`w-2 h-2 rounded-full shrink-0 ${isLive ? "bg-success" : isMock ? "bg-warning" : "bg-destructive"}`} />
  );
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

function formatStatus(status: string) {
  return status.replace(/_/g, " ").replace(/inprogress/i, "in progress");
}

const TAB_CONFIG: { key: DetailTab; label: string; icon: typeof ListChecks }[] = [
  { key: "tasks", label: "Tasks", icon: ListChecks },
  { key: "team", label: "Team", icon: Users },
  { key: "chat", label: "Chat", icon: MessageSquare },
  { key: "logs", label: "Logs", icon: Terminal },
];

function MobileDetailView({ drone, isMock, projectPath }: { drone: DroneInfo; isMock: boolean; projectPath?: string }) {
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
          <span className="ml-auto text-xs font-medium text-accent">{fmtCost(drone.cost.total_usd)}</span>
        </div>
        <div className="flex items-center gap-3">
          <div className="flex-1 h-2 rounded-full overflow-hidden bg-muted">
            <div className="h-full rounded-full transition-all bg-success" style={{ width: `${pct}%` }} />
          </div>
          <span className="text-xs font-semibold text-success shrink-0">{done}/{total}</span>
        </div>
      </div>

      {/* Tab content */}
      <div className="flex-1 overflow-y-auto p-4 animate-fade-in">
        {tab === "tasks" && (
          <TaskList tasks={drone.tasks} members={drone.members} />
        )}
        {tab === "team" && (
          drone.members.length > 0 ? (
            <TeamBar members={drone.members} leadModel={drone.lead_model} droneLiveness={drone.liveness} />
          ) : (
            <div className="text-sm text-muted-foreground py-3">No team members</div>
          )
        )}
        {tab === "chat" && (
          <ChatMessages messages={drone.messages} members={drone.members} leadModel={drone.lead_model} />
        )}
        {tab === "logs" && (
          <LogViewer logs={logs} raw={rawLogs} onToggleRaw={() => setRawLogs(r => !r)} />
        )}
      </div>

      {/* Bottom tab bar */}
      <div className="shrink-0 border-t border-border bg-card safe-area-pb">
        <div className="flex">
          {TAB_CONFIG.map(({ key, label, icon: Icon }) => (
            <button
              key={key}
              onClick={() => setTab(key)}
              className={cn(
                "flex-1 flex flex-col items-center gap-1 py-2.5 text-[11px] font-medium transition-colors",
                tab === key
                  ? "text-accent"
                  : "text-muted-foreground"
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

function MobileProjectList({
  projects,
  selectedProject,
  onSelectProject,
}: {
  projects: ProjectInfo[];
  selectedProject: string | null;
  onSelectProject: (path: string) => void;
}) {
  return (
    <div className="divide-y divide-border">
      {projects.map((project) => {
        const initial = project.name.charAt(0).toUpperCase();
        const isSelected = selectedProject === project.path;
        return (
          <button
            key={project.path}
            onClick={() => onSelectProject(project.path)}
            className={`w-full text-left px-4 py-4 flex items-center gap-4 transition-colors
              ${isSelected ? "bg-accent/10" : "hover:bg-muted/50"}`}
          >
            <div className="w-10 h-10 rounded-lg bg-accent/15 text-accent flex items-center justify-center text-lg font-bold shrink-0">
              {initial}
            </div>
            <div className="flex-1 min-w-0">
              <div className="text-base font-medium text-foreground">{project.name}</div>
              <div className="text-sm text-muted-foreground">
                {project.active_count} active / {project.drones.length} drones
              </div>
            </div>
            <div className="text-sm text-muted-foreground shrink-0">
              {fmtCost(project.total_cost)}
            </div>
            <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2" className="text-muted-foreground shrink-0">
              <path d="M6 4L10 8L6 12" />
            </svg>
          </button>
        );
      })}
    </div>
  );
}

function MobileDroneList({
  drones,
  selectedDrone,
  onSelectDrone,
}: {
  drones: DroneInfo[];
  selectedDrone: string | null;
  onSelectDrone: (name: string) => void;
}) {
  if (drones.length === 0) {
    return (
      <div className="p-6 text-center text-muted-foreground">
        No drones detected.
      </div>
    );
  }

  return (
    <div>
      {drones.map((drone) => (
        <div key={drone.name} onClick={() => onSelectDrone(drone.name)} className="cursor-pointer">
          <DroneItem
            drone={drone}
            selected={selectedDrone === drone.name}
            onClick={() => onSelectDrone(drone.name)}
          />
        </div>
      ))}
    </div>
  );
}
