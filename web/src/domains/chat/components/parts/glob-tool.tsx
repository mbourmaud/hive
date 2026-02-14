import { FolderSearch } from "lucide-react";
import { useMemo } from "react";
import { BasicTool } from "../basic-tool";
import type { ToolProps } from "../tool-registry";
import { registerTool } from "../tool-registry";

const MAX_DISPLAY_FILES = 50;

function GlobTool({
  input,
  output,
  status,
  hideDetails,
  defaultOpen,
  forceOpen,
  locked,
}: ToolProps) {
  const pattern = typeof input.pattern === "string" ? input.pattern : undefined;

  const files = useMemo(
    () => (output ? output.split("\n").filter((l) => l.trim().length > 0) : []),
    [output],
  );

  const displayFiles = files.slice(0, MAX_DISPLAY_FILES);
  const remaining = files.length - displayFiles.length;
  const subtitle = pattern
    ? files.length > 0
      ? `${pattern} (${files.length} files)`
      : pattern
    : undefined;

  return (
    <BasicTool
      icon={<FolderSearch />}
      status={status}
      trigger={{
        title: "Glob",
        subtitle,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {displayFiles.length > 0 && (
        <ul
          data-slot="glob-file-list"
          className="space-y-0.5 py-2 text-xs font-mono text-muted-foreground"
        >
          {displayFiles.map((file) => (
            <li key={file} className="truncate" title={file}>
              {file}
            </li>
          ))}
          {remaining > 0 && (
            <li className="text-muted-foreground/60 pt-1">... and {remaining} more</li>
          )}
        </ul>
      )}
    </BasicTool>
  );
}

registerTool("Glob", GlobTool);
