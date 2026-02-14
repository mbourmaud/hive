import type { EffortLevel } from "@/domains/settings/store";
import type { CyclePillOption } from "./cycle-pill";
import { CyclePill } from "./cycle-pill";

const EFFORT_OPTIONS: CyclePillOption<EffortLevel>[] = [
  {
    value: "low",
    label: "Low",
    color: "var(--success)",
    tooltip: "Faster, less thorough responses",
  },
  {
    value: "medium",
    label: "Medium",
    color: "var(--muted-foreground)",
    tooltip: "Balanced speed and quality",
  },
  {
    value: "high",
    label: "High",
    color: "var(--destructive)",
    tooltip: "Slower, more thorough responses",
  },
];

interface EffortToggleProps {
  effort: EffortLevel;
  onChange: (effort: EffortLevel) => void;
  disabled?: boolean;
}

export function EffortToggle({ effort, onChange, disabled }: EffortToggleProps) {
  return (
    <CyclePill
      options={EFFORT_OPTIONS}
      value={effort}
      onChange={onChange}
      disabled={disabled}
      label="Effort"
    />
  );
}
