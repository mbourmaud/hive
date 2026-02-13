import { Terminal } from "lucide-react"
import { BasicTool } from "../basic-tool"
import { CodeBlock } from "../code-block"
import type { ToolProps } from "../tool-registry"
import { registerTool } from "../tool-registry"

function BashTool({ input, output, status, hideDetails, defaultOpen, forceOpen, locked }: ToolProps) {
  const command = input.command as string | undefined

  return (
    <BasicTool
      icon={<Terminal />}
      status={status}
      trigger={{
        title: "Bash",
        args: command ? [command] : undefined,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {output && (
        <CodeBlock code={output} language="bash" maxHeight={400} lineNumbers={false} />
      )}
    </BasicTool>
  )
}

registerTool("Bash", BashTool)
