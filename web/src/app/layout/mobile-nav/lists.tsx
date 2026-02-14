import { DroneItem } from "@/domains/monitor/components/drone-item";
import type { DroneInfo, ProjectInfo } from "@/domains/monitor/types";
import { fmtCost } from "@/shared/constants";

export function MobileProjectList({
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
            type="button"
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
            <svg
              width="16"
              height="16"
              viewBox="0 0 16 16"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              className="text-muted-foreground shrink-0"
              aria-hidden="true"
            >
              <path d="M6 4L10 8L6 12" />
            </svg>
          </button>
        );
      })}
    </div>
  );
}

export function MobileDroneList({
  drones,
  selectedDrone,
  onSelectDrone,
}: {
  drones: DroneInfo[];
  selectedDrone: string | null;
  onSelectDrone: (name: string) => void;
}) {
  if (drones.length === 0) {
    return <div className="p-6 text-center text-muted-foreground">No drones detected.</div>;
  }

  return (
    <div>
      {drones.map((drone) => (
        <DroneItem
          key={drone.name}
          drone={drone}
          selected={selectedDrone === drone.name}
          onClick={() => onSelectDrone(drone.name)}
        />
      ))}
    </div>
  );
}
