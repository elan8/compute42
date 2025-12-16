/**
 * File Tree State Management Composable
 * Handles tree state, expansion, selection, and basic operations
 */

import { ref, computed, watch, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { debug, error } from '../../utils/logger';
import { createIoniconsFileIconComponent } from '../../utils/ioniconsFileIconUtils';
import type {
  FileNode,
  UseFileTreeReturn,
  FileTreeState,
  FileChangeEvent,
} from '../../types/fileTree';

export function useFileTree(initialPath?: string): UseFileTreeReturn {
  // ============================
  // State
  // ============================

  const fileTree = ref<FileNode[]>([]);
  const expandedKeys = ref<Set<string>>(new Set());
  const selectedKeys = ref<Set<string>>(new Set());
  const checkedKeys = ref<Set<string>>(new Set());
  const loading = ref<boolean>(false);
  const error = ref<string | null>(null);
  const rootPath = ref<string | null>(initialPath || null);

  // ============================
  // Computed Properties
  // ============================

  const treeState = computed<FileTreeState>(() => ({
    fileTree: fileTree.value,
    expandedKeys: expandedKeys.value,
    selectedKeys: selectedKeys.value,
    checkedKeys: checkedKeys.value,
    loading: loading.value,
    error: error.value,
    searchQuery: '',
    filteredTree: fileTree.value,
  }));

  // ============================
  // Utility Functions
  // ============================

  const findNodeByPath = (path: string): FileNode | null => {
    const searchInNodes = (nodes: FileNode[]): FileNode | null => {
      for (const node of nodes) {
        if (node.path === path) {
          return node;
        }
        if (node.children) {
          const found = searchInNodes(node.children);
          if (found) return found;
        }
      }
      return null;
    };

    return searchInNodes(fileTree.value);
  };

  const findNodeByKey = (key: string): FileNode | null => {
    debug(`useFileTree: findNodeByKey called with key: ${key}`);
    debug(`useFileTree: Searching in fileTree with ${fileTree.value.length} top-level nodes`);

    const searchInNodes = (nodes: FileNode[]): FileNode | null => {
      for (const node of nodes) {
        const nodeKey = getNodeKey(node);
        debug(
          `useFileTree: Checking node: ${node.name} (${node.path}) with key: ${nodeKey} against search key: ${key}`
        );
        if (nodeKey === key) {
          debug(`useFileTree: Found matching node: ${node.name} (${node.path})`);
          return node;
        }
        if (node.children) {
          debug(
            `useFileTree: Searching in children of ${node.name}, children count: ${node.children.length}`
          );
          const found = searchInNodes(node.children);
          if (found) return found;
        }
      }
      return null;
    };

    const result = searchInNodes(fileTree.value);
    debug(`useFileTree: findNodeByKey result:`, result);
    return result;
  };

  const getNodePath = (node: FileNode): string => {
    return node.path;
  };

  const getNodeKey = (node: FileNode): string => {
    return node.key || node.path;
  };

  // ============================
  // Tree Operations
  // ============================

  const expandNode = (key: string): void => {
    debug(`useFileTree: expandNode called with key: ${key}`);
    debug(`useFileTree: Current expandedKeys before adding:`, Array.from(expandedKeys.value));
    expandedKeys.value.add(key);
    debug(`useFileTree: Current expandedKeys after adding:`, Array.from(expandedKeys.value));

    // Load directory contents if needed
    const node = findNodeByKey(key);
    debug(`useFileTree: findNodeByKey(${key}) returned:`, node);

    if (node && node.is_directory) {
      debug(
        `useFileTree: Node found: ${node.name} (${node.path}), is_directory: ${node.is_directory}`
      );
      debug(`useFileTree: Node children:`, node.children);
      debug(`useFileTree: Children length: ${node.children ? node.children.length : 'null'}`);

      // Check if we need to load children (no children or empty children array)
      if (!node.children || node.children.length === 0) {
        debug(`useFileTree: No children found, loading directory contents for: ${node.path}`);
        loadDirectoryContents(node.path);
      } else {
        debug(
          `useFileTree: Directory ${node.path} already has ${node.children.length} children:`,
          node.children.map((c) => c.name)
        );
      }
    } else {
      debug(`useFileTree: Node ${key} not found or not a directory. Found node:`, node);
      if (node) {
        debug(`useFileTree: Node is_directory: ${node.is_directory}`);
      }
    }
  };

  const collapseNode = (key: string): void => {
    debug(`Collapsing node: ${key}`);
    expandedKeys.value.delete(key);
  };

  const selectNode = (key: string): void => {
    debug(`Selecting node: ${key}`);
    selectedKeys.value.clear();
    selectedKeys.value.add(key);
  };

  const checkNode = (key: string, checked: boolean): void => {
    debug(`Checking node: ${key}, checked: ${checked}`);
    if (checked) {
      checkedKeys.value.add(key);
    } else {
      checkedKeys.value.delete(key);
    }
  };

  // ============================
  // File Tree Loading
  // ============================

  const refreshTree = async (): Promise<void> => {
    if (!rootPath.value) {
      error('No root path set for file tree');
      return;
    }

    loading.value = true;
    error.value = null;

    try {
      debug(`Refreshing file tree for: ${rootPath.value}`);
      const tree = await invoke('get_file_tree', { rootPath: rootPath.value });

      if (tree && tree.is_directory && tree.children) {
        fileTree.value = mapNodesForTree(tree.children);
        debug(`Loaded ${fileTree.value.length} top-level items`);
      } else if (tree && tree.is_directory) {
        fileTree.value = [];
        debug('Root directory is empty');
      } else {
        fileTree.value = [];
        debug('Invalid tree structure received');
      }
    } catch (err) {
      const errorMessage = `Failed to load file tree: ${err}`;
      error(errorMessage);
      error.value = errorMessage;
    } finally {
      loading.value = false;
    }
  };

  const loadDirectoryContents = async (path: string): Promise<void> => {
    try {
      debug(`useFileTree: loadDirectoryContents called for: ${path}`);
      debug(`useFileTree: Current fileTree before loading:`, fileTree.value);

      const contents = await invoke('load_directory_contents', { path });
      debug(
        `useFileTree: Backend returned ${contents.length} items for directory: ${path}`,
        contents
      );

      // Find the target node and update its children
      const targetNode = findNodeByPath(path);
      debug(`useFileTree: findNodeByPath(${path}) returned:`, targetNode);

      if (targetNode && targetNode.is_directory) {
        debug(`useFileTree: Target node found, updating children`);
        debug(`useFileTree: Target node before update:`, targetNode);
        targetNode.children = mapNodesForTree(contents);
        debug(
          `useFileTree: Updated target node children. New children count: ${targetNode.children.length}`
        );
        debug(`useFileTree: Target node after update:`, targetNode);
        debug(`useFileTree: Current fileTree after loading:`, fileTree.value);
      } else {
        debug(`useFileTree: Target node not found or not a directory:`, targetNode);
      }
    } catch (err) {
      const errorMessage = `Failed to load directory contents: ${err}`;
      error(errorMessage);
      error.value = errorMessage;
      debug(`useFileTree: Error loading directory contents:`, err);
    }
  };

  // ============================
  // File System Change Handling
  // ============================

  const handleFileChange = (event: FileChangeEvent): void => {
    debug(`File change event: ${event.change_type} - ${event.path}`);

    // For now, just refresh the entire tree
    // In a more sophisticated implementation, we would update specific nodes
    if (rootPath.value) {
      refreshTree();
    }
  };

  // ============================
  // Helper Functions
  // ============================

  const mapNodesForTree = (nodes: any[]): FileNode[] => {
    if (!nodes) return [];

    return nodes.map((node) => ({
      ...node,
      key: node.path,
      label: node.name,
      isLeaf: !node.is_directory,
      prefix: createIoniconsFileIconComponent(node.name, node.is_directory),
      children: node.is_directory && node.children ? mapNodesForTree(node.children) : undefined,
    }));
  };

  // ============================
  // Watchers
  // ============================

  // Watch for root path changes
  watch(rootPath, (newPath) => {
    if (newPath) {
      refreshTree();
    } else {
      fileTree.value = [];
      expandedKeys.value.clear();
      selectedKeys.value.clear();
      checkedKeys.value.clear();
    }
  });

  // ============================
  // Public API
  // ============================

  return {
    // State
    fileTree,
    expandedKeys,
    selectedKeys,
    checkedKeys,
    loading,
    error,
    rootPath,

    // Actions
    expandNode,
    collapseNode,
    selectNode,
    checkNode,
    refreshTree,
    loadDirectoryContents,

    // Utilities
    findNodeByPath,
    findNodeByKey,
    getNodePath,
    getNodeKey,
  };
}
