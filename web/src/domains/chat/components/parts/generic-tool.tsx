import type { ToolProps } from "../tool-registry";
import { getToolComponent } from "../tool-registry";
import { BasicTool } from "../basic-tool";
import { CodeBlock } from "../code-block";
import { Wrench } from "lucide-react";

interface GenericToolProps extends ToolProps {
  toolName: string;
}

export function GenericTool({ toolName, ...props }: GenericToolProps) {
  const Registered = getToolComponent(toolName);
  if (Registered) return <Registered {...props} />;

  return (
    <BasicTool icon={Wrench} trigger={{ title: toolName }} status={props.status}>
      <CodeBlock code={props.output ?? JSON.stringify(props.input, null, 2)} language="json" />
    </BasicTool>
  );
}
