import { Wrench } from "lucide-react";
import { BasicTool } from "../basic-tool";
import { CodeBlock } from "../code-block";
import type { ToolProps } from "../tool-registry";
import { getToolComponent } from "../tool-registry";

/**
 * Fallback renderer for tools without a dedicated renderer.
 * This is NOT registered in the registry — it's used as the default.
 */
export function GenericTool({ toolName, ...props }: ToolProps & { toolName: string }) {
  const { input, output, status, hideDetails, defaultOpen, forceOpen, locked } = props;

  // Check if there is a registered renderer first
  const Registered = getToolComponent(toolName);
  if (Registered) {
    return <Registered {...props} />;
  }

  return (
    <BasicTool
      icon={<Wrench />}
      status={status}
      trigger={{
        title: toolName,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      <CodeBlock
        code={output ?? JSON.stringify(input, null, 2)}
        language="json"
        maxHeight={300}
        lineNumbers={false}
      />
    </BasicTool>
  );
}
