import type { TaskInfo, MemberInfo } from "@/types/api";
import { agentColor } from "./constants";

interface TaskListProps {
  tasks: TaskInfo[];
  members: MemberInfo[];
}

function statusIcon(status: string) {
  switch (status) {
    case "completed": return "\u2713";
    case "in_progress": return "\u25CB";
    default: return "\u00B7";
  }
}

function statusColor(status: string) {
  switch (status) {
    case "completed": return "var(--green)";
    case "in_progress": return "var(--accent)";
    default: return "var(--text-muted)";
  }
}

export function TaskList({ tasks, members }: TaskListProps) {
  const userTasks = tasks.filter((t) => !t.is_internal);

  if (userTasks.length === 0) {
    return <div className="text-sm" style={{ color: "var(--text-muted)" }}>No tasks</div>;
  }

  return (
    <div className="flex flex-col gap-1">
      {userTasks.map((task) => {
        const ownerIdx = members.findIndex((m) => m.name === task.owner);
        const ownerColor = ownerIdx >= 0 ? agentColor(ownerIdx + 1) : agentColor(1);

        return (
          <div key={task.id} className="flex items-start gap-3 py-1.5">
            <div
              className="mt-0.5 w-5 text-center text-sm font-bold shrink-0"
              style={{ color: statusColor(task.status) }}
            >
              {statusIcon(task.status)}
            </div>
            <div className="flex-1 min-w-0">
              <div
                className="text-sm"
                style={{
                  color: task.status === "in_progress" ? "var(--text)" : "var(--text-muted)",
                  fontWeight: task.status === "in_progress" ? 500 : 400,
                }}
              >
                {task.subject}
              </div>
              <div className="flex items-center gap-2 mt-0.5 flex-wrap">
                {task.owner && (
                  <span className="text-xs font-medium" style={{ color: ownerColor }}>
                    @{task.owner}
                  </span>
                )}
                {task.duration && (
                  <span className="text-xs" style={{ color: "var(--text-muted)" }}>
                    {task.duration}
                  </span>
                )}
                {task.active_form && (
                  <span className="text-xs italic" style={{ color: "var(--text-muted)" }}>
                    {task.active_form}
                  </span>
                )}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
