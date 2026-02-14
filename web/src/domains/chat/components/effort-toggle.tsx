import type { EffortLevel } from "@/domains/settings/store";
import { cn } from "@/shared/lib/utils";

const LEVELS: { value: EffortLevel; label: string }[] = [
  { value: "low", label: "Lo" },
  { value: "medium", label: "Med" },
  { value: "high", label: "Hi" },
];

interface EffortToggleProps {
  effort: EffortLevel;
  onChange: (effort: EffortLevel) => void;
  disabled?: boolean;
}

export function EffortToggle({ effort, onChange, disabled }: EffortToggleProps) {
  return (
    <div data-component="effort-toggle" className="flex items-center gap-0.5">
      {LEVELS.map(({ value, label }) => (
        <button
          key={value}
          type="button"
          disabled={disabled}
          onClick={() => onChange(value)}
          className={cn(
            "px-1.5 py-0.5 text-[11px] font-medium rounded transition-colors",
            "hover:bg-muted",
            effort === value ? "bg-muted text-foreground" : "text-muted-foreground",
            disabled && "opacity-50 pointer-events-none",
          )}
          aria-label={`Set effort to ${value}`}
          aria-pressed={effort === value}
        >
          {label}
        </button>
      ))}
    </div>
  );
}
