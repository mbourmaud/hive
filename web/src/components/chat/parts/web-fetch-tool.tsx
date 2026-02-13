import { Globe } from "lucide-react"
import { BasicTool } from "../basic-tool"
import { MarkdownRenderer } from "../markdown-renderer"
import type { ToolProps } from "../tool-registry"
import { registerTool } from "../tool-registry"

function WebFetchTool({ input, output, status, hideDetails, defaultOpen, forceOpen, locked }: ToolProps) {
  const url = input.url as string | undefined

  return (
    <BasicTool
      icon={<Globe />}
      status={status}
      trigger={{
        title: "WebFetch",
        subtitle: url,
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

registerTool("WebFetch", WebFetchTool)
