import { History, Search } from "lucide-react";
import { BasicTool } from "../basic-tool";
import { MarkdownRenderer } from "../markdown-renderer";
import type { ToolProps } from "../tool-registry";
import { registerTool } from "../tool-registry";

function SessionSearchTool({
  input,
  output,
  status,
  hideDetails,
  defaultOpen,
  forceOpen,
  locked,
}: ToolProps) {
  const query = input.query as string | undefined;

  return (
    <BasicTool
      icon={<Search />}
      status={status}
      trigger={{
        title: "SessionSearch",
        subtitle: query,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {output && <MarkdownRenderer text={output} />}
    </BasicTool>
  );
}

function RecentSessionsTool({
  input,
  output,
  status,
  hideDetails,
  defaultOpen,
  forceOpen,
  locked,
}: ToolProps) {
  const limit = input.limit as number | undefined;

  return (
    <BasicTool
      icon={<History />}
      status={status}
      trigger={{
        title: "RecentSessions",
        subtitle: limit ? `last ${limit}` : undefined,
      }}
      hideDetails={hideDetails}
      defaultOpen={defaultOpen}
      forceOpen={forceOpen}
      locked={locked}
    >
      {output && <MarkdownRenderer text={output} />}
    </BasicTool>
  );
}

registerTool("SessionSearch", SessionSearchTool);
registerTool("RecentSessions", RecentSessionsTool);
