import { DiCss3, DiNodejsSmall, DiPython, DiRust } from "react-icons/di";
import { SiGo, SiHtml5, SiJson, SiMarkdown, SiToml, SiTypescript, SiYaml } from "react-icons/si";
import { FileText } from "lucide-react";

// ── File extension → icon + color ────────────────────────────────────────────

interface FileStyle {
  icon: React.ComponentType<{ className?: string }>;
  color: string;
}

const FILE_STYLES: Record<string, FileStyle> = {
  rs:   { icon: DiRust,        color: "oklch(0.72 0.14 45)" },
  ts:   { icon: SiTypescript,  color: "oklch(0.65 0.14 250)" },
  tsx:  { icon: SiTypescript,  color: "oklch(0.65 0.14 250)" },
  js:   { icon: DiNodejsSmall, color: "oklch(0.75 0.14 90)" },
  jsx:  { icon: DiNodejsSmall, color: "oklch(0.75 0.14 90)" },
  css:  { icon: DiCss3,        color: "oklch(0.65 0.18 330)" },
  html: { icon: SiHtml5,       color: "oklch(0.68 0.16 25)" },
  json: { icon: SiJson,        color: "oklch(0.68 0.12 160)" },
  toml: { icon: SiToml,        color: "oklch(0.68 0.12 160)" },
  yaml: { icon: SiYaml,        color: "oklch(0.68 0.12 160)" },
  yml:  { icon: SiYaml,        color: "oklch(0.68 0.12 160)" },
  md:   { icon: SiMarkdown,    color: "var(--color-muted-foreground)" },
  py:   { icon: DiPython,      color: "oklch(0.72 0.14 230)" },
  go:   { icon: SiGo,          color: "oklch(0.72 0.14 200)" },
};

const DEFAULT_STYLE: FileStyle = {
  icon: FileText,
  color: "var(--color-muted-foreground)",
};

function getFileStyle(path: string): FileStyle {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return FILE_STYLES[ext] ?? DEFAULT_STYLE;
}

// ── Component ────────────────────────────────────────────────────────────────

interface PlanFilesListProps {
  files: string[];
}

export function PlanFilesList({ files }: PlanFilesListProps) {
  return (
    <div className="plan-meta-files-list">
      {files.map((filePath) => {
        const { icon: Icon, color } = getFileStyle(filePath);
        return (
          <div
            key={filePath}
            className="plan-meta-file-item"
            style={{ color } as React.CSSProperties}
          >
            <span className="plan-meta-file-icon">
              <Icon className="h-3.5 w-3.5" />
            </span>
            <code>{filePath}</code>
          </div>
        );
      })}
    </div>
  );
}
