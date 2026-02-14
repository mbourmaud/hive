import { useState } from "react";
import {
  File,
  FileCode,
  FileJson,
  FileText,
  Folder,
  FolderOpen,
  Search,
} from "lucide-react";
import { cn } from "@/lib/utils";

interface GlobToolProps {
  pattern: string;
  files: string[];
}

interface TreeNode {
  name: string;
  path: string;
  isDirectory: boolean;
  children: TreeNode[];
  fileCount?: number;
}

function getFileIcon(fileName: string) {
  const ext = fileName.split(".").pop()?.toLowerCase();
  switch (ext) {
    case "json":
      return FileJson;
    case "ts":
    case "tsx":
    case "js":
    case "jsx":
    case "rs":
    case "py":
    case "go":
      return FileCode;
    case "md":
    case "txt":
      return FileText;
    default:
      return File;
  }
}

function buildTree(files: string[]): TreeNode {
  const root: TreeNode = {
    name: "",
    path: "",
    isDirectory: true,
    children: [],
  };

  for (const filePath of files) {
    const parts = filePath.split("/").filter(Boolean);
    let current = root;

    parts.forEach((part, index) => {
      const isLastPart = index === parts.length - 1;
      let child = current.children.find((c) => c.name === part);

      if (!child) {
        child = {
          name: part,
          path: parts.slice(0, index + 1).join("/"),
          isDirectory: !isLastPart,
          children: [],
        };
        current.children.push(child);
      }

      current = child;
    });
  }

  // Calculate file counts for directories
  function calculateFileCounts(node: TreeNode): number {
    if (!node.isDirectory) {
      return 1;
    }
    const count = node.children.reduce((sum, child) => {
      return sum + calculateFileCounts(child);
    }, 0);
    node.fileCount = count;
    return count;
  }

  calculateFileCounts(root);

  // Sort: directories first, then alphabetically
  function sortChildren(node: TreeNode) {
    node.children.sort((a, b) => {
      if (a.isDirectory && !b.isDirectory) return -1;
      if (!a.isDirectory && b.isDirectory) return 1;
      return a.name.localeCompare(b.name);
    });
    node.children.forEach(sortChildren);
  }

  sortChildren(root);

  return root;
}

function TreeNodeView({
  node,
  level = 0,
}: {
  node: TreeNode;
  level?: number;
}) {
  const [isOpen, setIsOpen] = useState(level < 2);

  if (!node.isDirectory) {
    const Icon = getFileIcon(node.name);
    return (
      <div
        className="flex items-center gap-2 py-1 px-2 hover:bg-muted/50 rounded text-sm"
        style={{ paddingLeft: `${level * 16 + 8}px` }}
      >
        <Icon className="w-4 h-4 text-muted-foreground shrink-0" />
        <span className="text-foreground truncate">{node.name}</span>
      </div>
    );
  }

  const Icon = isOpen ? FolderOpen : Folder;

  return (
    <div>
      <div
        className="flex items-center gap-2 py-1 px-2 hover:bg-muted/50 rounded cursor-pointer text-sm select-none"
        style={{ paddingLeft: `${level * 16 + 8}px` }}
        onClick={() => setIsOpen(!isOpen)}
      >
        <Icon className="w-4 h-4 text-accent shrink-0" />
        <span className="text-foreground font-medium truncate">
          {node.name}
        </span>
        {node.fileCount !== undefined && node.fileCount > 0 && (
          <span className="text-xs text-muted-foreground">
            ({node.fileCount})
          </span>
        )}
      </div>
      {isOpen && (
        <div className={cn(level > 0 && "border-l border-border/50 ml-2")}>
          {node.children.map((child) => (
            <TreeNodeView key={child.path} node={child} level={level + 1} />
          ))}
        </div>
      )}
    </div>
  );
}

export function GlobTool({ pattern, files }: GlobToolProps) {
  const tree = buildTree(files);
  const totalFiles = files.length;

  return (
    <div className="border border-border rounded-lg overflow-hidden bg-card">
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-2 bg-card-header border-b border-border">
        <Search className="w-4 h-4 text-muted-foreground shrink-0" />
        <div className="flex-1 min-w-0">
          <div className="text-sm font-medium text-foreground">
            Pattern: <code className="font-mono text-accent">{pattern}</code>
          </div>
          <div className="text-xs text-muted-foreground mt-0.5">
            {totalFiles} {totalFiles === 1 ? "file" : "files"} matched
          </div>
        </div>
      </div>

      {/* File tree */}
      <div className="p-2 max-h-96 overflow-y-auto bg-surface-inset">
        {tree.children.length > 0 ? (
          tree.children.map((child) => (
            <TreeNodeView key={child.path} node={child} />
          ))
        ) : (
          <div className="text-sm text-muted-foreground text-center py-4">
            No files matched
          </div>
        )}
      </div>
    </div>
  );
}
