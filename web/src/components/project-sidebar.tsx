import type { ProjectInfo } from "@/types/api";
import { ThemeToggle } from "./theme-toggle";
import { fmtCost } from "./constants";
import { cn } from "@/lib/utils";
import beeIcon from "@/assets/bee-icon.png";

interface ProjectSidebarProps {
  projects: ProjectInfo[];
  selectedProject: string | null;
  onSelectProject: (path: string) => void;
}

export function ProjectSidebar({ projects, selectedProject, onSelectProject }: ProjectSidebarProps) {
  const hasSelection = selectedProject !== null;

  return (
    <div
      className={cn(
        "w-[200px] flex flex-col shrink-0 bg-sidebar border-r transition-colors duration-200",
        hasSelection ? "border-sidebar-border/40" : "border-sidebar-border"
      )}
    >
      {/* Header — h-14 aligned with other panels */}
      <div className="h-14 flex items-center gap-2.5 px-4 shrink-0 border-b border-sidebar-border">
        <img src={beeIcon} alt="Hive" className="w-7 h-7 shrink-0" />
        <span className="text-base font-black tracking-wider text-accent font-mono">
          HIVE
        </span>
        <div className="ml-auto">
          <ThemeToggle />
        </div>
      </div>

      {/* Project list */}
      <div className="flex-1 overflow-y-auto">
        {projects.map((project) => {
          const isSelected = selectedProject === project.path;
          const initial = project.name.charAt(0).toUpperCase();
          return (
            <button
              key={project.path}
              onClick={() => onSelectProject(project.path)}
              className={cn(
                "relative w-full text-left px-4 py-3 min-h-[72px] flex items-center gap-3 transition-all duration-200 cursor-pointer border-l-3",
                isSelected
                  ? "bg-accent/10 border-l-accent"
                  : "border-l-transparent hover:bg-muted/50"
              )}
            >
              <div className={cn(
                "w-8 h-8 rounded-md flex items-center justify-center text-sm font-bold shrink-0 transition-colors duration-200",
                isSelected
                  ? "bg-accent/20 text-accent"
                  : "bg-accent/10 text-accent/70"
              )}>
                {initial}
              </div>
              <div className="min-w-0 flex-1">
                <div className={cn(
                  "text-sm font-semibold truncate transition-colors duration-200",
                  isSelected ? "text-foreground" : "text-foreground/80"
                )}>{project.name}</div>
                <div className="text-xs text-muted-foreground mt-0.5">
                  {project.active_count} active / {project.drones.length} drones
                </div>
              </div>
              {/* Right-edge glow bar for selected state */}
              {isSelected && (
                <div
                  className="absolute right-0 top-3 bottom-3 w-[3px] rounded-full bg-accent/50 shadow-[0_0_8px_var(--color-accent)]"
                  aria-hidden="true"
                />
              )}
            </button>
          );
        })}
      </div>

      {/* Footer */}
      <div className="px-4 py-3 border-t border-sidebar-border">
        <div className="text-xs text-muted-foreground">
          Total: <strong className="text-foreground">{fmtCost(projects.reduce((s, p) => s + p.total_cost, 0))}</strong>
        </div>
      </div>
    </div>
  );
}
