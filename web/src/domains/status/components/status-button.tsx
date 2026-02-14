import { forwardRef } from "react";
import { Activity } from "lucide-react";
import type { OverallHealth } from "../types";

interface StatusButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  health: OverallHealth;
}

export const StatusButton = forwardRef<HTMLButtonElement, StatusButtonProps>(
  function StatusButton({ health, ...props }, ref) {
    return (
      <button
        ref={ref}
        type="button"
        data-slot="icon-bar-footer-btn"
        data-component="status-button"
        title="System status"
        {...props}
      >
        <Activity className="h-4 w-4" />
        <span data-slot="status-health-dot" data-health={health} />
      </button>
    );
  },
);
