/**
 * Comprehensive TypeScript types for file tree operations
 * Based on VS Code file tree architecture design
 */

// ============================
// Core File Node Types
// ============================

export interface FileNode {
  path: string;
  name: string;
  is_directory: boolean;
  is_leaf: boolean;
  children?: FileNode[];
  size?: number;
  modified_time?: number;
  git_status?: GitStatus;
  decorations?: FileDecoration[];
  // Additional properties for tree functionality
  key?: string;
  label?: string;
  prefix?: () => any; // Vue component for file icon
  disabled?: boolean;
  selectable?: boolean;
  checkable?: boolean;
  indeterminate?: boolean;
  checked?: boolean;
  expanded?: boolean;
  selected?: boolean;
  loading?: boolean;
}

// ============================
// File Decorations
// ============================

export interface FileDecoration {
  icon: string;
  color: string;
  tooltip?: string;
  position: 'before' | 'after';
  weight?: number;
}

// ============================
// Git Status
// ============================

export interface GitStatus {
  status: 'added' | 'modified' | 'deleted' | 'renamed' | 'untracked' | 'ignored';
  staged: boolean;
  conflict?: boolean;
}

// ============================
// File Change Events
// ============================

export interface FileChangeEvent {
  path: string;
  change_type: FileChangeType;
  timestamp: number;
}

export type FileChangeType = 'created' | 'modified' | 'deleted' | 'renamed';

// ============================
// File Tree State
// ============================

export interface FileTreeState {
  fileTree: FileNode[];
  expandedKeys: Set<string>;
  selectedKeys: Set<string>;
  checkedKeys: Set<string>;
  loading: boolean;
  error: string | null;
  searchQuery: string;
  filteredTree: FileNode[];
}

// ============================
// File Operations
// ============================

export interface FileOperation {
  type: 'create' | 'delete' | 'rename' | 'move' | 'copy';
  path: string;
  newPath?: string;
  content?: string;
}

export interface FileOperationResult {
  success: boolean;
  error?: string;
  data?: any;
}

// ============================
// Context Menu
// ============================

export interface ContextMenuAction {
  id: string;
  label: string;
  icon?: string;
  disabled?: boolean;
  separator?: boolean;
  children?: ContextMenuAction[];
}

export interface ContextMenuState {
  visible: boolean;
  x: number;
  y: number;
  target: FileNode | null;
  actions: ContextMenuAction[];
}

// ============================
// Search and Filter
// ============================

export interface SearchOptions {
  query: string;
  caseSensitive: boolean;
  wholeWord: boolean;
  regex: boolean;
  includeFiles: boolean;
  includeFolders: boolean;
  excludePatterns: string[];
}

export interface SearchResult {
  node: FileNode;
  matches: SearchMatch[];
  score: number;
}

export interface SearchMatch {
  start: number;
  end: number;
  text: string;
}

// ============================
// Drag and Drop
// ============================

export interface DragDropState {
  isDragging: boolean;
  dragNode: FileNode | null;
  dropTarget: FileNode | null;
  dropPosition: 'before' | 'after' | 'inside';
}

// ============================
// Virtual Scrolling
// ============================

export interface VirtualScrollOptions {
  enabled: boolean;
  itemHeight: number;
  bufferSize: number;
  overscan: number;
}

export interface VirtualScrollState {
  scrollTop: number;
  visibleStart: number;
  visibleEnd: number;
  totalHeight: number;
}

// ============================
// File Tree Configuration
// ============================

export interface FileTreeConfig {
  showHiddenFiles: boolean;
  showGitStatus: boolean;
  showFileIcons: boolean;
  showFileSize: boolean;
  showModifiedTime: boolean;
  sortBy: 'name' | 'size' | 'modified' | 'type';
  sortOrder: 'asc' | 'desc';
  groupBy: 'none' | 'type' | 'extension';
  virtualScrolling: VirtualScrollOptions;
  contextMenu: {
    enabled: boolean;
    actions: ContextMenuAction[];
  };
  dragDrop: {
    enabled: boolean;
    allowMove: boolean;
    allowCopy: boolean;
  };
  search: {
    enabled: boolean;
    options: SearchOptions;
  };
}

// ============================
// File Tree Events
// ============================

export interface FileTreeEvents {
  onNodeSelect: (node: FileNode) => void;
  onNodeExpand: (node: FileNode) => void;
  onNodeCollapse: (node: FileNode) => void;
  onNodeCheck: (node: FileNode, checked: boolean) => void;
  onNodeContextMenu: (node: FileNode | null, event: MouseEvent) => void;
  onNodeDoubleClick: (node: FileNode) => void;
  onNodeDragStart: (node: FileNode, event: DragEvent) => void;
  onNodeDragEnd: (node: FileNode, event: DragEvent) => void;
  onNodeDrop: (target: FileNode, source: FileNode, position: 'before' | 'after' | 'inside') => void;
  onFileChange: (event: FileChangeEvent) => void;
  onSearch: (query: string, results: SearchResult[]) => void;
  onError: (error: string) => void;
}

// ============================
// File Tree Props
// ============================

export interface FileTreeProps {
  data: FileNode[];
  config?: Partial<FileTreeConfig>;
  events?: Partial<FileTreeEvents>;
  className?: string;
  style?: Record<string, any>;
  loading?: boolean;
  error?: string | null;
}

// ============================
// Utility Types
// ============================

export type FileNodeKey = string;
export type FileNodePath = string;

export interface FileTreeUtils {
  findNodeByPath: (tree: FileNode[], path: string) => FileNode | null;
  findNodeByKey: (tree: FileNode[], key: string) => FileNode | null;
  getNodePath: (node: FileNode) => string;
  getNodeKey: (node: FileNode) => string;
  isNodeExpanded: (node: FileNode, expandedKeys: Set<string>) => boolean;
  isNodeSelected: (node: FileNode, selectedKeys: Set<string>) => boolean;
  isNodeChecked: (node: FileNode, checkedKeys: Set<string>) => boolean;
  getNodeIcon: (node: FileNode) => string;
  getNodeLabel: (node: FileNode) => string;
  getNodeTooltip: (node: FileNode) => string;
  canNodeBeExpanded: (node: FileNode) => boolean;
  canNodeBeSelected: (node: FileNode) => boolean;
  canNodeBeChecked: (node: FileNode) => boolean;
  canNodeBeDragged: (node: FileNode) => boolean;
  canNodeAcceptDrop: (node: FileNode, source: FileNode) => boolean;
}

// ============================
// File Tree Composable Return Types
// ============================

export interface UseFileTreeReturn {
  // State
  fileTree: Ref<FileNode[]>;
  expandedKeys: Ref<Set<string>>;
  selectedKeys: Ref<Set<string>>;
  checkedKeys: Ref<Set<string>>;
  loading: Ref<boolean>;
  error: Ref<string | null>;

  // Actions
  expandNode: (key: string) => void;
  collapseNode: (key: string) => void;
  selectNode: (key: string) => void;
  checkNode: (key: string, checked: boolean) => void;
  refreshTree: () => Promise<void>;
  loadDirectoryContents: (path: string) => Promise<void>;

  // Utilities
  findNodeByPath: (path: string) => FileNode | null;
  findNodeByKey: (key: string) => FileNode | null;
  getNodePath: (node: FileNode) => string;
  getNodeKey: (node: FileNode) => string;
}

export interface UseFileOperationsReturn {
  createFile: (path: string, content?: string) => Promise<FileOperationResult>;
  createFolder: (path: string) => Promise<FileOperationResult>;
  deleteItem: (path: string) => Promise<FileOperationResult>;
  renameItem: (oldPath: string, newPath: string) => Promise<FileOperationResult>;
  moveItem: (oldPath: string, newPath: string) => Promise<FileOperationResult>;
  copyItem: (oldPath: string, newPath: string) => Promise<FileOperationResult>;
  readFile: (path: string) => Promise<string>;
  writeFile: (path: string, content: string) => Promise<FileOperationResult>;
}

export interface UseFileSearchReturn {
  searchQuery: Ref<string>;
  searchResults: Ref<SearchResult[]>;
  isSearching: Ref<boolean>;
  searchOptions: Ref<SearchOptions>;

  search: (query: string) => Promise<void>;
  clearSearch: () => void;
  setSearchOptions: (options: Partial<SearchOptions>) => void;
  highlightMatches: (node: FileNode) => SearchMatch[];
}

export interface UseFileWatchingReturn {
  isWatching: Ref<boolean>;
  watcherId: Ref<string | null>;

  startWatching: (path: string, recursive?: boolean) => Promise<void>;
  stopWatching: () => Promise<void>;
  onFileChange: (callback: (event: FileChangeEvent) => void) => void;
}

// ============================
// Import Vue types for composables
// ============================

import type { Ref } from 'vue';
