import { useCallback, useRef, useState } from "react";
import type { DetectionEvent, DetectionStep, ProjectContext } from "./types";

const INITIAL_STEPS: DetectionStep[] = [
  { step: "git", label: "Scanning git repository", status: "pending" },
  { step: "runtimes", label: "Detecting runtimes & versions", status: "pending" },
  { step: "key_files", label: "Finding configuration files", status: "pending" },
  { step: "pr", label: "Checking for open PR/MR", status: "pending" },
];

function isDetectionEvent(data: unknown): data is DetectionEvent {
  if (typeof data !== "object" || data === null) return false;
  const obj = data as Record<string, unknown>;
  return typeof obj.type === "string";
}

export function useDetection() {
  const [steps, setSteps] = useState<DetectionStep[]>(INITIAL_STEPS);
  const [isDetecting, setIsDetecting] = useState(false);
  const [context, setContext] = useState<ProjectContext | null>(null);
  const eventSourceRef = useRef<EventSource | null>(null);

  const startDetection = useCallback((projectId: string) => {
    // Close any existing connection
    if (eventSourceRef.current) {
      eventSourceRef.current.close();
    }

    setSteps(INITIAL_STEPS.map((s) => ({ ...s, status: "pending" })));
    setIsDetecting(true);
    setContext(null);

    const es = new EventSource(`/api/registry/projects/${projectId}/detect`);
    eventSourceRef.current = es;

    es.onmessage = (event) => {
      try {
        const parsed: unknown = JSON.parse(event.data);
        if (!isDetectionEvent(parsed)) return;

        switch (parsed.type) {
          case "step_started":
            setSteps((prev) =>
              prev.map((s) => (s.step === parsed.step ? { ...s, status: "running" } : s)),
            );
            break;
          case "step_completed":
            setSteps((prev) =>
              prev.map((s) =>
                s.step === parsed.step ? { ...s, status: "completed", result: parsed.result } : s,
              ),
            );
            break;
          case "step_failed":
            setSteps((prev) =>
              prev.map((s) =>
                s.step === parsed.step ? { ...s, status: "failed", error: parsed.error } : s,
              ),
            );
            break;
          case "all_complete":
            setContext(parsed.context);
            setIsDetecting(false);
            es.close();
            break;
        }
      } catch {
        // Ignore parse errors
      }
    };

    es.onerror = () => {
      setIsDetecting(false);
      es.close();
    };
  }, []);

  return { steps, isDetecting, context, startDetection };
}
