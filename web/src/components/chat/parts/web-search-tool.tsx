import { Search } from "lucide-react"
import { BasicTool } from "../basic-tool"
import { MarkdownRenderer } from "../markdown-renderer"
import type { ToolProps } from "../tool-registry"
import { registerTool } from "../tool-registry"

function WebSearchTool({ input, output, status, hideDetails, defaultOpen, forceOpen, locked }: ToolProps) {
  const query = input.query as string | undefined

  return (
    <BasicTool
      icon={<Search />}
      status={status}
      trigger={{
        title: "WebSearch",
        subtitle: query,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {output && <MarkdownRenderer text={output} />}
    </BasicTool>
  )
}

registerTool("WebSearch", WebSearchTool)
