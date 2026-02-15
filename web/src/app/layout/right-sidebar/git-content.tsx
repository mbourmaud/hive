import { GitBranch, Plus, Minus, FileQuestion, FilePen, FileX, FilePlus } from "lucide-react";
import { useCallback, useState } from "react";
import type { ChangedFile, FileStatus } from "@/domains/git/types";
import { useGitStatus, useFileDiff } from "@/domains/git/use-git-status";
import { useAppStore } from "@/store";
import { DiffViewer } from "@/domains/chat/components/diff-viewer";

// ── File status icons ────────────────────────────────────────────────────────

const STATUS_CONFIG: Record<FileStatus, { icon: typeof FilePen; color: string; label: string }> = {
  modified: { icon: FilePen, color: "oklch(0.72 0.19 250)", label: "M" },
  added: { icon: FilePlus, color: "oklch(0.72 0.19 142)", label: "A" },
  deleted: { icon: FileX, color: "oklch(0.65 0.2 25)", label: "D" },
  renamed: { icon: FilePen, color: "oklch(0.72 0.15 80)", label: "R" },
  copied: { icon: FilePlus, color: "oklch(0.72 0.15 80)", label: "C" },
  untracked: { icon: FileQuestion, color: "var(--color-muted-foreground)", label: "?" },
};

// ── Component ────────────────────────────────────────────────────────────────

export function GitContent() {
  const selectedProject = useAppStore((s) => s.selectedProject);
  const { data: status, isLoading } = useGitStatus(selectedProject);
  const [selectedFile, setSelectedFile] = useState<{ path: string; staged: boolean } | null>(null);

  const { data: diffData } = useFileDiff(
    selectedProject,
    selectedFile?.path ?? null,
    selectedFile?.staged ?? false,
  );

  const handleFileClick = useCallback((path: string, staged: boolean) => {
    setSelectedFile((prev) =>
      prev?.path === path && prev.staged === staged ? null : { path, staged },
    );
  }, []);

  if (!selectedProject) {
    return <div className="p-4 text-xs text-muted-foreground">Select a project to view git status.</div>;
  }

  if (isLoading) {
    return <div className="p-4 text-xs text-muted-foreground">Loading git status...</div>;
  }

  if (!status) {
    return <div className="p-4 text-xs text-muted-foreground">Could not load git status.</div>;
  }

  const hasChanges = status.staged.length > 0 || status.unstaged.length > 0 || status.untracked.length > 0;

  return (
    <div className="flex flex-col flex-1 overflow-hidden">
      {/* Branch header */}
      <div className="flex items-center gap-2 px-3 py-2.5 border-b border-sidebar-border">
        <GitBranch className="h-3.5 w-3.5 text-accent" />
        <span className="text-xs font-semibold truncate">{status.branch}</span>
        {status.ahead > 0 && (
          <span className="text-[10px] font-bold text-success">+{status.ahead}</span>
        )}
        {status.behind > 0 && (
          <span className="text-[10px] font-bold text-warning">-{status.behind}</span>
        )}
      </div>

      {/* File list */}
      <div className="flex-1 overflow-y-auto">
        {!hasChanges && (
          <div className="p-4 text-xs text-muted-foreground">Working tree clean</div>
        )}

        {status.staged.length > 0 && (
          <FileSection
            title="Staged"
            files={status.staged}
            staged
            selectedPath={selectedFile?.staged ? selectedFile.path : null}
            onFileClick={handleFileClick}
          />
        )}

        {status.unstaged.length > 0 && (
          <FileSection
            title="Changes"
            files={status.unstaged}
            staged={false}
            selectedPath={!selectedFile?.staged ? selectedFile?.path ?? null : null}
            onFileClick={handleFileClick}
          />
        )}

        {status.untracked.length > 0 && (
          <FileSection
            title="Untracked"
            files={status.untracked}
            staged={false}
            selectedPath={null}
            onFileClick={handleFileClick}
          />
        )}
      </div>

      {/* Inline diff */}
      {selectedFile && diffData && (
        <div className="border-t border-sidebar-border max-h-[40%] overflow-y-auto">
          <DiffViewer
            oldText={diffData.old_content}
            newText={diffData.new_content}
            filePath={selectedFile.path}
          />
        </div>
      )}
    </div>
  );
}

// ── File section ─────────────────────────────────────────────────────────────

function FileSection({ title, files, staged, selectedPath, onFileClick }: {
  title: string;
  files: ChangedFile[];
  staged: boolean;
  selectedPath: string | null;
  onFileClick: (path: string, staged: boolean) => void;
}) {
  return (
    <div>
      <div className="px-3 py-1.5 text-[10px] font-medium uppercase tracking-wide text-muted-foreground">
        {title} ({files.length})
      </div>
      {files.map((file) => {
        const cfg = STATUS_CONFIG[file.status];
        const Icon = cfg.icon;
        const isSelected = selectedPath === file.path;
        return (
          <button
            key={`${file.path}-${staged}`}
            type="button"
            className={`flex items-center gap-2 w-full px-3 py-1 text-left hover:bg-muted/50 transition-colors ${isSelected ? "bg-accent/8" : ""}`}
            onClick={() => onFileClick(file.path, staged)}
          >
            <Icon className="h-3 w-3 shrink-0" style={{ color: cfg.color }} />
            <span className="text-xs truncate flex-1">{file.path.split("/").pop()}</span>
            <span className="flex items-center gap-1 text-[10px] shrink-0">
              {file.additions > 0 && (
                <span className="text-success flex items-center gap-0.5">
                  <Plus className="h-2.5 w-2.5" />{file.additions}
                </span>
              )}
              {file.deletions > 0 && (
                <span className="text-destructive flex items-center gap-0.5">
                  <Minus className="h-2.5 w-2.5" />{file.deletions}
                </span>
              )}
            </span>
          </button>
        );
      })}
    </div>
  );
}
