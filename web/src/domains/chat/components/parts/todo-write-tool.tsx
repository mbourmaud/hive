import { CheckSquare } from "lucide-react";
import { BasicTool } from "../basic-tool";
import type { ToolProps } from "../tool-registry";
import { registerTool } from "../tool-registry";

interface TodoItem {
  id?: string;
  content?: string;
  status?: "completed" | "in_progress" | "pending";
}

function TodoWriteTool({
  input,
  output,
  status,
  hideDetails,
  defaultOpen,
  forceOpen,
  locked,
}: ToolProps) {
  const todos = input.todos as TodoItem[] | undefined;
  const total = todos?.length ?? 0;
  const completed = todos?.filter((t) => t.status === "completed").length ?? 0;
  const subtitle = total > 0 ? `${completed}/${total} tasks` : undefined;

  return (
    <BasicTool
      icon={<CheckSquare />}
      status={status}
      trigger={{
        title: "Todos",
        subtitle,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {todos && todos.length > 0 && (
        <ul className="space-y-1 py-2">
          {todos.map((todo, i) => (
            <li key={todo.id ?? i} className="flex items-start gap-2 text-xs">
              <input
                type="checkbox"
                checked={todo.status === "completed"}
                readOnly
                className="mt-0.5 shrink-0"
              />
              <span className="text-muted-foreground">{todo.content ?? `Task ${i + 1}`}</span>
            </li>
          ))}
        </ul>
      )}
      {!todos && output && (
        <p className="py-2 text-xs text-muted-foreground whitespace-pre-wrap">{output}</p>
      )}
    </BasicTool>
  );
}

registerTool("TodoWrite", TodoWriteTool);
