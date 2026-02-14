import { Eye } from "lucide-react";
import { BasicTool } from "../basic-tool";
import { CodeBlock } from "../code-block";
import { guessLanguage } from "../lang-utils";
import type { ToolProps } from "../tool-registry";
import { registerTool } from "../tool-registry";

/** Strip `cat -n` line-number prefix (e.g. "   149→\t" or "   149\t") from each line.
 *  Returns { code, firstLine } where firstLine is parsed from the first numbered line. */
function stripLineNumbers(raw: string): { code: string; firstLine: number } {
  const lines = raw.split("\n");
  const prefix = /^\s*(\d+)[→\t]/;
  const match = lines[0]?.match(prefix);
  const firstLine = match?.[1] ? Number.parseInt(match[1], 10) : 1;
  const code = lines.map((l) => l.replace(prefix, "")).join("\n");
  return { code, firstLine };
}

function ReadTool({
  input,
  output,
  status,
  hideDetails,
  defaultOpen,
  forceOpen,
  locked,
}: ToolProps) {
  const filePath = typeof input.file_path === "string" ? input.file_path : undefined;
  const lang = guessLanguage(filePath);
  const offset = typeof input.offset === "number" ? input.offset : undefined;
  const startLine = offset != null && offset > 0 ? offset : 1;
  const { code, firstLine } = output
    ? stripLineNumbers(output)
    : { code: "", firstLine: startLine };

  return (
    <BasicTool
      icon={<Eye />}
      status={status}
      trigger={{
        title: "Read",
        subtitle: filePath,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {output && <CodeBlock code={code} language={lang} startLine={firstLine} maxHeight={400} />}
    </BasicTool>
  );
}

registerTool("Read", ReadTool);
