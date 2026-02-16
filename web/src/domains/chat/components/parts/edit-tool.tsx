import { FileEdit } from "lucide-react";
import { BasicTool } from "../basic-tool";
import { CodeBlock } from "../code-block";
import { DiffChanges } from "../diff-changes";
import { SideBySideDiff } from "../side-by-side-diff";
import type { ToolProps } from "../tool-registry";
import { registerTool } from "../tool-registry";

function EditTool({
  input,
  output,
  status,
  hideDetails,
  defaultOpen,
  forceOpen,
  locked,
}: ToolProps) {
  const filePath = typeof input.file_path === "string" ? input.file_path : undefined;
  const oldString = typeof input.old_string === "string" ? input.old_string : undefined;
  const newString = typeof input.new_string === "string" ? input.new_string : undefined;

  const oldLines = oldString?.split("\n").length ?? 0;
  const newLines = newString?.split("\n").length ?? 0;
  const additions = Math.max(0, newLines - oldLines);
  const deletions = Math.max(0, oldLines - newLines);

  return (
    <BasicTool
      icon={<FileEdit />}
      status={status}
      trigger={{
        title: "Edit",
        subtitle: filePath,
        action:
          additions > 0 || deletions > 0 ? (
            <DiffChanges additions={additions} deletions={deletions} />
          ) : undefined,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {oldString !== undefined && newString !== undefined ? (
        <SideBySideDiff oldText={oldString} newText={newString} filePath={filePath} />
      ) : (
        output && <CodeBlock code={output} language="diff" maxHeight={400} />
      )}
    </BasicTool>
  );
}

registerTool("Edit", EditTool);
