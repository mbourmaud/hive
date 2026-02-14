import { FileEdit, GitBranch } from "lucide-react";
import { DiDocker, DiNodejsSmall, DiPython, DiRust } from "react-icons/di";
import { SiBitbucket, SiGithub, SiGitlab, SiGo } from "react-icons/si";
import type { ProjectContext } from "@/domains/projects/types";
import "./context-bar.css";

// ── Runtime config ──────────────────────────────────────────────────────────

interface RuntimeStyle {
  label: string;
  icon: React.ComponentType<{ className?: string }>;
  color: string;
  bg: string;
}

const RUNTIME_STYLES: Record<string, RuntimeStyle> = {
  node: {
    label: "Node",
    icon: DiNodejsSmall,
    color: "oklch(0.72 0.19 142)",
    bg: "oklch(0.72 0.19 142 / 12%)",
  },
  rust: {
    label: "Rust",
    icon: DiRust,
    color: "oklch(0.7 0.12 40)",
    bg: "oklch(0.7 0.12 40 / 12%)",
  },
  python: {
    label: "Python",
    icon: DiPython,
    color: "oklch(0.72 0.14 230)",
    bg: "oklch(0.72 0.14 230 / 12%)",
  },
  go: {
    label: "Go",
    icon: SiGo,
    color: "oklch(0.72 0.14 200)",
    bg: "oklch(0.72 0.14 200 / 12%)",
  },
  docker: {
    label: "Docker",
    icon: DiDocker,
    color: "oklch(0.68 0.15 240)",
    bg: "oklch(0.68 0.15 240 / 12%)",
  },
};

// ── Platform icons (MR/PR) ──────────────────────────────────────────────────

interface PlatformStyle {
  icon: React.ComponentType<{ className?: string }>;
  color: string;
  bg: string;
}

const PLATFORM_STYLES: Record<string, PlatformStyle> = {
  gitlab: {
    icon: SiGitlab,
    color: "oklch(0.72 0.16 45)",
    bg: "oklch(0.72 0.16 45 / 12%)",
  },
  github: {
    icon: SiGithub,
    color: "var(--color-foreground)",
    bg: "var(--color-muted)",
  },
  bitbucket: {
    icon: SiBitbucket,
    color: "oklch(0.65 0.15 250)",
    bg: "oklch(0.65 0.15 250 / 12%)",
  },
};

// ── Component ───────────────────────────────────────────────────────────────

interface ContextBarProps {
  context: ProjectContext;
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
          <span data-slot="context-bar-branch">{git.branch}</span>
          {git.ahead > 0 && (
            <span data-slot="context-bar-sync" data-variant="ahead">
              ↑{git.ahead}
            </span>
          )}
          {git.behind > 0 && (
            <span data-slot="context-bar-sync" data-variant="behind">
              ↓{git.behind}
            </span>
          )}
        </div>
      )}

      {/* Runtime pills */}
      {context.runtimes.map((rt) => {
        const style = RUNTIME_STYLES[rt.name];
        if (!style) {
          return (
            <span key={rt.name} data-slot="context-bar-runtime-fallback">
              {rt.name}
              {rt.version ? ` ${rt.version}` : ""}
            </span>
          );
        }
        const Icon = style.icon;
        return (
          <span
            key={rt.name}
            data-slot="context-bar-runtime"
            style={
              {
                "--rt-color": style.color,
                "--rt-bg": style.bg,
              } as React.CSSProperties
            }
          >
            <Icon className="h-3.5 w-3.5" />
            {style.label}
            {rt.version ? ` ${rt.version}` : ""}
          </span>
        );
      })}

      {/* PR/MR link */}
      {context.open_pr && git && (
        <PrButton
          platform={git.platform}
          label={prLabel}
          number={context.open_pr.number}
          isDraft={context.open_pr.is_draft}
          url={context.open_pr.url}
        />
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

// ── PR/MR Button ────────────────────────────────────────────────────────────

function PrButton({
  platform,
  label,
  number,
  isDraft,
  url,
}: {
  platform: string;
  label: string;
  number: number;
  isDraft: boolean;
  url: string;
}) {
  const style = PLATFORM_STYLES[platform];
  const Icon = style?.icon;

  return (
    <button
      type="button"
      data-slot="context-bar-pr"
      style={
        style
          ? ({
              "--pr-color": style.color,
              "--pr-bg": style.bg,
            } as React.CSSProperties)
          : undefined
      }
      onClick={() => window.open(url, "_blank")}
    >
      {Icon && <Icon className="h-3 w-3" />}
      <span>
        {label} #{number}
      </span>
      {isDraft && <span data-slot="context-bar-draft">draft</span>}
    </button>
  );
}
