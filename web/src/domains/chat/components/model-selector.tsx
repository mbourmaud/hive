import { ChevronDown } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { Model } from "@/domains/settings/types";
import { cn } from "@/shared/lib/utils";

// ── Types ────────────────────────────────────────────────────────────────────

interface ModelSelectorProps {
  models: Model[];
  selected: string;
  onChange: (modelId: string) => void;
  disabled?: boolean;
}

// ── Component ────────────────────────────────────────────────────────────────

export function ModelSelector({
  models,
  selected,
  onChange,
  disabled = false,
}: ModelSelectorProps) {
  const [open, setOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const selectedModel = models.find((m) => m.id === selected);
  const label = selectedModel?.name ?? selected;

  // Close on click outside
  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [open]);

  // Close on Escape
  useEffect(() => {
    if (!open) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [open]);

  return (
    <div ref={containerRef} data-component="model-selector" className="relative">
      <button
        type="button"
        onClick={() => !disabled && setOpen(!open)}
        disabled={disabled}
        className={cn(
          "inline-flex items-center gap-1 rounded-md px-2 py-1 text-xs",
          "text-muted-foreground hover:text-foreground",
          "transition-colors",
          disabled && "opacity-50 cursor-not-allowed",
        )}
      >
        {label}
        <ChevronDown className={cn("h-3 w-3 transition-transform", open && "rotate-180")} />
      </button>

      {open && (
        <div
          data-slot="model-dropdown"
          className={cn(
            "absolute bottom-full left-0 mb-1 z-50",
            "min-w-[200px] rounded-lg border border-border bg-card shadow-lg",
            "py-1",
          )}
        >
          {models.map((model) => (
            <button
              key={model.id}
              type="button"
              onClick={() => {
                onChange(model.id);
                setOpen(false);
              }}
              className={cn(
                "w-full text-left px-3 py-2 text-sm transition-colors",
                "hover:bg-muted",
                model.id === selected ? "text-foreground font-medium" : "text-muted-foreground",
              )}
            >
              <div>{model.name}</div>
              <div className="text-xs text-muted-foreground/60">{model.description}</div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
