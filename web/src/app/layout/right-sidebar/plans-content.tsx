import * as Collapsible from "@radix-ui/react-collapsible";
import { Eye, FileText, Rocket, Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { MarkdownRenderer } from "@/domains/chat/components/markdown-renderer";
import { type PlanDetail, PlanViewerModal } from "./plan-viewer-modal";

// ── Types ──────────────────────────────────────────────────────────────────

interface PlanSummary {
  id: string;
  title: string;
  tldr: string | null;
  task_count: number;
  created_at: string;
  updated_at: string;
}

interface PlansContentProps {
  onDispatch?: (droneName: string) => void;
}

// ── Constants ──────────────────────────────────────────────────────────────

const POLL_INTERVAL_MS = 10_000;

// ── Component ──────────────────────────────────────────────────────────────

export function PlansContent({ onDispatch }: PlansContentProps) {
  const [plans, setPlans] = useState<PlanSummary[]>([]);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [dispatching, setDispatching] = useState<string | null>(null);
  const [viewingPlan, setViewingPlan] = useState<PlanDetail | null>(null);

  const fetchPlans = useCallback(async () => {
    try {
      const res = await fetch("/api/plans");
      if (res.ok) {
        const data: unknown = await res.json();
        if (Array.isArray(data)) setPlans(data as PlanSummary[]);
      }
    } catch {
      // silent
    }
  }, []);

  useEffect(() => {
    fetchPlans();
    const interval = setInterval(fetchPlans, POLL_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [fetchPlans]);

  const handleView = useCallback(async (id: string) => {
    try {
      const res = await fetch(`/api/plans/${id}`);
      if (res.ok) {
        const data = (await res.json()) as PlanDetail;
        setViewingPlan(data);
      }
    } catch {
      // silent
    }
  }, []);

  const handleDispatch = useCallback(
    async (planId: string) => {
      setDispatching(planId);
      try {
        const res = await fetch(`/api/plans/${planId}/dispatch`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ droneName: planId, model: "sonnet" }),
        });
        if (res.ok) {
          setViewingPlan(null);
          onDispatch?.(planId);
        }
      } catch {
        // silent
      } finally {
        setDispatching(null);
      }
    },
    [onDispatch],
  );

  const handleDispatchFromList = useCallback(
    async (plan: PlanSummary) => {
      await handleDispatch(plan.id);
    },
    [handleDispatch],
  );

  const handleDelete = useCallback(
    async (id: string) => {
      try {
        const res = await fetch(`/api/plans/${id}`, { method: "DELETE" });
        if (res.ok) {
          setPlans((prev) => prev.filter((p) => p.id !== id));
          if (expandedId === id) setExpandedId(null);
          if (viewingPlan?.id === id) setViewingPlan(null);
        }
      } catch {
        // silent
      }
    },
    [expandedId, viewingPlan?.id],
  );

  // ── Empty state ──────────────────────────────────────────────────────

  if (plans.length === 0) {
    return (
      <div className="p-4 text-sm text-muted-foreground">
        No plans found. Use Hive Plan mode to create one.
      </div>
    );
  }

  // ── Plan list + modal ────────────────────────────────────────────────

  return (
    <>
      <div className="flex-1 overflow-y-auto">
        {plans.map((plan) => (
          <PlanItem
            key={plan.id}
            plan={plan}
            isExpanded={expandedId === plan.id}
            isDispatching={dispatching === plan.id}
            onToggle={() => setExpandedId(expandedId === plan.id ? null : plan.id)}
            onView={() => handleView(plan.id)}
            onDispatch={() => handleDispatchFromList(plan)}
            onDelete={() => handleDelete(plan.id)}
          />
        ))}
      </div>

      <PlanViewerModal
        plan={viewingPlan}
        onClose={() => setViewingPlan(null)}
        onDispatch={handleDispatch}
        isDispatching={dispatching !== null}
      />
    </>
  );
}

// ── Plan list item ─────────────────────────────────────────────────────────

function PlanItem({
  plan,
  isExpanded,
  isDispatching,
  onToggle,
  onView,
  onDispatch,
  onDelete,
}: {
  plan: PlanSummary;
  isExpanded: boolean;
  isDispatching: boolean;
  onToggle: () => void;
  onView: () => void;
  onDispatch: () => void;
  onDelete: () => void;
}) {
  return (
    <Collapsible.Root open={isExpanded} onOpenChange={onToggle}>
      <Collapsible.Trigger asChild>
        <button type="button" data-slot="drone-panel-item" data-expanded={isExpanded || undefined}>
          <div className="flex items-center gap-2.5">
            <FileText className="h-3.5 w-3.5 text-muted-foreground shrink-0" />
            <span className="text-sm font-semibold truncate flex-1">{plan.title}</span>
            <span className="inline-flex items-center justify-center h-4 min-w-[16px] px-1 rounded-full bg-accent/15 text-accent text-[10px] font-bold leading-none">
              {plan.task_count}
            </span>
          </div>
        </button>
      </Collapsible.Trigger>

      <Collapsible.Content>
        <div data-slot="drone-panel-detail">
          {plan.tldr && (
            <div className="mb-3">
              <div className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide mb-1.5">
                TL;DR
              </div>
              <MarkdownRenderer text={plan.tldr} cacheKey={`plan-tldr-${plan.id}`} className="plan-tldr" />
            </div>
          )}

          <div className="flex items-center gap-2 mt-2">
            <button
              type="button"
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-muted text-foreground text-xs font-semibold hover:bg-muted/70 transition-colors"
              onClick={(e) => {
                e.stopPropagation();
                onView();
              }}
            >
              <Eye className="h-3 w-3" />
              View
            </button>
            <button
              type="button"
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-accent text-accent-foreground text-xs font-semibold hover:bg-accent/90 transition-colors disabled:opacity-50"
              onClick={(e) => {
                e.stopPropagation();
                onDispatch();
              }}
              disabled={isDispatching}
            >
              <Rocket className="h-3 w-3" />
              {isDispatching ? "Dispatching..." : "Dispatch"}
            </button>
            <button
              type="button"
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-destructive/10 text-destructive text-xs font-semibold hover:bg-destructive/20 transition-colors"
              onClick={(e) => {
                e.stopPropagation();
                onDelete();
              }}
            >
              <Trash2 className="h-3 w-3" />
              Delete
            </button>
          </div>
        </div>
      </Collapsible.Content>
    </Collapsible.Root>
  );
}
