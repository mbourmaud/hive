import * as Collapsible from "@radix-ui/react-collapsible";
import { ChevronRight, Loader2 } from "lucide-react";
import { type ReactNode, useState } from "react";
import { cn } from "@/shared/lib/utils";
import type { ToolStatus } from "../types";
import "./basic-tool.css";

// ── Trigger title types ─────────────────────────────────────────────────────

export interface TriggerTitle {
  title: string;
  titleClass?: string;
  subtitle?: string;
  subtitleClass?: string;
  args?: string[];
  argsClass?: string;
  action?: ReactNode;
}

function isTriggerTitle(trigger: TriggerTitle | ReactNode): trigger is TriggerTitle {
  return (
    trigger !== null &&
    typeof trigger === "object" &&
    "title" in trigger &&
    typeof trigger.title === "string"
  );
}

// ── Props ───────────────────────────────────────────────────────────────────

interface BasicToolProps {
  icon: ReactNode;
  status?: ToolStatus;
  trigger: TriggerTitle | ReactNode;
  children?: ReactNode;
  hideDetails?: boolean;
  defaultOpen?: boolean;
  forceOpen?: boolean;
  locked?: boolean;
}

// ── Component ───────────────────────────────────────────────────────────────

export function BasicTool({
  icon,
  status = "completed",
  trigger,
  children,
  hideDetails = false,
  defaultOpen = false,
  forceOpen = false,
  locked = false,
}: BasicToolProps) {
  const [open, setOpen] = useState(defaultOpen || forceOpen);
  const isRunning = status === "running" || status === "pending";
  const hasContent = !hideDetails && children != null;

  const handleOpenChange = (value: boolean) => {
    if (locked || forceOpen) return;
    setOpen(value);
  };

  const effectiveOpen = forceOpen ? true : open;

  return (
    <Collapsible.Root
      data-component="basic-tool"
      data-status={status}
      open={effectiveOpen}
      onOpenChange={handleOpenChange}
      disabled={!hasContent}
    >
      <Collapsible.Trigger asChild disabled={!hasContent}>
        <button
          data-slot="basic-tool-trigger"
          type="button"
          className={cn(!hasContent && "cursor-default")}
        >
          <span data-slot="basic-tool-icon">
            {isRunning ? <Loader2 className="h-4 w-4 animate-spin" /> : icon}
          </span>

          <span data-slot="basic-tool-info">
            {isTriggerTitle(trigger) ? <TriggerTitleView {...trigger} /> : trigger}
          </span>

          {hasContent && (
            <ChevronRight
              data-slot="basic-tool-chevron"
              className={cn(
                "h-3.5 w-3.5 shrink-0 text-muted-foreground transition-transform duration-150",
                effectiveOpen && "rotate-90",
              )}
            />
          )}
        </button>
      </Collapsible.Trigger>

      {hasContent && (
        <Collapsible.Content data-slot="basic-tool-content">
          <div data-slot="basic-tool-body">{children}</div>
        </Collapsible.Content>
      )}
    </Collapsible.Root>
  );
}

// ── Trigger title sub-component ─────────────────────────────────────────────

function TriggerTitleView({
  title,
  titleClass,
  subtitle,
  subtitleClass,
  args,
  argsClass,
  action,
}: TriggerTitle) {
  return (
    <>
      <span data-slot="basic-tool-title" className={cn("font-medium", titleClass)}>
        {title}
      </span>
      {subtitle && (
        <span
          data-slot="basic-tool-subtitle"
          className={cn("truncate text-muted-foreground", subtitleClass)}
          title={subtitle}
        >
          {subtitle}
        </span>
      )}
      {args?.map((arg) => (
        <span
          key={arg}
          data-slot="basic-tool-arg"
          className={cn("truncate font-mono text-muted-foreground", argsClass)}
          title={arg}
        >
          {arg}
        </span>
      ))}
      {action && (
        <span data-slot="basic-tool-action" className="ml-auto shrink-0">
          {action}
        </span>
      )}
    </>
  );
}
