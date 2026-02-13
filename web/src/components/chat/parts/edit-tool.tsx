import { FileEdit } from "lucide-react"
import { BasicTool } from "../basic-tool"
import { CodeBlock } from "../code-block"
import { DiffChanges } from "../diff-changes"
import type { ToolProps } from "../tool-registry"
import { registerTool } from "../tool-registry"

function EditTool({ input, output, status, hideDetails, defaultOpen, forceOpen, locked }: ToolProps) {
  const filePath = input.file_path as string | undefined
  const oldString = input.old_string as string | undefined
  const newString = input.new_string as string | undefined

  // Compute rough addition/deletion counts from the old/new strings
  const oldLines = oldString?.split("\n").length ?? 0
  const newLines = newString?.split("\n").length ?? 0
  const additions = Math.max(0, newLines - oldLines)
  const deletions = Math.max(0, oldLines - newLines)

  return (
    <BasicTool
      icon={<FileEdit />}
      status={status}
      trigger={{
        title: "Edit",
        subtitle: filePath,
        action:
          (additions > 0 || deletions > 0) ? (
            <DiffChanges additions={additions} deletions={deletions} />
          ) : undefined,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {output && (
        <CodeBlock code={output} language="diff" maxHeight={400} />
      )}
    </BasicTool>
  )
}

registerTool("Edit", EditTool)
