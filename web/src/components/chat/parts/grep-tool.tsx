import { Search } from "lucide-react"
import { BasicTool } from "../basic-tool"
import { MarkdownRenderer } from "../markdown-renderer"
import type { ToolProps } from "../tool-registry"
import { registerTool } from "../tool-registry"

function GrepTool({ input, output, status, hideDetails, defaultOpen, forceOpen, locked }: ToolProps) {
  const pattern = input.pattern as string | undefined

  return (
    <BasicTool
      icon={<Search />}
      status={status}
      trigger={{
        title: "Grep",
        subtitle: pattern,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {output && (
        <MarkdownRenderer text={`\`\`\`\n${output}\n\`\`\``} />
      )}
    </BasicTool>
  )
}

registerTool("Grep", GrepTool)
