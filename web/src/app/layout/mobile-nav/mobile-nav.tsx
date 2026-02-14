import { useEffect, useState } from "react";
import type { ProjectInfo } from "@/domains/monitor/types";
import { ThemeToggle } from "@/shared/theme/theme-toggle";
import { MobileDetailView } from "./detail-view";
import { MobileDroneList, MobileProjectList } from "./lists";

type Screen = "projects" | "drones" | "detail";

interface MobileNavProps {
  projects: ProjectInfo[];
  selectedProject: string | null;
  onSelectProject: (path: string) => void;
  selectedDrone: string | null;
  onSelectDrone: (name: string) => void;
  connectionStatus: "connected" | "disconnected" | "mock";
  isMock: boolean;
}

function SseIndicatorMobile({ status }: { status: string }) {
  const isLive = status === "connected";
  const isMockStatus = status === "mock";
  return (
    <div
      className={`w-2 h-2 rounded-full shrink-0 ${isLive ? "bg-success" : isMockStatus ? "bg-warning" : "bg-destructive"}`}
    />
  );
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

  const title =
    screen === "projects"
      ? "HIVE"
      : screen === "drones"
        ? (activeProject?.name ?? "HIVE")
        : (activeDrone?.name ?? "HIVE");

  const showBack = screen === "detail" || (screen === "drones" && !isSingleProject);

  return (
    <div className="flex flex-col h-[100dvh] bg-background">
      {/* Fixed header */}
      <div className="flex items-center gap-3 px-4 py-3 shrink-0 bg-card border-b border-border">
        {showBack && (
          <button
            type="button"
            onClick={handleBack}
            className="text-muted-foreground hover:text-foreground p-1 -ml-1"
            aria-label="Go back"
          >
            <svg
              width="20"
              height="20"
              viewBox="0 0 20 20"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
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
          <MobileDetailView drone={activeDrone} isMock={isMock} projectPath={activeProject?.path} />
        )}
        {screen === "detail" && !activeDrone && (
          <div className="p-6 text-center text-muted-foreground">Select a drone</div>
        )}
      </div>
    </div>
  );
}
