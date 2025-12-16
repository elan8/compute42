import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useFileTree } from './useFileTree';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('../../utils/ioniconsFileIconUtils', () => ({
  createIoniconsFileIconComponent: vi.fn(() => 'icon'),
}));

describe('useFileTree', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('initialization', () => {
    it('should initialize with empty state', () => {
      const { fileTree, expandedKeys, selectedKeys, loading, error } = useFileTree();

      expect(fileTree.value).toEqual([]);
      expect(expandedKeys.value.size).toBe(0);
      expect(selectedKeys.value.size).toBe(0);
      expect(loading.value).toBe(false);
      expect(error.value).toBeNull();
    });

    it('should initialize with root path', () => {
      const { rootPath } = useFileTree('/path/to/root');
      expect(rootPath.value).toBe('/path/to/root');
    });
  });

  describe('expandNode', () => {
    it('should add node to expanded keys', () => {
      const { expandNode, expandedKeys } = useFileTree();

      expandNode('node1');
      expect(expandedKeys.value.has('node1')).toBe(true);
    });

    it('should load directory contents when expanding directory', async () => {
      const mockChildren = [
        { name: 'file1.jl', path: '/path/file1.jl', is_directory: false },
      ];

      (invoke as any).mockResolvedValue(mockChildren);

      const { expandNode, fileTree } = useFileTree();
      
      // First set up a directory node
      fileTree.value = [
        {
          name: 'dir',
          path: '/path/dir',
          is_directory: true,
          key: '/path/dir',
          label: 'dir',
          isLeaf: false,
          children: [],
        },
      ];

      expandNode('/path/dir');
      
      // Wait for async operation
      await new Promise((resolve) => setTimeout(resolve, 0));
      
      // The loadDirectoryContents should be called
      expect(invoke).toHaveBeenCalled();
    });
  });

  describe('collapseNode', () => {
    it('should remove node from expanded keys', () => {
      const { expandNode, collapseNode, expandedKeys } = useFileTree();

      expandNode('node1');
      expect(expandedKeys.value.has('node1')).toBe(true);

      collapseNode('node1');
      expect(expandedKeys.value.has('node1')).toBe(false);
    });
  });

  describe('selectNode', () => {
    it('should select a node and clear previous selection', () => {
      const { selectNode, selectedKeys } = useFileTree();

      selectNode('node1');
      expect(selectedKeys.value.has('node1')).toBe(true);
      expect(selectedKeys.value.size).toBe(1);

      selectNode('node2');
      expect(selectedKeys.value.has('node1')).toBe(false);
      expect(selectedKeys.value.has('node2')).toBe(true);
      expect(selectedKeys.value.size).toBe(1);
    });
  });

  describe('checkNode', () => {
    it('should check a node', () => {
      const { checkNode, checkedKeys } = useFileTree();

      checkNode('node1', true);
      expect(checkedKeys.value.has('node1')).toBe(true);
    });

    it('should uncheck a node', () => {
      const { checkNode, checkedKeys } = useFileTree();

      checkNode('node1', true);
      checkNode('node1', false);
      expect(checkedKeys.value.has('node1')).toBe(false);
    });
  });

  describe('refreshTree', () => {
    it('should load file tree from backend', async () => {
      const mockTree = {
        is_directory: true,
        children: [
          { name: 'file1.jl', path: '/path/file1.jl', is_directory: false },
          { name: 'dir1', path: '/path/dir1', is_directory: true, children: [] },
        ],
      };

      (invoke as any).mockResolvedValue(mockTree);

      const { refreshTree, fileTree, rootPath } = useFileTree('/path/to/root');
      rootPath.value = '/path/to/root';

      await refreshTree();

      expect(invoke).toHaveBeenCalledWith('get_file_tree', { rootPath: '/path/to/root' });
      expect(fileTree.value.length).toBeGreaterThan(0);
    });

    it('should handle errors', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const { refreshTree, error, rootPath } = useFileTree('/path/to/root');
      rootPath.value = '/path/to/root';

      await refreshTree();

      expect(error.value).toBeDefined();
    });

    it('should not refresh if no root path', async () => {
      const { refreshTree } = useFileTree();
      // rootPath is null by default

      await refreshTree();
      expect(invoke).not.toHaveBeenCalled();
    });
  });

  describe('loadDirectoryContents', () => {
    it('should load directory contents', async () => {
      const mockContents = [
        { name: 'file1.jl', path: '/path/dir/file1.jl', is_directory: false },
      ];

      (invoke as any).mockResolvedValue(mockContents);

      const { loadDirectoryContents, fileTree } = useFileTree();
      
      // Set up a directory node
      fileTree.value = [
        {
          name: 'dir',
          path: '/path/dir',
          is_directory: true,
          key: '/path/dir',
          label: 'dir',
          isLeaf: false,
          children: [],
        },
      ];

      await loadDirectoryContents('/path/dir');

      expect(invoke).toHaveBeenCalledWith('load_directory_contents', { path: '/path/dir' });
    });
  });

  describe('findNodeByPath', () => {
    it('should find node by path', () => {
      const { fileTree, findNodeByPath } = useFileTree();

      fileTree.value = [
        {
          name: 'file1.jl',
          path: '/path/file1.jl',
          is_directory: false,
          key: '/path/file1.jl',
          label: 'file1.jl',
          isLeaf: true,
        },
      ];

      const node = findNodeByPath('/path/file1.jl');
      expect(node).toBeDefined();
      expect(node?.path).toBe('/path/file1.jl');
    });

    it('should return null for non-existent path', () => {
      const { findNodeByPath } = useFileTree();
      const node = findNodeByPath('/nonexistent');
      expect(node).toBeNull();
    });
  });

  describe('findNodeByKey', () => {
    it('should find node by key', () => {
      const { fileTree, findNodeByKey } = useFileTree();

      fileTree.value = [
        {
          name: 'file1.jl',
          path: '/path/file1.jl',
          is_directory: false,
          key: '/path/file1.jl',
          label: 'file1.jl',
          isLeaf: true,
        },
      ];

      const node = findNodeByKey('/path/file1.jl');
      expect(node).toBeDefined();
      expect(node?.key).toBe('/path/file1.jl');
    });
  });
});


