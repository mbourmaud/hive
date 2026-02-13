import { Eye } from "lucide-react";
import { BasicTool } from "../basic-tool";
import { CodeBlock } from "../code-block";
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
  const filePath = input.file_path as string | undefined;

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
      {output && <CodeBlock code={output} language={guessLanguage(filePath)} maxHeight={400} />}
    </BasicTool>
  );
}

function guessLanguage(filePath?: string): string | undefined {
  if (!filePath) return undefined;
  const ext = filePath.split(".").pop()?.toLowerCase();
  const map: Record<string, string> = {
    ts: "typescript",
    tsx: "tsx",
    js: "javascript",
    jsx: "jsx",
    rs: "rust",
    py: "python",
    go: "go",
    json: "json",
    toml: "toml",
    yaml: "yaml",
    yml: "yaml",
    md: "markdown",
    css: "css",
    html: "html",
    sh: "bash",
    bash: "bash",
    zsh: "bash",
  };
  return ext ? map[ext] : undefined;
}

registerTool("Read", ReadTool);
