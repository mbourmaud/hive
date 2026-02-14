import { Check, Loader2, X } from "lucide-react";
import type { DetectionStepStatus } from "@/domains/projects/types";

export function StepIndicator({ status }: { status: DetectionStepStatus }) {
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
