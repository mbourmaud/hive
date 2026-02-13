import { useMemo } from "react";
import { structuredPatch } from "diff";
import type { StructuredPatchHunk } from "diff";
import "./diff-viewer.css";

interface DiffViewerProps {
  oldText: string;
  newText: string;
  filePath?: string;
  className?: string;
}

type LineType = "added" | "removed" | "context";

interface DiffLine {
  type: LineType;
  oldLineNumber: number | null;
  newLineNumber: number | null;
  content: string;
}

interface HunkSeparator {
  type: "separator";
  label: string;
}

type DiffRow = DiffLine | HunkSeparator;

function isSeparator(row: DiffRow): row is HunkSeparator {
  return row.type === "separator";
}

function buildRows(hunks: StructuredPatchHunk[]): DiffRow[] {
  const rows: DiffRow[] = [];

  for (let hunkIdx = 0; hunkIdx < hunks.length; hunkIdx++) {
    const hunk = hunks[hunkIdx]!;

    if (hunkIdx > 0) {
      rows.push({
        type: "separator",
        label: `@@ -${hunk.oldStart},${hunk.oldLines} +${hunk.newStart},${hunk.newLines} @@`,
      });
    }

    let oldLine = hunk.oldStart;
    let newLine = hunk.newStart;

    for (const line of hunk.lines) {
      const prefix = line[0];
      const content = line.slice(1);

      if (prefix === "+") {
        rows.push({
          type: "added",
          oldLineNumber: null,
          newLineNumber: newLine,
          content,
        });
        newLine++;
      } else if (prefix === "-") {
        rows.push({
          type: "removed",
          oldLineNumber: oldLine,
          newLineNumber: null,
          content,
        });
        oldLine++;
      } else {
        rows.push({
          type: "context",
          oldLineNumber: oldLine,
          newLineNumber: newLine,
          content,
        });
        oldLine++;
        newLine++;
      }
    }
  }

  return rows;
}

const GUTTER_CHARS: Record<LineType, string> = {
  added: "+",
  removed: "-",
  context: " ",
};

export function DiffViewer({
  oldText,
  newText,
  filePath,
  className,
}: DiffViewerProps) {
  const rows = useMemo(() => {
    const patch = structuredPatch(
      filePath ?? "a",
      filePath ?? "b",
      oldText,
      newText,
      undefined,
      undefined,
      { context: 3 }
    );
    return buildRows(patch.hunks);
  }, [oldText, newText, filePath]);

  if (rows.length === 0) {
    return (
      <div data-component="diff-viewer" className={className}>
        {filePath && <div data-slot="diff-header">{filePath}</div>}
        <div
          style={{
            padding: "12px 16px",
            color: "var(--muted-foreground)",
            fontSize: 12,
          }}
        >
          No changes
        </div>
      </div>
    );
  }

  return (
    <div data-component="diff-viewer" className={className}>
      {filePath && <div data-slot="diff-header">{filePath}</div>}
      <table data-slot="diff-table">
        <tbody>
          {rows.map((row, idx) => {
            if (isSeparator(row)) {
              return (
                <tr key={idx}>
                  <td colSpan={4} data-slot="diff-hunk-separator">
                    ...
                  </td>
                </tr>
              );
            }

            return (
              <tr key={idx} data-slot="diff-line" data-type={row.type}>
                <td data-slot="diff-line-number">
                  {row.oldLineNumber ?? ""}
                </td>
                <td data-slot="diff-line-number">
                  {row.newLineNumber ?? ""}
                </td>
                <td
                  data-slot="diff-gutter"
                  data-type={row.type !== "context" ? row.type : undefined}
                >
                  {GUTTER_CHARS[row.type]}
                </td>
                <td data-slot="diff-content">{row.content}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
