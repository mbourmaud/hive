import { FileEdit, GitBranch } from "lucide-react";
import type { ProjectContext } from "@/domains/projects/types";
import { Badge } from "@/shared/ui/badge";
import "./context-bar.css";

interface ContextBarProps {
  context: ProjectContext;
  projectName: string;
}

export function ContextBar({ context }: ContextBarProps) {
  const git = context.git;
  const prLabel = git?.platform === "gitlab" ? "MR" : "PR";

  return (
    <div data-component="context-bar">
      {/* Git branch */}
      {git && (
        <div data-slot="context-bar-git">
          <GitBranch className="h-3.5 w-3.5" />
          <span className="max-w-[24ch] truncate">{git.branch}</span>
          {git.ahead > 0 && (
            <Badge variant="success" className="px-1.5 py-0 text-[10px] leading-[18px]">
              ↑{git.ahead}
            </Badge>
          )}
          {git.behind > 0 && (
            <Badge variant="warning" className="px-1.5 py-0 text-[10px] leading-[18px]">
              ↓{git.behind}
            </Badge>
          )}
        </div>
      )}

      {/* Runtime pills */}
      {context.runtimes.length > 0 && (
        <div data-slot="context-bar-runtimes">
          {context.runtimes.map((rt) => (
            <Badge
              key={rt.name}
              variant="secondary"
              className="px-1.5 py-0 text-[10px] leading-[18px]"
            >
              {rt.name.charAt(0).toUpperCase() + rt.name.slice(1)}
              {rt.version ? ` ${rt.version}` : ""}
            </Badge>
          ))}
        </div>
      )}

      {/* PR/MR link */}
      {context.open_pr && (
        <button
          type="button"
          data-slot="context-bar-pr"
          onClick={() => window.open(context.open_pr?.url, "_blank")}
        >
          <Badge variant="outline" className="px-1.5 py-0 text-[10px] leading-[18px]">
            {prLabel} #{context.open_pr.number}
            {context.open_pr.is_draft ? " (draft)" : ""}
          </Badge>
        </button>
      )}

      {/* Dirty indicator */}
      {git && git.dirty_count > 0 && (
        <div data-slot="context-bar-dirty">
          <FileEdit className="h-3 w-3" />
          <span>{git.dirty_count} changed</span>
        </div>
      )}
    </div>
  );
}
