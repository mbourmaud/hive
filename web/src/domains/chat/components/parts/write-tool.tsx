import { FilePlus } from "lucide-react";
import { BasicTool } from "../basic-tool";
import { CodeBlock } from "../code-block";
import { guessLanguage } from "../lang-utils";
import type { ToolProps } from "../tool-registry";
import { registerTool } from "../tool-registry";

function WriteTool({
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

  return (
    <BasicTool
      icon={<FilePlus />}
      status={status}
      trigger={{
        title: "Write",
        subtitle: filePath,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {output && <CodeBlock code={output} language={lang} maxHeight={400} />}
    </BasicTool>
  );
}

registerTool("Write", WriteTool);
