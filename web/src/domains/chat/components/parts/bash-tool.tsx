import Anser from "anser";
import DOMPurify from "dompurify";
import { Terminal } from "lucide-react";
import { useMemo } from "react";
import { BasicTool } from "../basic-tool";
import { CodeBlock } from "../code-block";
import type { ToolProps } from "../tool-registry";
import { registerTool } from "../tool-registry";

// biome-ignore lint/suspicious/noControlCharactersInRegex: ANSI escape detection requires matching ESC (\x1b)
const ANSI_ESCAPE_RE = /\x1b\[[0-9;]*m/;

function BashTool({
  input,
  output,
  status,
  hideDetails,
  defaultOpen,
  forceOpen,
  locked,
}: ToolProps) {
  const command = typeof input.command === "string" ? input.command : undefined;

  const hasAnsi = output ? ANSI_ESCAPE_RE.test(output) : false;

  const sanitizedHtml = useMemo(() => {
    if (!output || !hasAnsi) return "";
    const raw = Anser.ansiToHtml(output, { use_classes: true });
    return DOMPurify.sanitize(raw);
  }, [output, hasAnsi]);

  return (
    <BasicTool
      icon={<Terminal />}
      status={status}
      trigger={{
        title: "Shell",
        args: command ? [command] : undefined,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {output && hasAnsi ? (
        <pre
          data-slot="bash-ansi-output"
          className="overflow-auto p-3 text-xs font-mono leading-relaxed"
          style={{ maxHeight: 400 }}
          // biome-ignore lint/security/noDangerouslySetInnerHtml: sanitized with DOMPurify
          dangerouslySetInnerHTML={{ __html: sanitizedHtml }}
        />
      ) : (
        output && <CodeBlock code={output} language="bash" maxHeight={400} lineNumbers={false} />
      )}
    </BasicTool>
  );
}

registerTool("Bash", BashTool);
