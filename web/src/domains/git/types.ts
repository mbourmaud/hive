export type FileStatus = "modified" | "added" | "deleted" | "renamed" | "copied" | "untracked";

export interface ChangedFile {
  path: string;
  status: FileStatus;
  additions: number;
  deletions: number;
}

export interface PrSummary {
  number: number;
  title: string;
  url: string;
  state: string;
  is_draft: boolean;
}

export interface GitStatus {
  branch: string;
  base_branch: string | null;
  remote_url: string;
  platform: string;
  ahead: number;
  behind: number;
  staged: ChangedFile[];
  unstaged: ChangedFile[];
  untracked: ChangedFile[];
  open_pr: PrSummary | null;
  last_commit: string | null;
}

export interface FileDiff {
  path: string;
  diff: string;
  old_content: string;
  new_content: string;
}
