import { Layers } from "lucide-react";
import { BasicTool } from "../basic-tool";
import type { ToolProps } from "../tool-registry";
import { registerTool } from "../tool-registry";

function TaskTool({
  input,
  output,
  status,
  hideDetails,
  defaultOpen,
  forceOpen,
  locked,
}: ToolProps) {
  const description = input.description as string | undefined;

  return (
    <BasicTool
      icon={<Layers />}
      status={status}
      trigger={{
        title: "Task",
        subtitle: description,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {output && <p className="py-2 text-xs text-muted-foreground whitespace-pre-wrap">{output}</p>}
    </BasicTool>
  );
}

registerTool("Task", TaskTool);
