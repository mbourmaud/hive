import * as Collapsible from "@radix-ui/react-collapsible";
import { Archive, ArchiveRestore, ChevronDown, Eye, FileText, Rocket, Trash2 } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { MarkdownRenderer } from "@/domains/chat/components/markdown-renderer";
import { useAppStore } from "@/store";
import { useToast } from "@/shared/ui/toast";
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

/** Build API URL with optional project_path scoping. */
function planApiUrl(path: string, projectPath: string | null, extra?: Record<string, string>): string {
  const params = new URLSearchParams(extra);
  if (projectPath) params.set("project_path", projectPath);
  const qs = params.toString();
  return qs ? `${path}?${qs}` : path;
}

// ── Component ──────────────────────────────────────────────────────────────

export function PlansContent({ onDispatch }: PlansContentProps) {
  const selectedProject = useAppStore((s) => s.selectedProject);
  const [plans, setPlans] = useState<PlanSummary[]>([]);
  const [archived, setArchived] = useState<PlanSummary[]>([]);
  const [archivedOpen, setArchivedOpen] = useState(false);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [dispatching, setDispatching] = useState<string | null>(null);
  const [viewingPlan, setViewingPlan] = useState<PlanDetail | null>(null);
  const [viewingArchived, setViewingArchived] = useState(false);
  const { toast } = useToast();

  const fetchAll = useCallback(async () => {
    const [activeRes, archivedRes] = await Promise.all([
      fetch(planApiUrl("/api/plans", selectedProject)).catch(() => null),
      fetch(planApiUrl("/api/plans", selectedProject, { archived: "true" })).catch(() => null),
    ]);
    if (activeRes?.ok) {
      const data: unknown = await activeRes.json();
      if (Array.isArray(data)) setPlans(data as PlanSummary[]);
    }
    if (archivedRes?.ok) {
      const data: unknown = await archivedRes.json();
      if (Array.isArray(data)) setArchived(data as PlanSummary[]);
    }
  }, [selectedProject]);

  useEffect(() => {
    fetchAll();
    const interval = setInterval(fetchAll, POLL_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [fetchAll]);

  const handleView = useCallback(async (id: string, isArchived: boolean) => {
    try {
      const res = await fetch(planApiUrl(`/api/plans/${id}`, selectedProject));
      if (res.ok) {
        setViewingPlan((await res.json()) as PlanDetail);
        setViewingArchived(isArchived);
      }
    } catch { /* silent */ }
  }, [selectedProject]);

  const handleDispatch = useCallback(async (planId: string) => {
    setDispatching(planId);
    try {
      const res = await fetch(planApiUrl(`/api/plans/${planId}/dispatch`, selectedProject), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ droneName: planId, model: "sonnet" }),
      });
      if (res.ok) {
        toast(`Drone '${planId}' dispatched`, "success");
        setViewingPlan(null);
        onDispatch?.(planId);
      } else {
        const body = await res.text().catch(() => "");
        toast(body || `Dispatch failed (${res.status})`, "error");
      }
    } catch {
      toast("Network error — could not dispatch", "error");
    } finally {
      setDispatching(null);
    }
  }, [selectedProject, onDispatch, toast]);

  const handleArchive = useCallback(async (id: string) => {
    try {
      const res = await fetch(planApiUrl(`/api/plans/${id}/archive`, selectedProject), { method: "POST" });
      if (res.ok) {
        setPlans((prev) => {
          const plan = prev.find((p) => p.id === id);
          if (plan) setArchived((a) => [plan, ...a]);
          return prev.filter((p) => p.id !== id);
        });
        if (expandedId === id) setExpandedId(null);
        toast("Plan archived", "success");
      }
    } catch { /* silent */ }
  }, [selectedProject, expandedId, toast]);

  const handleUnarchive = useCallback(async (id: string) => {
    try {
      const res = await fetch(planApiUrl(`/api/plans/${id}/unarchive`, selectedProject), { method: "POST" });
      if (res.ok) {
        setArchived((prev) => {
          const plan = prev.find((p) => p.id === id);
          if (plan) setPlans((a) => [plan, ...a]);
          return prev.filter((p) => p.id !== id);
        });
        toast("Plan restored", "success");
      }
    } catch { /* silent */ }
  }, [selectedProject, toast]);

  const handleDelete = useCallback(async (id: string) => {
    try {
      const res = await fetch(planApiUrl(`/api/plans/${id}`, selectedProject), { method: "DELETE" });
      if (res.ok) {
        setPlans((prev) => prev.filter((p) => p.id !== id));
        setArchived((prev) => prev.filter((p) => p.id !== id));
        if (expandedId === id) setExpandedId(null);
        if (viewingPlan?.id === id) setViewingPlan(null);
      }
    } catch { /* silent */ }
  }, [selectedProject, expandedId, viewingPlan?.id]);

  // ── Empty state ──────────────────────────────────────────────────────

  if (plans.length === 0 && archived.length === 0) {
    return (
      <div className="p-4 text-sm text-muted-foreground">
        No plans found. Use Hive Plan mode to create one.
      </div>
    );
  }

  return (
    <>
      <div className="flex-1 overflow-y-auto">
        {/* Active plans */}
        {plans.map((plan) => (
          <ActivePlanItem
            key={plan.id}
            plan={plan}
            isExpanded={expandedId === plan.id}
            isDispatching={dispatching === plan.id}
            onToggle={() => setExpandedId(expandedId === plan.id ? null : plan.id)}
            onView={() => handleView(plan.id, false)}
            onDispatch={() => handleDispatch(plan.id)}
            onArchive={() => handleArchive(plan.id)}
            onDelete={() => handleDelete(plan.id)}
          />
        ))}

        {/* Archived section */}
        {archived.length > 0 && (
          <Collapsible.Root open={archivedOpen} onOpenChange={setArchivedOpen}>
            <Collapsible.Trigger asChild>
              <button type="button" data-slot="archived-section-trigger">
                <ChevronDown
                  className="h-3 w-3 text-muted-foreground shrink-0 transition-transform"
                  style={{ transform: archivedOpen ? "rotate(0deg)" : "rotate(-90deg)" }}
                />
                <span className="text-xs font-medium text-muted-foreground">
                  Archived ({archived.length})
                </span>
              </button>
            </Collapsible.Trigger>
            <Collapsible.Content>
              {archived.map((plan) => (
                <ArchivedPlanItem
                  key={plan.id}
                  plan={plan}
                  onView={() => handleView(plan.id, true)}
                  onUnarchive={() => handleUnarchive(plan.id)}
                  onDelete={() => handleDelete(plan.id)}
                />
              ))}
            </Collapsible.Content>
          </Collapsible.Root>
        )}
      </div>

      <PlanViewerModal
        plan={viewingPlan}
        onClose={() => setViewingPlan(null)}
        onDispatch={viewingArchived ? undefined : handleDispatch}
        isDispatching={dispatching !== null}
      />
    </>
  );
}

// ── Active plan item ──────────────────────────────────────────────────────

function ActivePlanItem({ plan, isExpanded, isDispatching, onToggle, onView, onDispatch, onArchive, onDelete }: {
  plan: PlanSummary; isExpanded: boolean; isDispatching: boolean;
  onToggle: () => void; onView: () => void; onDispatch: () => void;
  onArchive: () => void; onDelete: () => void;
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
              <div className="text-[11px] font-medium text-muted-foreground uppercase tracking-wide mb-1.5">TL;DR</div>
              <MarkdownRenderer text={plan.tldr} cacheKey={`plan-tldr-${plan.id}`} className="plan-tldr" />
            </div>
          )}
          <div className="flex items-center flex-wrap gap-1.5 mt-2">
            <PlanButton icon={Eye} label="View" onClick={onView} />
            <PlanButton icon={Rocket} label={isDispatching ? "Dispatching..." : "Dispatch"} onClick={onDispatch} disabled={isDispatching} variant="accent" />
            <PlanButton icon={Archive} label="Archive" onClick={onArchive} variant="muted" />
            <PlanButton icon={Trash2} label="Delete" onClick={onDelete} variant="destructive" />
          </div>
        </div>
      </Collapsible.Content>
    </Collapsible.Root>
  );
}

// ── Archived plan item (compact) ──────────────────────────────────────────

function ArchivedPlanItem({ plan, onView, onUnarchive, onDelete }: {
  plan: PlanSummary; onView: () => void; onUnarchive: () => void; onDelete: () => void;
}) {
  return (
    <div data-slot="archived-plan-item">
      <FileText className="h-3 w-3 text-muted-foreground shrink-0" />
      <span className="text-xs text-muted-foreground truncate flex-1">{plan.title}</span>
      <button type="button" className="archived-action" title="View" onClick={onView}><Eye className="h-3 w-3" /></button>
      <button type="button" className="archived-action" title="Restore" onClick={onUnarchive}><ArchiveRestore className="h-3 w-3" /></button>
      <button type="button" className="archived-action text-destructive" title="Delete" onClick={onDelete}><Trash2 className="h-3 w-3" /></button>
    </div>
  );
}

// ── Shared button helper ─────────────────────────────────────────────────

const VARIANT_CLASSES: Record<string, string> = {
  default: "bg-muted text-foreground hover:bg-muted/70",
  accent: "bg-accent text-accent-foreground hover:bg-accent/90 disabled:opacity-50",
  muted: "bg-muted text-muted-foreground hover:bg-muted/70",
  destructive: "bg-destructive/10 text-destructive hover:bg-destructive/20",
};

function PlanButton({ icon: Icon, label, onClick, disabled, variant = "default" }: {
  icon: React.ComponentType<{ className?: string }>; label: string;
  onClick: () => void; disabled?: boolean; variant?: string;
}) {
  return (
    <button
      type="button"
      className={`inline-flex items-center gap-1 px-2.5 py-1.5 rounded-md text-xs font-semibold transition-colors ${VARIANT_CLASSES[variant] ?? VARIANT_CLASSES.default}`}
      onClick={(e) => { e.stopPropagation(); onClick(); }}
      disabled={disabled}
    >
      <Icon className="h-3 w-3" />
      {label}
    </button>
  );
}
