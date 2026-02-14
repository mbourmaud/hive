import { Eye } from "lucide-react";
import { BasicTool } from "../basic-tool";
import { CodeBlock } from "../code-block";
import { guessLanguage } from "../lang-utils";
import type { ToolProps } from "../tool-registry";
import { registerTool } from "../tool-registry";

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
      {output && <CodeBlock code={output} language={lang} startLine={startLine} maxHeight={400} />}
    </BasicTool>
  );
}

registerTool("Read", ReadTool);
