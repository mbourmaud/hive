import { Settings } from "lucide-react";
import beeIcon from "@/assets/bee-icon.png";
import type { ProjectInfo } from "@/domains/monitor/types";
import { ThemeToggle } from "@/shared/theme/theme-toggle";
import "./icon-bar.css";

interface IconBarProps {
  projects: ProjectInfo[];
  activeProject: string | null;
  onSelectProject: (path: string) => void;
  onOpenSettings?: () => void;
}

export function IconBar({
  projects,
  activeProject,
  onSelectProject,
  onOpenSettings,
}: IconBarProps) {
  return (
    <nav data-component="icon-bar">
      {/* Bee logo */}
      <div className="mb-2">
        <img src={beeIcon} alt="Hive" className="w-7 h-7" />
      </div>

      {/* Project icons */}
      {projects.map((project) => {
        const initial = project.name.charAt(0).toUpperCase();
        const isActive = activeProject === project.path;
        return (
          <button
            key={project.path}
            type="button"
            data-slot="icon-bar-item"
            data-active={isActive || undefined}
            onClick={() => onSelectProject(project.path)}
            title={project.name}
          >
            {initial}
          </button>
        );
      })}

      {/* Footer */}
      <div data-slot="icon-bar-footer">
        {onOpenSettings && (
          <button
            type="button"
            data-slot="icon-bar-footer-btn"
            onClick={onOpenSettings}
            title="Settings"
          >
            <Settings className="h-4 w-4" />
          </button>
        )}
        <ThemeToggle />
      </div>
    </nav>
  );
}
