import { useState } from "react";
import { cn } from "@/lib/utils";
import { ChevronDown, FileEdit } from "lucide-react";
import { DiffViewer } from "../diff-viewer";

interface EditToolProps {
  filePath: string;
  oldString: string;
  newString: string;
}

export function EditTool({ filePath, oldString, newString }: EditToolProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  const additionCount = newString.split("\n").length - oldString.split("\n").length;
  const deletionCount = oldString.split("\n").length - newString.split("\n").length;

  return (
    <div className="rounded-lg border border-border overflow-hidden bg-card">
      {/* Header */}
      <button
        onClick={() => setIsExpanded(!isExpanded)}
        className={cn(
          "w-full flex items-center gap-2 px-3 py-2 bg-surface-inset hover:bg-muted/50 transition-colors text-left",
          "border-b border-border"
        )}
      >
        <FileEdit className="w-4 h-4 text-accent shrink-0" />
        <span className="flex-1 font-mono text-xs text-foreground truncate">{filePath}</span>
        <div className="flex items-center gap-2 shrink-0">
          {additionCount > 0 && (
            <span className="text-[11px] font-medium text-success">+{additionCount}</span>
          )}
          {deletionCount > 0 && (
            <span className="text-[11px] font-medium text-destructive">-{deletionCount}</span>
          )}
          <ChevronDown
            className={cn(
              "w-4 h-4 text-muted-foreground transition-transform",
              isExpanded && "rotate-180"
            )}
          />
        </div>
      </button>

      {/* Expanded diff view */}
      {isExpanded && (
        <div className="p-2">
          <DiffViewer
            oldString={oldString}
            newString={newString}
            fileName={filePath}
          />
        </div>
      )}

      {/* Collapsed summary */}
      {!isExpanded && (
        <div className="px-3 py-2 text-xs text-muted-foreground bg-surface-inset">
          Click to view diff
        </div>
      )}
    </div>
  );
}
