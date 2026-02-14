import type { ChatMode } from "@/domains/settings/store";
import type { CyclePillOption } from "./cycle-pill";
import { CyclePill } from "./cycle-pill";

const MODE_OPTIONS: CyclePillOption<ChatMode>[] = [
  {
    value: "code",
    label: "Code",
    color: "var(--model-sonnet)",
    tooltip: "Direct code editing and generation",
  },
  {
    value: "hive-plan",
    label: "Hive Plan",
    color: "var(--honey)",
    tooltip: "Collaborative planning with Hive drones",
  },
  {
    value: "plan",
    label: "Plan",
    color: "var(--model-opus)",
    tooltip: "Step-by-step implementation planning",
  },
];

interface ModeToggleProps {
  mode: ChatMode;
  onChange: (mode: ChatMode) => void;
  disabled?: boolean;
}

export function ModeToggle({ mode, onChange, disabled }: ModeToggleProps) {
  return (
    <CyclePill
      options={MODE_OPTIONS}
      value={mode}
      onChange={onChange}
      disabled={disabled}
      label="Mode"
    />
  );
}
