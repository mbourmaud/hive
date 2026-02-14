import { useEffect } from "react";
import type { Model } from "@/domains/settings/types";
import { useAppStore } from "@/store";

const MODEL_PRIORITY = ["opus", "sonnet", "haiku"];

/**
 * Auto-selects a default model when none is selected.
 * Priority: opus > sonnet > haiku > first available.
 */
export function useDefaultModel(models: Model[]) {
  const selectedModel = useAppStore((s) => s.selectedModel);
  const setSelectedModel = useAppStore((s) => s.setSelectedModel);

  useEffect(() => {
    if (selectedModel || models.length === 0) return;

    for (const tier of MODEL_PRIORITY) {
      const match = models.find((m) => m.id.includes(tier));
      if (match) {
        setSelectedModel(match.id);
        return;
      }
    }
    if (models[0]) {
      setSelectedModel(models[0].id);
    }
  }, [selectedModel, models, setSelectedModel]);
}
