import { describe, it, expect, beforeEach } from 'vitest';
import { ref } from 'vue';
import { useFileSearch } from './useFileSearch';
import type { FileNode } from '../../types/fileTree';

describe('useFileSearch', () => {
  const createMockFileTree = (): FileNode[] => [
    {
      name: 'file1.jl',
      path: '/path/file1.jl',
      is_directory: false,
      key: '/path/file1.jl',
      label: 'file1.jl',
      isLeaf: true,
    },
    {
      name: 'file2.ts',
      path: '/path/file2.ts',
      is_directory: false,
      key: '/path/file2.ts',
      label: 'file2.ts',
      isLeaf: true,
    },
    {
      name: 'folder1',
      path: '/path/folder1',
      is_directory: true,
      key: '/path/folder1',
      label: 'folder1',
      isLeaf: false,
      children: [
        {
          name: 'file3.jl',
          path: '/path/folder1/file3.jl',
          is_directory: false,
          key: '/path/folder1/file3.jl',
          label: 'file3.jl',
          isLeaf: true,
        },
      ],
    },
  ];

  describe('initialization', () => {
    it('should initialize with empty state', () => {
      const fileTree = ref<FileNode[]>([]);
      const { searchQuery, searchResults, isSearching } = useFileSearch(fileTree);

      expect(searchQuery.value).toBe('');
      expect(searchResults.value).toEqual([]);
      expect(isSearching.value).toBe(false);
    });
  });

  describe('search', () => {
    it('should find matching files', async () => {
      const fileTree = ref(createMockFileTree());
      const { search, searchResults } = useFileSearch(fileTree);

      await search('file1');

      expect(searchResults.value.length).toBeGreaterThan(0);
      expect(searchResults.value[0].node.name).toContain('file1');
    });

    it('should return empty results for empty query', async () => {
      const fileTree = ref(createMockFileTree());
      const { search, searchResults } = useFileSearch(fileTree);

      await search('');

      expect(searchResults.value).toEqual([]);
    });

    it('should be case-insensitive by default', async () => {
      const fileTree = ref(createMockFileTree());
      const { search, searchResults } = useFileSearch(fileTree);

      await search('FILE1');

      expect(searchResults.value.length).toBeGreaterThan(0);
    });

    it('should support case-sensitive search', async () => {
      const fileTree = ref(createMockFileTree());
      const { search, setSearchOptions, searchResults } = useFileSearch(fileTree);

      setSearchOptions({ caseSensitive: true });
      await search('FILE1');

      // With case-sensitive, FILE1 should not match file1
      expect(searchResults.value.length).toBe(0);
    });
  });

  describe('clearSearch', () => {
    it('should clear search query and results', async () => {
      const fileTree = ref(createMockFileTree());
      const { search, clearSearch, searchQuery, searchResults } = useFileSearch(fileTree);

      await search('file1');
      expect(searchQuery.value).toBe('file1');
      expect(searchResults.value.length).toBeGreaterThan(0);

      clearSearch();
      expect(searchQuery.value).toBe('');
      expect(searchResults.value).toEqual([]);
    });
  });

  describe('setSearchOptions', () => {
    it('should update search options', () => {
      const fileTree = ref<FileNode[]>([]);
      const { setSearchOptions, searchOptions } = useFileSearch(fileTree);

      setSearchOptions({ caseSensitive: true, wholeWord: true });
      expect(searchOptions.value.caseSensitive).toBe(true);
      expect(searchOptions.value.wholeWord).toBe(true);
    });
  });

  describe('filteredTree', () => {
    it('should return full tree when no search query', () => {
      const fileTree = ref(createMockFileTree());
      const { filteredTree } = useFileSearch(fileTree);

      expect(filteredTree.value.length).toBe(fileTree.value.length);
    });

    it('should filter tree based on search query', async () => {
      const fileTree = ref(createMockFileTree());
      const { search, filteredTree } = useFileSearch(fileTree);

      await search('file1');
      // filteredTree should contain matching nodes
      expect(filteredTree.value.length).toBeGreaterThan(0);
    });
  });

  describe('highlightMatches', () => {
    it('should return matches for a node', async () => {
      const fileTree = ref(createMockFileTree());
      const { search, highlightMatches } = useFileSearch(fileTree);

      await search('file1');
      const matches = highlightMatches(fileTree.value[0]);

      expect(matches.length).toBeGreaterThan(0);
    });

    it('should return empty array when no search query', () => {
      const fileTree = ref(createMockFileTree());
      const { highlightMatches } = useFileSearch(fileTree);

      const matches = highlightMatches(fileTree.value[0]);
      expect(matches).toEqual([]);
    });
  });
});


