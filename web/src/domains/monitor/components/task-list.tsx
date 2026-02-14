import type { MemberInfo, TaskInfo } from "@/domains/monitor/types";
import { agentColor } from "@/shared/constants";

interface TaskListProps {
  tasks: TaskInfo[];
  members: MemberInfo[];
}

function statusIcon(status: string) {
  switch (status) {
    case "completed":
      return "\u2713";
    case "in_progress":
      return "\u25CB";
    default:
      return "\u00B7";
  }
}

function statusColor(status: string) {
  switch (status) {
    case "completed":
      return "text-success";
    case "in_progress":
      return "text-accent";
    default:
      return "text-muted-foreground";
  }
}

export function TaskList({ tasks, members }: TaskListProps) {
  if (tasks.length === 0) {
    return <div className="text-sm text-muted-foreground">No tasks</div>;
  }

  return (
    <div className="flex flex-col gap-1">
      {tasks.map((task) => {
        const ownerIdx = members.findIndex((m) => m.name === task.owner);
        const ownerColor = ownerIdx >= 0 ? agentColor(ownerIdx + 1) : agentColor(1);

        return (
          <div key={task.id} className="flex items-start gap-3 py-1.5">
            <div
              className={`mt-0.5 w-5 text-center text-sm font-bold shrink-0 ${statusColor(task.status)}`}
            >
              {statusIcon(task.status)}
            </div>
            <div className="flex-1 min-w-0">
              <div
                className={`text-sm text-foreground ${task.status === "in_progress" ? "font-semibold" : "opacity-70"}`}
              >
                {task.subject}
              </div>
              <div className="flex items-center gap-2 mt-0.5 flex-wrap">
                {task.owner && (
                  <span className="text-xs font-bold" style={{ color: ownerColor }}>
                    @{task.owner}
                  </span>
                )}
                {task.duration && (
                  <span className="text-xs text-muted-foreground">{task.duration}</span>
                )}
                {task.active_form && (
                  <span className="text-xs italic text-muted-foreground">{task.active_form}</span>
                )}
              </div>
            </div>
          </div>
        );
      })}
    </div>
  );
}
