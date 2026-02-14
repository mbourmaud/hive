import { useState } from "react";
import { cn } from "@/shared/lib/utils";

// ── Types ────────────────────────────────────────────────────────────────────

interface CyclePillOption<T extends string> {
  value: T;
  label: string;
  /** CSS color value or CSS variable reference for this option's tint */
  color: string;
  /** Tooltip text shown on hover */
  tooltip?: string;
}

interface CyclePillProps<T extends string> {
  options: CyclePillOption<T>[];
  value: T;
  onChange: (value: T) => void;
  disabled?: boolean;
  label?: string;
}

// ── Component ────────────────────────────────────────────────────────────────

export type { CyclePillOption };

export function CyclePill<T extends string>({
  options,
  value,
  onChange,
  disabled,
  label,
}: CyclePillProps<T>) {
  const [hovered, setHovered] = useState(false);
  const currentIndex = options.findIndex((o) => o.value === value);
  const current = options[currentIndex];
  if (!current) return null;

  function handleClick() {
    if (disabled) return;
    const nextIndex = (currentIndex + 1) % options.length;
    const next = options[nextIndex];
    if (next) onChange(next.value);
  }

  const ariaLabel = label
    ? `${label}: ${current.label}. Click to cycle.`
    : `${current.label}. Click to cycle.`;

  return (
    <div data-component="cycle-pill-wrapper" className="relative">
      <button
        type="button"
        data-component="cycle-pill"
        disabled={disabled}
        onClick={handleClick}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        className={cn(
          "inline-flex items-center gap-1 rounded-md px-2 py-0.5",
          "text-[11px] font-semibold tracking-wide uppercase",
          "border transition-all duration-150 cursor-pointer select-none",
          "hover:brightness-110 active:scale-[0.97]",
          disabled && "opacity-40 pointer-events-none",
        )}
        style={
          {
            "--pill-color": current.color,
            borderColor: "oklch(from var(--pill-color) l c h / 0.25)",
            background: "oklch(from var(--pill-color) l c h / 0.1)",
            color: current.color,
          } as React.CSSProperties
        }
        aria-label={ariaLabel}
      >
        <span
          data-slot="pill-dot"
          className="h-1.5 w-1.5 rounded-full shrink-0"
          style={{ background: current.color }}
        />
        {current.label}
      </button>

      {hovered && current.tooltip && (
        <div data-slot="pill-tooltip" role="tooltip">
          {current.tooltip}
        </div>
      )}
    </div>
  );
}
