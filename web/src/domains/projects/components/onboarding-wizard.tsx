import { Check, ChevronRight, Loader2, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import beeIcon from "@/assets/bee-icon.png";
import { useCreateProject } from "@/domains/projects/mutations";
import type { ProjectProfile } from "@/domains/projects/types";
import { useDetection } from "@/domains/projects/use-detection";
import { cn } from "@/shared/lib/utils";
import { THEMES, type ThemeInfo } from "@/shared/theme/use-theme";
import { Button } from "@/shared/ui/button";

// ── Types ───────────────────────────────────────────────────────────────────

type WizardStep = 1 | 2 | 3;

interface OnboardingWizardProps {
  onComplete: (project: ProjectProfile) => void;
}

// ── Theme Swatch ────────────────────────────────────────────────────────────

function ThemeSwatch({
  theme,
  selected,
  onSelect,
}: {
  theme: ThemeInfo;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onSelect}
      title={theme.label}
      className={cn(
        "relative flex h-10 w-full rounded-lg overflow-hidden transition-shadow",
        selected && "ring-2 ring-accent ring-offset-2 ring-offset-background",
      )}
    >
      <div className="flex-1" style={{ background: theme.bg } as React.CSSProperties} />
      <div className="w-3" style={{ background: theme.accent } as React.CSSProperties} />
    </button>
  );
}

// ── Step Icons ──────────────────────────────────────────────────────────────

function StepIndicator({ status }: { status: "pending" | "running" | "completed" | "failed" }) {
  switch (status) {
    case "running":
      return <Loader2 className="h-4 w-4 text-accent animate-spin" />;
    case "completed":
      return <Check className="h-4 w-4 text-success" />;
    case "failed":
      return <X className="h-4 w-4 text-destructive" />;
    default:
      return <div className="h-4 w-4 rounded-full border border-border" />;
  }
}

// ── Component ───────────────────────────────────────────────────────────────

export function OnboardingWizard({ onComplete }: OnboardingWizardProps) {
  const [step, setStep] = useState<WizardStep>(1);
  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [selectedTheme, setSelectedTheme] = useState<string>("hive");
  const [project, setProject] = useState<ProjectProfile | null>(null);

  const createProject = useCreateProject();
  const { steps: detectionSteps, isDetecting, context, startDetection } = useDetection();

  const canContinue = name.trim().length > 0 && path.trim().length > 0;

  const handleContinue = useCallback(() => {
    if (!canContinue || createProject.isPending) return;

    createProject.mutate(
      { name: name.trim(), path: path.trim(), color_theme: selectedTheme },
      {
        onSuccess: (created) => {
          setProject(created);
          setStep(2);
          startDetection(created.id);
        },
      },
    );
  }, [canContinue, createProject, name, path, selectedTheme, startDetection]);

  // Auto-advance from step 2 to step 3
  useEffect(() => {
    if (step === 2 && !isDetecting && context !== null) {
      setStep(3);
    }
  }, [step, isDetecting, context]);

  const handleComplete = useCallback(() => {
    if (project) {
      onComplete(project);
    }
  }, [project, onComplete]);

  return (
    <div
      data-component="onboarding-wizard"
      className="flex-1 flex items-center justify-center bg-background p-4"
    >
      <div className="w-full max-w-[600px]">
        {/* Header */}
        <div className="flex flex-col items-center gap-3 mb-8">
          <img src={beeIcon} alt="Hive" className="w-16 h-16" />
          <h1 className="text-2xl font-semibold text-foreground">
            {step === 1 && "Add your project"}
            {step === 2 && "Setting up your project..."}
            {step === 3 && "Ready!"}
          </h1>
          <p className="text-sm text-muted-foreground text-center">
            {step === 1 && "Configure your project to get started with Hive"}
            {step === 2 && "Detecting your project environment"}
            {step === 3 && "Your project is configured and ready to go"}
          </p>
        </div>

        {/* Step 1: Project config */}
        {step === 1 && (
          <div className="flex flex-col gap-5">
            {/* Error */}
            {createProject.error && (
              <div className="rounded-lg border border-destructive/30 bg-destructive/10 px-4 py-3 text-sm text-destructive">
                {createProject.error.message}
              </div>
            )}

            {/* Name */}
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

            {/* Path */}
            <div className="flex flex-col gap-1.5">
              <label className="text-sm font-medium text-foreground" htmlFor="project-path">
                Path
              </label>
              <input
                id="project-path"
                type="text"
                value={path}
                onChange={(e) => setPath(e.target.value)}
                placeholder="/home/user/projects/my-project"
                className={cn(
                  "w-full rounded-lg border border-border bg-background px-3 py-2 text-sm text-foreground font-mono",
                  "placeholder:text-muted-foreground",
                  "focus:outline-none focus:ring-1 focus:ring-ring",
                )}
              />
            </div>

            {/* Theme */}
            <div className="flex flex-col gap-1.5">
              <span className="text-sm font-medium text-foreground">Theme</span>
              <div className="grid grid-cols-3 gap-2">
                {THEMES.map((theme) => (
                  <ThemeSwatch
                    key={theme.name}
                    theme={theme}
                    selected={selectedTheme === theme.name}
                    onSelect={() => setSelectedTheme(theme.name)}
                  />
                ))}
              </div>
            </div>

            {/* Continue */}
            <Button
              onClick={handleContinue}
              disabled={!canContinue || createProject.isPending}
              className="w-full"
            >
              {createProject.isPending ? (
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
        )}

        {/* Step 2: Detection */}
        {step === 2 && (
          <div className="flex flex-col gap-3">
            {detectionSteps.map((ds) => (
              <div
                key={ds.step}
                className="flex items-center gap-3 rounded-lg border border-border bg-card px-4 py-3"
              >
                <StepIndicator status={ds.status} />
                <span
                  className={cn(
                    "text-sm",
                    ds.status === "completed" && "text-foreground",
                    ds.status === "running" && "text-foreground",
                    ds.status === "failed" && "text-destructive",
                    ds.status === "pending" && "text-muted-foreground",
                  )}
                >
                  {ds.label}
                  {ds.status === "running" && "..."}
                </span>
              </div>
            ))}
          </div>
        )}

        {/* Step 3: Summary */}
        {step === 3 && context && (
          <div className="flex flex-col gap-4">
            <div className="rounded-lg border border-border bg-card p-4">
              <div className="flex flex-col gap-3">
                {/* Branch */}
                {context.git && (
                  <div className="flex items-center gap-2 text-sm">
                    <span className="text-muted-foreground">Branch:</span>
                    <span className="font-mono text-foreground">{context.git.branch}</span>
                  </div>
                )}

                {/* Runtimes */}
                {context.runtimes.length > 0 && (
                  <div className="flex items-center gap-2 text-sm">
                    <span className="text-muted-foreground">Runtimes:</span>
                    <div className="flex flex-wrap gap-1.5">
                      {context.runtimes.map((rt) => (
                        <span
                          key={rt.name}
                          className="inline-flex items-center rounded-md bg-muted px-2 py-0.5 text-xs font-medium text-foreground"
                        >
                          {rt.name}
                          {rt.version ? ` ${rt.version}` : ""}
                        </span>
                      ))}
                    </div>
                  </div>
                )}

                {/* Key files */}
                {context.key_files.length > 0 && (
                  <div className="flex items-center gap-2 text-sm">
                    <span className="text-muted-foreground">Key files:</span>
                    <span className="font-mono text-foreground text-xs">
                      {context.key_files.join(", ")}
                    </span>
                  </div>
                )}

                {/* PR link */}
                {context.open_pr && (
                  <div className="flex items-center gap-2 text-sm">
                    <span className="text-muted-foreground">Open PR:</span>
                    <a
                      href={context.open_pr.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-accent hover:underline"
                    >
                      #{context.open_pr.number} {context.open_pr.title}
                    </a>
                  </div>
                )}
              </div>
            </div>

            <Button onClick={handleComplete} className="w-full">
              Start coding
              <ChevronRight className="h-4 w-4" />
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}
