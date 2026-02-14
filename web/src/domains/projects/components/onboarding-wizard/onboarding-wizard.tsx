import { ChevronRight } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import beeIcon from "@/assets/bee-icon.png";
import { useCreateProject } from "@/domains/projects/mutations";
import type { ProjectProfile } from "@/domains/projects/types";
import { useDetection } from "@/domains/projects/use-detection";
import { cn } from "@/shared/lib/utils";
import { Button } from "@/shared/ui/button";
import "@/domains/settings/components/settings-dialog/theme-grid.css";
import { type WizardStep, STEP_HEADERS, pickFolder } from "./constants";
import { StepIndicator } from "./step-indicator";
import { StepOneForm } from "./step-one-form";

// ── Types ───────────────────────────────────────────────────────────────────

interface OnboardingWizardProps {
  onComplete: (project: ProjectProfile) => void;
  onCancel?: () => void;
}

// ── Component ───────────────────────────────────────────────────────────────

export function OnboardingWizard({ onComplete, onCancel }: OnboardingWizardProps) {
  const [step, setStep] = useState<WizardStep>(1);
  const [name, setName] = useState("");
  const [path, setPath] = useState("");
  const [selectedTheme, setSelectedTheme] = useState<string>("hive");
  const [project, setProject] = useState<ProjectProfile | null>(null);

  const createProject = useCreateProject();
  const { steps: detectionSteps, isDetecting, context, startDetection } = useDetection();

  const canContinue = name.trim().length > 0 && path.trim().length > 0;

  const handlePickFolder = useCallback(async () => {
    const folderName = await pickFolder();
    if (folderName) {
      if (!name) {
        setName(folderName);
      }
    }
  }, [name]);

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
          <h1 className="text-2xl font-semibold text-foreground">{STEP_HEADERS[step].title}</h1>
          <p className="text-sm text-muted-foreground text-center">{STEP_HEADERS[step].subtitle}</p>
        </div>

        {/* Step 1: Project config */}
        {step === 1 && (
          <StepOneForm
            name={name}
            setName={setName}
            path={path}
            setPath={setPath}
            selectedTheme={selectedTheme}
            setSelectedTheme={setSelectedTheme}
            canContinue={canContinue}
            isPending={createProject.isPending}
            error={createProject.error}
            onPickFolder={handlePickFolder}
            onContinue={handleContinue}
            onCancel={onCancel}
          />
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
                {context.git && (
                  <div className="flex items-center gap-2 text-sm">
                    <span className="text-muted-foreground">Branch:</span>
                    <span className="font-mono text-foreground">{context.git.branch}</span>
                  </div>
                )}
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
                {context.key_files.length > 0 && (
                  <div className="flex items-center gap-2 text-sm">
                    <span className="text-muted-foreground">Key files:</span>
                    <span className="font-mono text-foreground text-xs">
                      {context.key_files.join(", ")}
                    </span>
                  </div>
                )}
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
