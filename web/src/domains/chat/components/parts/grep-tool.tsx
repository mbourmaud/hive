import { Search } from "lucide-react";
import { useMemo } from "react";
import { BasicTool } from "../basic-tool";
import { CodeBlock } from "../code-block";
import type { ToolProps } from "../tool-registry";
import { registerTool } from "../tool-registry";

function GrepTool({
  input,
  output,
  status,
  hideDetails,
  defaultOpen,
  forceOpen,
  locked,
}: ToolProps) {
  const pattern = typeof input.pattern === "string" ? input.pattern : undefined;

  const { matchCount, fileCount } = useMemo(() => {
    if (!output) return { matchCount: 0, fileCount: 0 };
    const lines = output.split("\n").filter((l) => l.trim().length > 0);
    const files = new Set<string>();
    for (const line of lines) {
      // Grep output lines typically start with a file path followed by ":"
      const colonIdx = line.indexOf(":");
      if (colonIdx > 0) {
        files.add(line.slice(0, colonIdx));
      }
    }
    return { matchCount: lines.length, fileCount: files.size };
  }, [output]);

  const subtitle = pattern
    ? fileCount > 0
      ? `${pattern} (${matchCount} matches in ${fileCount} files)`
      : pattern
    : undefined;

  return (
    <BasicTool
      icon={<Search />}
      status={status}
      trigger={{
        title: "Grep",
        subtitle,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {output && <CodeBlock code={output} maxHeight={400} lineNumbers={false} />}
    </BasicTool>
  );
}

registerTool("Grep", GrepTool);
