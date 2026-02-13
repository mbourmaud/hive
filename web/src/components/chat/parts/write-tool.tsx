import { FilePlus } from "lucide-react"
import { BasicTool } from "../basic-tool"
import { CodeBlock } from "../code-block"
import type { ToolProps } from "../tool-registry"
import { registerTool } from "../tool-registry"

function WriteTool({ input, output, status, hideDetails, defaultOpen, forceOpen, locked }: ToolProps) {
  const filePath = input.file_path as string | undefined

  return (
    <BasicTool
      icon={<FilePlus />}
      status={status}
      trigger={{
        title: "Write",
        subtitle: filePath,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {output && (
        <CodeBlock code={output} maxHeight={400} />
      )}
    </BasicTool>
  )
}

registerTool("Write", WriteTool)
