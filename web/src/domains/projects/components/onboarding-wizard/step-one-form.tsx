import { ChevronRight, FolderOpen, Loader2 } from "lucide-react";
import { cn } from "@/shared/lib/utils";
import { THEMES } from "@/shared/theme/use-theme";
import { Button } from "@/shared/ui/button";

interface StepOneFormProps {
  name: string;
  setName: (name: string) => void;
  path: string;
  setPath: (path: string) => void;
  selectedTheme: string;
  setSelectedTheme: (theme: string) => void;
  canContinue: boolean;
  isPending: boolean;
  error: Error | null;
  onPickFolder: () => void;
  onContinue: () => void;
  onCancel?: () => void;
}

export function StepOneForm({
  name,
  setName,
  path,
  setPath,
  selectedTheme,
  setSelectedTheme,
  canContinue,
  isPending,
  error,
  onPickFolder,
  onContinue,
  onCancel,
}: StepOneFormProps) {
  return (
    <div className="flex flex-col gap-5">
      {error && (
        <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
          {error.message}
        </div>
      )}

      <div className="flex flex-col gap-1.5">
        <label className="text-sm font-medium text-foreground" htmlFor="project-name">
          Name
        </label>
        <input
          id="project-name"
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="my-project"
          className={cn(
            "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground",
            "placeholder:text-muted-foreground",
            "focus:outline-none focus:ring-1 focus:ring-ring",
          )}
        />
      </div>

      <div className="flex flex-col gap-1.5">
        <label className="text-sm font-medium text-foreground" htmlFor="project-path">
          Path
        </label>
        <div className="flex gap-2">
          <input
            id="project-path"
            type="text"
            value={path}
            onChange={(e) => setPath(e.target.value)}
            placeholder="/home/user/projects/my-project"
            className={cn(
              "flex-1 rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground font-mono",
              "placeholder:text-muted-foreground",
              "focus:outline-none focus:ring-1 focus:ring-ring",
            )}
          />
          <button
            type="button"
            onClick={onPickFolder}
            title="Browse folder"
            className={cn(
              "flex items-center justify-center rounded-lg border border-border bg-background px-3",
              "text-muted-foreground hover:text-foreground hover:border-foreground",
              "transition-colors",
            )}
          >
            <FolderOpen className="h-4 w-4" />
          </button>
        </div>
      </div>

      <div className="flex flex-col gap-1.5">
        <span className="text-sm font-medium text-foreground">Theme</span>
        <div data-slot="settings-theme-grid">
          {THEMES.map((theme) => (
            <button
              key={theme.name}
              type="button"
              data-slot="settings-theme-card"
              data-active={selectedTheme === theme.name || undefined}
              onClick={() => setSelectedTheme(theme.name)}
              aria-label={`Select ${theme.label} theme`}
            >
              <div data-slot="settings-theme-swatch" style={{ background: theme.bg }}>
                <div data-slot="settings-theme-accent" style={{ background: theme.accent }} />
              </div>
              <span data-slot="settings-theme-label">{theme.label}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="flex gap-2">
        {onCancel && (
          <Button variant="outline" onClick={onCancel} className="flex-1">
            Cancel
          </Button>
        )}
        <Button
          onClick={onContinue}
          disabled={!canContinue || isPending}
          className="flex-1"
        >
          {isPending ? (
            <>
              <Loader2 className="h-4 w-4 animate-spin" />
              Creating...
            </>
          ) : (
            <>
              Continue
              <ChevronRight className="h-4 w-4" />
            </>
          )}
        </Button>
      </div>
    </div>
  );
}
