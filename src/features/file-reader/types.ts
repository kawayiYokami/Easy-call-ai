export type FileReaderFileKind = "markdown" | "code" | "image" | "audio" | "video" | "unsupported";

export type FileReaderFilePayload = {
  path: string;
  name: string;
  extension: string;
  kind: FileReaderFileKind | string;
  content: string;
  forcePlain?: boolean;
  virtualized?: boolean;
  totalLines?: number;
  blockLineCount?: number;
};

export type FileReaderFileBlockPayload = {
  path: string;
  startLine: number;
  endLine: number;
  content: string;
};

export type FileReaderDirectoryEntry = {
  path: string;
  name: string;
  isDirectory: boolean;
};

export type FileReaderDirectoryPayload = {
  path: string;
  name: string;
  entries: FileReaderDirectoryEntry[];
};

export type GitDiffTabSource = {
  workspacePath: string;
  path: string;
  staged: boolean;
  hash?: string;
  /** 未跟踪文件（git ??）：无 git diff 内容，直接打开文件本体 */
  untracked?: boolean;
  /** 当前使用的 -U 上下文行数；0 表示 git 默认(3)，点"展开后续"后递增 */
  context?: number;
};

export type FileTab = {
  path: string;
  title: string;
  extension: string;
  kind: FileReaderFileKind | string;
  content: string;
  rawMode: boolean;
  forcePlain: boolean;
  virtualized: boolean;
  totalLines: number;
  blockLineCount: number;
  loaded: boolean;
  loading: boolean;
  error: string;
  diffSource?: GitDiffTabSource;
};

export type VirtualCodeBlock = {
  key: string;
  path: string;
  startLine: number;
  endLine: number;
  lineCount: number;
};

export type DirectoryNode = {
  path: string;
  name: string;
  entries: FileReaderDirectoryEntry[];
  loaded: boolean;
  loading: boolean;
  error: string;
  expanded: boolean;
};

export type TreeRow =
  | { kind: "entry"; key: string; depth: number; entry: FileReaderDirectoryEntry }
  | { kind: "status"; key: string; depth: number; text: string };

export type FileReaderSessionState = {
  tabs?: string[];
  activePath?: string;
  directoryRootPath?: string;
  directoryTreeWidth?: number;
  /** 左侧栏模式：文件 / git，会话级记忆，下次自动恢复 */
  asideMode?: "files" | "git";
};

export type FileReaderWatchTarget = {
  path: string;
  kind: "file" | "directory";
};

export type FileReaderWatchEventPayload = {
  sessionId: string;
  path: string;
  kind: "file" | "directory";
};
