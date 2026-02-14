import "./tool.css";

import type { ToolResultPart } from "../../types";
import { DiffViewer } from "../diff-viewer";
import { guessLanguage } from "../lang-utils";
import { MarkdownRenderer } from "../markdown-renderer";

// ── Tool-specific expanded body renderers ───────────────────────────────────

export function BashToolBody({
  input,
  result,
}: {
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  const command = typeof input.command === "string" ? input.command : "";
  const content = result?.content ?? "";
  const truncated = content.length > 2000 ? `${content.slice(0, 2000)}\n... (truncated)` : content;

  return (
    <>
      {command && (
        <div data-slot="tool-body-command">
          <span data-slot="tool-body-command-prefix">$</span>
          <code>{command}</code>
        </div>
      )}
      {result && (
        <div data-slot="tool-body-result" data-error={result.isError || undefined}>
          <MarkdownRenderer text={`\`\`\`\n${truncated}\n\`\`\``} />
        </div>
      )}
    </>
  );
}

export function ReadToolBody({
  input,
  result,
}: {
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  const filePath =
    typeof input.file_path === "string"
      ? input.file_path
      : typeof input.path === "string"
        ? input.path
        : "";
  const lang = guessLanguage(filePath) ?? "";
  const content = result?.content ?? "";
  const truncated = content.length > 2000 ? `${content.slice(0, 2000)}\n... (truncated)` : content;

  return (
    <>
      {filePath && (
        <div data-slot="tool-body-filepath">
          <code>{filePath}</code>
        </div>
      )}
      {result && (
        <div data-slot="tool-body-result" data-error={result.isError || undefined}>
          <MarkdownRenderer text={`\`\`\`${lang}\n${truncated}\n\`\`\``} />
        </div>
      )}
    </>
  );
}

export function EditToolBody({
  input,
  result,
}: {
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  const filePath =
    typeof input.file_path === "string"
      ? input.file_path
      : typeof input.path === "string"
        ? input.path
        : "";
  const oldString = typeof input.old_string === "string" ? input.old_string : undefined;
  const newString = typeof input.new_string === "string" ? input.new_string : undefined;

  return (
    <>
      {filePath && (
        <div data-slot="tool-body-filepath">
          <code>{filePath}</code>
        </div>
      )}
      {oldString !== undefined && newString !== undefined ? (
        <DiffViewer oldText={oldString} newText={newString} filePath={filePath || undefined} />
      ) : (
        result && (
          <div data-slot="tool-body-result" data-error={result.isError || undefined}>
            <pre>
              <code>
                {result.content.length > 2000
                  ? `${result.content.slice(0, 2000)}\n... (truncated)`
                  : result.content}
              </code>
            </pre>
          </div>
        )
      )}
    </>
  );
}

export function FileListToolBody({
  input,
  result,
}: {
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  const pattern =
    typeof input.pattern === "string"
      ? input.pattern
      : typeof input.query === "string"
        ? input.query
        : "";
  const content = result?.content ?? "";
  const lines = content.split("\n").filter((l) => l.trim().length > 0);
  const displayLines = lines.slice(0, 50);
  const remaining = lines.length - displayLines.length;

  return (
    <>
      {pattern && (
        <div data-slot="tool-body-filepath">
          <code>{pattern}</code>
        </div>
      )}
      {result && (
        <div data-slot="tool-body-result" data-error={result.isError || undefined}>
          <div data-slot="tool-body-filelist">
            {displayLines.map((line) => (
              <div key={line} data-slot="tool-body-filelist-item">
                {line}
              </div>
            ))}
            {remaining > 0 && (
              <div data-slot="tool-body-filelist-more">... and {remaining} more</div>
            )}
          </div>
        </div>
      )}
    </>
  );
}

export function TaskToolBody({
  input,
  result,
}: {
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  const subject =
    typeof input.subject === "string"
      ? input.subject
      : typeof input.description === "string"
        ? input.description
        : "";
  const content = result?.content ?? "";
  const truncated = content.length > 500 ? `${content.slice(0, 500)}...` : content;

  return (
    <>
      {subject && (
        <div data-slot="tool-body-task-label">
          <span data-slot="tool-body-task-tree">&#x2514;</span>
          <span>{subject.length > 100 ? `${subject.slice(0, 100)}...` : subject}</span>
        </div>
      )}
      {result && truncated && (
        <div data-slot="tool-body-result" data-error={result.isError || undefined}>
          <pre>
            <code>{truncated}</code>
          </pre>
        </div>
      )}
    </>
  );
}

export function DefaultToolBody({
  input,
  result,
}: {
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  return (
    <>
      <pre>
        <code>{JSON.stringify(input, null, 2)}</code>
      </pre>
      {result && (
        <div data-slot="tool-body-result" data-error={result.isError || undefined}>
          <pre>
            <code>
              {result.content.length > 2000
                ? `${result.content.slice(0, 2000)}\n... (truncated)`
                : result.content}
            </code>
          </pre>
        </div>
      )}
    </>
  );
}

export function ToolExpandedBody({
  name,
  input,
  result,
}: {
  name: string;
  input: Record<string, unknown>;
  result: ToolResultPart | undefined;
}) {
  const lower = name.toLowerCase();

  if (lower === "bash" || lower === "execute" || lower === "run") {
    return <BashToolBody input={input} result={result} />;
  }

  if (lower === "read" || lower === "readfile" || lower === "view") {
    return <ReadToolBody input={input} result={result} />;
  }

  if (lower === "edit" || lower === "write" || lower === "writefile") {
    return <EditToolBody input={input} result={result} />;
  }

  if (lower === "glob" || lower === "grep" || lower === "search") {
    return <FileListToolBody input={input} result={result} />;
  }

  if (lower === "task" || lower === "sendmessage" || lower === "delegate") {
    return <TaskToolBody input={input} result={result} />;
  }

  return <DefaultToolBody input={input} result={result} />;
}
