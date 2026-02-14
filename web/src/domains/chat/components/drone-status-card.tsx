import { Activity, CheckCircle2, Loader2, Users, XCircle } from "lucide-react";
import { useEffect, useState } from "react";
import type { DroneInfo } from "@/domains/monitor/types";

// ── Types ────────────────────────────────────────────────────────────────────

interface DroneStatusCardProps {
  droneName: string;
  prompt: string;
}

// ── Type guard ────────────────────────────────────────────────────────────────

function isDroneInfo(data: unknown): data is DroneInfo {
  return (
    typeof data === "object" &&
    data !== null &&
    "name" in data &&
    "liveness" in data &&
    "tasks" in data &&
    "members" in data &&
    "progress" in data
  );
}

// ── Helpers ──────────────────────────────────────────────────────────────────

function isTerminalLiveness(liveness: string): boolean {
  return liveness !== "working" && liveness !== "starting";
}

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : "Unknown error";
}

// ── Hook: drone polling ──────────────────────────────────────────────────────

function useDronePolling(droneName: string): {
  drone: DroneInfo | null;
  error: string | null;
} {
  const [drone, setDrone] = useState<DroneInfo | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    let intervalId: ReturnType<typeof setInterval> | null = null;

    async function fetchDrone() {
      try {
        const res = await fetch(`/api/drones/${droneName}`);
        if (!res.ok) {
          if (res.status !== 404) setError(`Failed to fetch drone: ${res.statusText}`);
          return;
        }
        const data: unknown = await res.json();
        if (cancelled || !isDroneInfo(data)) return;

        setDrone(data);
        setError(null);
        if (isTerminalLiveness(data.liveness) && intervalId) {
          clearInterval(intervalId);
        }
      } catch (err) {
        if (!cancelled) setError(errorMessage(err));
      }
    }

    fetchDrone();
    intervalId = setInterval(fetchDrone, 3000);

    return () => {
      cancelled = true;
      if (intervalId) clearInterval(intervalId);
    };
  }, [droneName]);

  return { drone, error };
}

// ── Component ────────────────────────────────────────────────────────────────

export function DroneStatusCard({ droneName, prompt }: DroneStatusCardProps) {
  const { drone, error } = useDronePolling(droneName);

  const isActive = drone?.liveness === "working" || drone?.liveness === "starting";
  const isDone = drone !== null && !isActive;
  const completedTasks = drone?.tasks.filter((t) => t.status === "completed").length ?? 0;
  const totalTasks = drone?.tasks.length ?? 0;

  return (
    <div data-component="drone-status-card" data-active={isActive || undefined}>
      {/* Header */}
      <div data-slot="drone-card-header">
        <div data-slot="drone-card-header-left">
          {isActive ? (
            <Loader2 className="h-4 w-4 animate-spin text-accent" />
          ) : isDone ? (
            <CheckCircle2 className="h-4 w-4 text-success" />
          ) : (
            <Activity className="h-4 w-4 text-muted-foreground" />
          )}
          <span data-slot="drone-card-name">{droneName}</span>
          {drone && (
            <span data-slot="drone-card-liveness" data-liveness={drone.liveness}>
              {drone.liveness}
            </span>
          )}
        </div>
        {drone && <span data-slot="drone-card-elapsed">{drone.elapsed}</span>}
      </div>

      {/* Prompt */}
      <p data-slot="drone-card-prompt">
        {prompt.length > 120 ? `${prompt.slice(0, 120)}\u2026` : prompt}
      </p>

      {/* Progress */}
      {drone && totalTasks > 0 && (
        <div data-slot="drone-card-progress">
          <div data-slot="drone-card-progress-bar">
            <div
              data-slot="drone-card-progress-fill"
              style={{ width: `${totalTasks > 0 ? (completedTasks / totalTasks) * 100 : 0}%` }}
            />
          </div>
          <span data-slot="drone-card-progress-label">
            {completedTasks}/{totalTasks} tasks
          </span>
        </div>
      )}

      {/* Task list */}
      {drone && drone.tasks.length > 0 && (
        <div data-slot="drone-card-tasks">
          {drone.tasks.map((task) => (
            <div key={task.id} data-slot="drone-card-task" data-status={task.status}>
              {task.status === "completed" ? (
                <CheckCircle2 className="h-3 w-3 text-success shrink-0" />
              ) : task.status === "in_progress" ? (
                <Loader2 className="h-3 w-3 animate-spin text-accent shrink-0" />
              ) : (
                <XCircle className="h-3 w-3 text-muted-foreground shrink-0 opacity-40" />
              )}
              <span>{task.subject}</span>
              {task.owner && <span data-slot="drone-card-task-owner">{task.owner}</span>}
            </div>
          ))}
        </div>
      )}

      {/* Agents */}
      {drone && drone.members.length > 0 && (
        <div data-slot="drone-card-agents">
          <Users className="h-3 w-3 text-muted-foreground shrink-0" />
          <span>
            {drone.members.length} agent{drone.members.length !== 1 ? "s" : ""}
          </span>
          {drone.cost.total_usd > 0 && (
            <span data-slot="drone-card-cost">${drone.cost.total_usd.toFixed(2)}</span>
          )}
        </div>
      )}

      {/* Error */}
      {error && <div data-slot="drone-card-error">{error}</div>}
    </div>
  );
}
