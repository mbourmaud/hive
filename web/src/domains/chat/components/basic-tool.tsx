import { type ReactNode, useState, useEffect } from "react";
import * as Collapsible from "@radix-ui/react-collapsible";
import { ChevronDown, Loader2, Copy, RotateCcw, ExternalLink, type LucideIcon } from "lucide-react";
import type { ToolStatus } from "./tool-registry";
import "./basic-tool.css";

interface TriggerTitle {
  title: string;
  subtitle?: string;
  args?: string[];
  action?: ReactNode;
}

interface BasicToolProps {
  icon: LucideIcon;
  trigger: TriggerTitle;
  status?: ToolStatus;
  children?: ReactNode;
  hideDetails?: boolean;
  defaultOpen?: boolean;
  forceOpen?: boolean;
  locked?: boolean;
  onCopy?: () => void;
  onRetry?: () => void;
  onOpen?: () => void;
}

export function BasicTool({
  icon: Icon,
  trigger,
  status = "completed",
  children,
  hideDetails = false,
  defaultOpen = false,
  forceOpen = false,
  locked = false,
  onCopy,
  onRetry,
  onOpen,
}: BasicToolProps) {
  const [isOpen, setIsOpen] = useState(defaultOpen || forceOpen);

  useEffect(() => {
    if (forceOpen) {
      setIsOpen(true);
    }
  }, [forceOpen]);

  const canToggle = !locked && !forceOpen && !hideDetails && children;
  const showChevron = canToggle;

  const handleOpenChange = (open: boolean) => {
    if (canToggle) {
      setIsOpen(open);
    }
  };

  const isRunning = status === "running";

  return (
    <Collapsible.Root
      className={`basic-tool ${status}`}
      open={isOpen}
      onOpenChange={handleOpenChange}
    >
      <Collapsible.Trigger
        className="basic-tool-trigger"
        disabled={!canToggle}
        asChild
      >
        <button type="button">
          <div className="basic-tool-icon">
            {isRunning ? (
              <Loader2 className="animate-spin" size={16} />
            ) : (
              <Icon size={16} />
            )}
          </div>
          <div className="basic-tool-content">
            <div className="basic-tool-title">
              <span className="basic-tool-title-text">{trigger.title}</span>
              {trigger.args && trigger.args.length > 0 && (
                <span className="basic-tool-args">
                  {trigger.args.map((arg, i) => (
                    <span key={i} className="basic-tool-arg">
                      {arg}
                    </span>
                  ))}
                </span>
              )}
            </div>
            {trigger.subtitle && (
              <div className="basic-tool-subtitle">{trigger.subtitle}</div>
            )}
          </div>
          {trigger.action && (
            <div className="basic-tool-action">{trigger.action}</div>
          )}
          <div className="basic-tool-actions">
            {onCopy && (
              <button
                type="button"
                className="basic-tool-action-btn"
                onClick={(e) => {
                  e.stopPropagation();
                  onCopy();
                }}
                title="Copy"
              >
                <Copy size={14} />
              </button>
            )}
            {onRetry && (
              <button
                type="button"
                className="basic-tool-action-btn"
                onClick={(e) => {
                  e.stopPropagation();
                  onRetry();
                }}
                title="Retry"
              >
                <RotateCcw size={14} />
              </button>
            )}
            {onOpen && (
              <button
                type="button"
                className="basic-tool-action-btn"
                onClick={(e) => {
                  e.stopPropagation();
                  onOpen();
                }}
                title="Open"
              >
                <ExternalLink size={14} />
              </button>
            )}
          </div>
          {showChevron && (
            <ChevronDown
              className={`basic-tool-chevron ${isOpen ? "open" : ""}`}
              size={16}
            />
          )}
        </button>
      </Collapsible.Trigger>
      {!hideDetails && children && (
        <Collapsible.Content className="basic-tool-collapsible-content">
          <div className="basic-tool-details">{children}</div>
        </Collapsible.Content>
      )}
    </Collapsible.Root>
  );
}
