import { useMemo } from "react";
import type { Model } from "@/domains/settings/types";
import { CyclePill } from "./cycle-pill";

// ── Helpers ──────────────────────────────────────────────────────────────────

/** Map a model ID to its color token. Falls back to accent for unknown models. */
function getModelColor(id: string): string {
  if (id.includes("opus")) return "var(--model-opus)";
  if (id.includes("sonnet")) return "var(--model-sonnet)";
  if (id.includes("haiku")) return "var(--model-haiku)";
  return "var(--accent)";
}

/** Extract a short display name from a Model. */
function getShortName(model: Model): string {
  return model.name.replace(/^Claude\s*/i, "") || model.name;
}

// ── Types ────────────────────────────────────────────────────────────────────

interface ModelSelectorProps {
  models: Model[];
  selected: string;
  onChange: (modelId: string) => void;
  disabled?: boolean;
}

// ── Component ────────────────────────────────────────────────────────────────

export function ModelSelector({ models, selected, onChange, disabled }: ModelSelectorProps) {
  const options = useMemo(
    () =>
      models.map((m) => ({
        value: m.id,
        label: getShortName(m),
        color: getModelColor(m.id),
        tooltip: m.description,
      })),
    [models],
  );

  if (options.length === 0) return null;

  return (
    <CyclePill
      options={options}
      value={selected}
      onChange={onChange}
      disabled={disabled}
      label="Model"
    />
  );
}
