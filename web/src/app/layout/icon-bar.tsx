import { Plus, Settings } from "lucide-react";
import beeIcon from "@/assets/bee-icon.png";
import type { ProjectInfo } from "@/domains/monitor/types";
import type { ProjectProfile } from "@/domains/projects/types";
import { ThemeToggle } from "@/shared/theme/theme-toggle";
import { THEMES } from "@/shared/theme/use-theme";
import { Avatar, AvatarFallback, AvatarImage } from "@/shared/ui/avatar";
import "./icon-bar.css";

interface IconBarProps {
  projects: ProjectInfo[];
  registryProjects: ProjectProfile[];
  activeProject: string | null;
  onSelectProject: (path: string) => void;
  onAddProject?: () => void;
  onOpenSettings?: () => void;
}

function getAccentForTheme(colorTheme: string | null): string | undefined {
  if (!colorTheme) return undefined;
  const theme = THEMES.find((t) => t.name === colorTheme);
  return theme?.accent;
}

export function IconBar({
  projects,
  registryProjects,
  activeProject,
  onSelectProject,
  onAddProject,
  onOpenSettings,
}: IconBarProps) {
  // Prefer registry data for enriched profiles, fall back to monitor projects
  const displayProjects =
    registryProjects.length > 0
      ? registryProjects
      : projects.map((p) => ({
          id: p.path,
          name: p.name,
          path: p.path,
          color_theme: null,
          image_url: null,
        }));

  return (
    <nav data-component="icon-bar">
      {/* Bee logo */}
      <div className="mb-2">
        <img src={beeIcon} alt="Hive" className="w-7 h-7" />
      </div>

      {/* Project icons */}
      {displayProjects.map((project) => {
        const initial = project.name.charAt(0).toUpperCase();
        const isActive = activeProject === project.path;
        const accent = getAccentForTheme(project.color_theme);

        return (
          <button
            key={project.path}
            type="button"
            data-slot="icon-bar-item"
            data-active={isActive || undefined}
            onClick={() => onSelectProject(project.path)}
            title={project.name}
            style={
              isActive && accent
                ? ({ "--project-accent": accent } as React.CSSProperties)
                : undefined
            }
          >
            <Avatar className="h-[28px] w-[28px] rounded-lg">
              {project.image_url && <AvatarImage src={project.image_url} alt={project.name} />}
              <AvatarFallback className="rounded-lg bg-transparent text-inherit text-[13px] font-bold">
                {initial}
              </AvatarFallback>
            </Avatar>
          </button>
        );
      })}

      {/* Add project button */}
      {onAddProject && (
        <button type="button" data-slot="icon-bar-add" onClick={onAddProject} title="Add project">
          <Plus className="h-4 w-4" />
        </button>
      )}

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
