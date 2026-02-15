import type { MemberInfo, TaskInfo } from "@/domains/monitor/types";
import { agentColor } from "@/shared/constants";
import "./task-list.css";

// ── Status helpers ────────────────────────────────────────────────────────────

type TaskStatus = "completed" | "in_progress" | "blocked" | "pending";

function resolveStatus(task: TaskInfo): TaskStatus {
  if (task.blocked_by) return "blocked";
  if (task.status === "completed") return "completed";
  if (task.status === "in_progress") return "in_progress";
  return "pending";
}

// ── Component ─────────────────────────────────────────────────────────────────

interface TaskListProps {
  tasks: TaskInfo[];
  members: MemberInfo[];
}

export function TaskList({ tasks, members }: TaskListProps) {
  if (tasks.length === 0) {
    return <div className="text-xs text-muted-foreground">No tasks</div>;
  }

  return (
    <div data-component="task-list">
      {tasks.map((task) => (
        <TaskRow key={task.id} task={task} members={members} />
      ))}
    </div>
  );
}

// ── Task row ──────────────────────────────────────────────────────────────────

function TaskRow({ task, members }: { task: TaskInfo; members: MemberInfo[] }) {
  const status = resolveStatus(task);
  const ownerIdx = members.findIndex((m) => m.name === task.owner);
  const ownerBg = ownerIdx >= 0 ? agentColor(ownerIdx + 1) : undefined;

  // Extract "US-NNN" prefix for the badge, rest for the label
  const usMatch = task.subject.match(/^(US-\d+):\s*(.*)/);
  const badge = usMatch?.[1] ?? null;
  const label = usMatch?.[2] ?? task.subject;

  return (
    <div data-slot="task-row" data-status={status}>
      <span data-slot="task-dot" />
      {badge && <span data-slot="task-badge">{badge}</span>}
      <span data-slot="task-label" title={task.subject}>
        {label}
      </span>
      {task.owner && (
        <span
          data-slot="task-owner"
          style={ownerBg ? ({ "--owner-color": ownerBg } as React.CSSProperties) : undefined}
          title={task.owner}
        >
          {task.owner}
        </span>
      )}
    </div>
  );
}
