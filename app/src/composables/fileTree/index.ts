/**
 * File Tree Composables
 * Centralized exports for all file tree-related composables
 */

export { useFileTree } from './useFileTree';
export { useFileOperations } from './useFileOperations';
export { useFileSearch } from './useFileSearch';
export { useFileWatching } from './useFileWatching';

// Re-export types for convenience
export type {
  FileNode,
  FileTreeState,
  FileChangeEvent,
  FileChangeType,
  UseFileTreeReturn,
  UseFileOperationsReturn,
  UseFileSearchReturn,
  UseFileWatchingReturn,
  SearchOptions,
  SearchResult,
  SearchMatch,
} from '../../types/fileTree';
