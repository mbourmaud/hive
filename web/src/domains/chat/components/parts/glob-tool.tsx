import { FolderSearch } from "lucide-react";
import { BasicTool } from "../basic-tool";
import type { ToolProps } from "../tool-registry";
import { registerTool } from "../tool-registry";

function GlobTool({
  input,
  output,
  status,
  hideDetails,
  defaultOpen,
  forceOpen,
  locked,
}: ToolProps) {
  const pattern = input.pattern as string | undefined;

  const files = output ? output.split("\n").filter((line) => line.trim().length > 0) : [];

  return (
    <BasicTool
      icon={<FolderSearch />}
      status={status}
      trigger={{
        title: "Glob",
        subtitle: pattern,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {files.length > 0 && (
        <ul
          data-slot="glob-file-list"
          className="space-y-0.5 py-2 text-xs font-mono text-muted-foreground"
        >
          {files.map((file) => (
            <li key={file} className="truncate" title={file}>
              {file}
            </li>
          ))}
        </ul>
      )}
    </BasicTool>
  );
}

registerTool("Glob", GlobTool);
