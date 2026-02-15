import * as Dialog from "@radix-ui/react-dialog";
import { Rocket, X } from "lucide-react";
import { useMemo } from "react";
import { MarkdownRenderer } from "@/domains/chat/components/markdown-renderer";
import { PlanFilesList } from "./plan-file-icons";
import "./plan-viewer.css";

// ── Types ────────────────────────────────────────────────────────────────────

export interface PlanDetail {
  id: string;
  content: string;
  title: string;
}

interface PlanViewerModalProps {
  plan: PlanDetail | null;
  onClose: () => void;
  onDispatch?: (planId: string) => void;
  isDispatching?: boolean;
}

// ── Metadata parsing ─────────────────────────────────────────────────────────

interface TaskMeta {
  type?: string;
  model?: string;
  files?: string[];
  dependsOn?: string;
}

/** A contiguous markdown block or a metadata block between task headings */
type PlanSegment =
  | { kind: "markdown"; content: string }
  | { kind: "meta"; meta: TaskMeta };

const METADATA_KEYS = new Set(["type", "model", "files", "depends_on"]);
const METADATA_LINE_RE = /^- (\w+):\s*(.+)$/;
const TASK_HEADING_RE = /^### \d+\./;

function parsePlanSegments(markdown: string): PlanSegment[] {
  const lines = markdown.split("\n");
  const segments: PlanSegment[] = [];
  let mdLines: string[] = [];
  let i = 0;

  function flushMarkdown() {
    if (mdLines.length > 0) {
      segments.push({ kind: "markdown", content: mdLines.join("\n") });
      mdLines = [];
    }
  }

  while (i < lines.length) {
    const line = lines[i] ?? "";

    if (TASK_HEADING_RE.test(line)) {
      mdLines.push(line);
      i++;

      // Collect metadata lines after the heading
      const meta: TaskMeta = {};
      let foundMeta = false;
      while (i < lines.length) {
        const current = lines[i] ?? "";
        const match = current.match(METADATA_LINE_RE);
        const key = match?.[1] ?? "";
        const value = match?.[2] ?? "";
        if (match && METADATA_KEYS.has(key)) {
          foundMeta = true;
          if (key === "type") meta.type = value;
          else if (key === "model") meta.model = value;
          else if (key === "files") meta.files = value.split(",").map((f) => f.trim());
          else if (key === "depends_on") meta.dependsOn = value;
          i++;
        } else {
          break;
        }
      }

      if (foundMeta) {
        flushMarkdown();
        segments.push({ kind: "meta", meta });
      }
      continue;
    }

    mdLines.push(line);
    i++;
  }

  flushMarkdown();
  return segments;
}

// ── Metadata pills (React) ──────────────────────────────────────────────────

function getModelClass(model: string): string {
  const lower = model.toLowerCase().trim();
  if (lower.includes("opus")) return "plan-meta-pill-model-opus";
  if (lower.includes("haiku")) return "plan-meta-pill-model-haiku";
  return "plan-meta-pill-model-sonnet";
}

function MetaPills({ meta }: { meta: TaskMeta }) {
  return (
    <>
      <div className="plan-meta">
        {meta.type && (
          <span className="plan-meta-pill plan-meta-pill-type">{meta.type}</span>
        )}
        {meta.model && (
          <span className={`plan-meta-pill plan-meta-pill-model ${getModelClass(meta.model)}`}>
            {meta.model}
          </span>
        )}
        {meta.dependsOn && (
          <span className="plan-meta-pill plan-meta-pill-depends">
            depends on {meta.dependsOn}
          </span>
        )}
      </div>
      {meta.files && <PlanFilesList files={meta.files} />}
    </>
  );
}

// ── Component ────────────────────────────────────────────────────────────────

export function PlanViewerModal({
  plan,
  onClose,
  onDispatch,
  isDispatching,
}: PlanViewerModalProps) {
  const segments = useMemo(() => {
    if (!plan) return [];
    return parsePlanSegments(plan.content);
  }, [plan]);

  return (
    <Dialog.Root open={plan !== null} onOpenChange={(open) => !open && onClose()}>
      <Dialog.Portal>
        <Dialog.Overlay data-component="plan-viewer-overlay" />
        <Dialog.Content
          data-component="plan-viewer"
          aria-describedby={undefined}
          aria-label={plan?.title ?? "Plan viewer"}
        >
          <Dialog.Title className="sr-only">{plan?.title ?? "Plan"}</Dialog.Title>

          {/* Header */}
          <div data-slot="plan-viewer-header">
            <span data-slot="plan-viewer-title">{plan?.title}</span>
            <Dialog.Close asChild>
              <button type="button" data-slot="plan-viewer-close" aria-label="Close">
                <X className="h-4 w-4" />
              </button>
            </Dialog.Close>
          </div>

          {/* Body */}
          <div data-slot="plan-viewer-body">
            {plan && segments.map((seg, idx) =>
              seg.kind === "markdown" ? (
                <MarkdownRenderer
                  key={`md-${idx}`}
                  text={seg.content}
                  cacheKey={`plan-${plan.id}-${idx}`}
                />
              ) : (
                <MetaPills key={`meta-${idx}`} meta={seg.meta} />
              ),
            )}
          </div>

          {/* Footer */}
          <div data-slot="plan-viewer-footer">
            <button
              type="button"
              data-slot="plan-viewer-action"
              data-variant="close"
              onClick={onClose}
            >
              Close
            </button>
            <div data-slot="plan-viewer-spacer" />
            {onDispatch && plan && (
              <button
                type="button"
                data-slot="plan-viewer-action"
                data-variant="dispatch"
                disabled={isDispatching}
                onClick={() => onDispatch(plan.id)}
              >
                <Rocket className="h-3.5 w-3.5" />
                {isDispatching ? "Dispatching..." : "Dispatch to Drone"}
              </button>
            )}
          </div>
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
