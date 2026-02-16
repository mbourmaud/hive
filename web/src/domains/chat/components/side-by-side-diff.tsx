import type { StructuredPatchHunk } from "diff";
import { structuredPatch } from "diff";
import { useEffect, useMemo, useState } from "react";
import { getHighlighter, getThemeName, resolveLanguage } from "@/shared/lib/shiki-highlighter";
import { guessLanguage } from "./lang-utils";
import "./side-by-side-diff.css";

// ── Types ────────────────────────────────────────────────────────────────────

type DiffRowType = "context" | "added" | "removed" | "modified";

interface DiffRow {
  type: DiffRowType;
  leftLine: number | null;
  leftContent: string | null;
  rightLine: number | null;
  rightContent: string | null;
}

interface HunkSeparator {
  type: "separator";
  label: string;
}

type RowEntry = DiffRow | HunkSeparator;

interface KeyedRow {
  key: string;
  row: RowEntry;
}

interface NumberedLine {
  line: number;
  content: string;
}

interface SideBySideDiffProps {
  oldText: string;
  newText: string;
  filePath?: string;
}

// ── Row builder helpers ──────────────────────────────────────────────────────

function pairChanges(removals: NumberedLine[], additions: NumberedLine[]): KeyedRow[] {
  const paired: KeyedRow[] = [];
  const maxLen = Math.max(removals.length, additions.length);

  for (let j = 0; j < maxLen; j++) {
    const rem = removals[j];
    const add = additions[j];

    if (rem && add) {
      paired.push({
        key: `mod-${rem.line}-${add.line}`,
        row: {
          type: "modified",
          leftLine: rem.line,
          leftContent: rem.content,
          rightLine: add.line,
          rightContent: add.content,
        },
      });
    } else if (rem) {
      paired.push({
        key: `del-${rem.line}`,
        row: {
          type: "removed",
          leftLine: rem.line,
          leftContent: rem.content,
          rightLine: null,
          rightContent: null,
        },
      });
    } else if (add) {
      paired.push({
        key: `add-${add.line}`,
        row: {
          type: "added",
          leftLine: null,
          leftContent: null,
          rightLine: add.line,
          rightContent: add.content,
        },
      });
    }
  }

  return paired;
}

function processHunk(hunk: StructuredPatchHunk): KeyedRow[] {
  const rows: KeyedRow[] = [];
  let oldLine = hunk.oldStart;
  let newLine = hunk.newStart;
  const { lines } = hunk;
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];
    if (!line) {
      i++;
      continue;
    }

    const prefix = line[0];

    if (prefix === "-") {
      const removals: NumberedLine[] = [];
      while (i < lines.length && lines[i]?.[0] === "-") {
        removals.push({ line: oldLine, content: lines[i]?.slice(1) ?? "" });
        oldLine++;
        i++;
      }
      const additions: NumberedLine[] = [];
      while (i < lines.length && lines[i]?.[0] === "+") {
        additions.push({ line: newLine, content: lines[i]?.slice(1) ?? "" });
        newLine++;
        i++;
      }
      rows.push(...pairChanges(removals, additions));
    } else if (prefix === "+") {
      rows.push({
        key: `add-${newLine}`,
        row: {
          type: "added",
          leftLine: null,
          leftContent: null,
          rightLine: newLine,
          rightContent: line.slice(1),
        },
      });
      newLine++;
      i++;
    } else {
      rows.push({
        key: `ctx-${oldLine}-${newLine}`,
        row: {
          type: "context",
          leftLine: oldLine,
          leftContent: line.slice(1),
          rightLine: newLine,
          rightContent: line.slice(1),
        },
      });
      oldLine++;
      newLine++;
      i++;
    }
  }

  return rows;
}

function buildSideBySideRows(hunks: StructuredPatchHunk[]): KeyedRow[] {
  const rows: KeyedRow[] = [];

  for (let hunkIdx = 0; hunkIdx < hunks.length; hunkIdx++) {
    const hunk = hunks[hunkIdx];
    if (!hunk) continue;

    if (hunkIdx > 0) {
      rows.push({
        key: `sep-${hunkIdx}`,
        row: { type: "separator", label: "..." },
      });
    }

    rows.push(...processHunk(hunk));
  }

  return rows;
}

// ── Shiki highlighting hook ──────────────────────────────────────────────────

function useHighlightedLines(text: string, lang: string | undefined): string[] | null {
  const [highlighted, setHighlighted] = useState<string[] | null>(null);
  const resolved = resolveLanguage(lang);

  useEffect(() => {
    if (resolved === "text") return;
    let cancelled = false;

    (async () => {
      try {
        const hl = await getHighlighter();
        if (cancelled) return;
        const theme = getThemeName();
        const result = text.split("\n").map((line) => {
          const html = hl.codeToHtml(line || " ", { lang: resolved, theme });
          const match = html.match(/<code[^>]*>([\s\S]*?)<\/code>/);
          return match ? (match[1] ?? line) : line;
        });
        if (!cancelled) setHighlighted(result);
      } catch {
        // Shiki failed — keep plain text
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [text, resolved]);

  return highlighted;
}

// ── Line component ───────────────────────────────────────────────────────────

function DiffLine({
  lineNumber,
  content,
  highlighted,
  dataType,
}: {
  lineNumber: number | null;
  content: string | null;
  highlighted: string | null;
  dataType: string;
}) {
  if (content === null) {
    return (
      <div data-slot="sbs-line" data-type="empty">
        <span data-slot="sbs-line-number" />
        <span data-slot="sbs-content">{"\u00A0"}</span>
      </div>
    );
  }

  return (
    <div data-slot="sbs-line" data-type={dataType}>
      <span data-slot="sbs-line-number">{lineNumber ?? ""}</span>
      {highlighted ? (
        // biome-ignore lint/security/noDangerouslySetInnerHtml: Shiki-generated syntax HTML
        <span data-slot="sbs-content" dangerouslySetInnerHTML={{ __html: highlighted }} />
      ) : (
        <span data-slot="sbs-content">{content}</span>
      )}
    </div>
  );
}

// ── Panel renderer ───────────────────────────────────────────────────────────

function DiffPanel({
  rows,
  side,
  getHighlighted,
}: {
  rows: KeyedRow[];
  side: "left" | "right";
  getHighlighted: (lineNum: number | null) => string | null;
}) {
  return (
    <div data-slot="sbs-panel" data-side={side}>
      {rows.map(({ key, row }) => {
        if (row.type === "separator") {
          return (
            <div key={key} data-slot="sbs-separator">
              {row.label}
            </div>
          );
        }
        const isLeft = side === "left";
        const lineNumber = isLeft ? row.leftLine : row.rightLine;
        const content = isLeft ? row.leftContent : row.rightContent;
        const modType = isLeft ? "modified-old" : "modified-new";
        const dataType = row.type === "modified" ? modType : row.type;

        return (
          <DiffLine
            key={`${side[0]}-${key}`}
            lineNumber={lineNumber}
            content={content}
            highlighted={getHighlighted(lineNumber)}
            dataType={dataType}
          />
        );
      })}
    </div>
  );
}

// ── Main component ───────────────────────────────────────────────────────────

export function SideBySideDiff({ oldText, newText, filePath }: SideBySideDiffProps) {
  const lang = guessLanguage(filePath);
  const highlightedOld = useHighlightedLines(oldText, lang);
  const highlightedNew = useHighlightedLines(newText, lang);

  const rows = useMemo(() => {
    const patch = structuredPatch(
      filePath ?? "a",
      filePath ?? "b",
      oldText,
      newText,
      undefined,
      undefined,
      { context: 3 },
    );
    return buildSideBySideRows(patch.hunks);
  }, [oldText, newText, filePath]);

  if (rows.length === 0) {
    return (
      <div data-component="side-by-side-diff">
        {filePath && <div data-slot="sbs-header">{filePath}</div>}
        <div data-slot="sbs-empty-state">No changes</div>
      </div>
    );
  }

  function getHighlightedOld(lineNum: number | null): string | null {
    if (!highlightedOld || lineNum === null) return null;
    return highlightedOld[lineNum - 1] ?? null;
  }

  function getHighlightedNew(lineNum: number | null): string | null {
    if (!highlightedNew || lineNum === null) return null;
    return highlightedNew[lineNum - 1] ?? null;
  }

  return (
    <div data-component="side-by-side-diff">
      {filePath && <div data-slot="sbs-header">{filePath}</div>}
      <div data-slot="sbs-grid">
        <DiffPanel rows={rows} side="left" getHighlighted={getHighlightedOld} />
        <div data-slot="sbs-divider" />
        <DiffPanel rows={rows} side="right" getHighlighted={getHighlightedNew} />
      </div>
    </div>
  );
}
