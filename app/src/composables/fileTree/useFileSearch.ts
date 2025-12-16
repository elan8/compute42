/**
 * File Search Composable
 * Handles search functionality for the file tree
 */

import { ref, computed, type Ref } from 'vue';
import { debug } from '../../utils/logger';
import type {
  FileNode,
  UseFileSearchReturn,
  SearchOptions,
  SearchResult,
  SearchMatch,
} from '../../types/fileTree';

export function useFileSearch(fileTree: Ref<FileNode[]>): UseFileSearchReturn {
  // ============================
  // State
  // ============================

  const searchQuery = ref<string>('');
  const isSearching = ref<boolean>(false);
  const searchResults = ref<SearchResult[]>([]);

  const searchOptions = ref<SearchOptions>({
    query: '',
    caseSensitive: false,
    wholeWord: false,
    regex: false,
    includeFiles: true,
    includeFolders: true,
    excludePatterns: [],
  });

  // ============================
  // Computed Properties
  // ============================

  const filteredTree = computed<FileNode[]>(() => {
    if (!searchQuery.value.trim()) {
      return fileTree.value;
    }

    return filterTreeNodes(fileTree.value, searchQuery.value, searchOptions.value);
  });

  // ============================
  // Search Functions
  // ============================

  const search = async (query: string): Promise<void> => {
    if (!query.trim()) {
      searchResults.value = [];
      return;
    }

    isSearching.value = true;
    searchQuery.value = query;

    try {
      debug(`Searching for: ${query}`);
      const results = performSearch(fileTree.value, query, searchOptions.value);
      searchResults.value = results;
      debug(`Found ${results.length} search results`);
    } catch (err) {
      debug(`Search error: ${err}`);
      searchResults.value = [];
    } finally {
      isSearching.value = false;
    }
  };

  const clearSearch = (): void => {
    searchQuery.value = '';
    searchResults.value = [];
    isSearching.value = false;
  };

  const setSearchOptions = (options: Partial<SearchOptions>): void => {
    searchOptions.value = { ...searchOptions.value, ...options };
  };

  // ============================
  // Search Implementation
  // ============================

  const performSearch = (
    nodes: FileNode[],
    query: string,
    options: SearchOptions
  ): SearchResult[] => {
    const results: SearchResult[] = [];

    const searchInNodes = (nodeList: FileNode[], parentPath: string = ''): void => {
      for (const node of nodeList) {
        const fullPath = parentPath ? `${parentPath}/${node.name}` : node.name;

        // Check if node should be included
        if (!shouldIncludeNode(node, options)) {
          continue;
        }

        // Check if node matches search criteria
        const matches = findMatches(node.name, query, options);
        if (matches.length > 0) {
          results.push({
            node,
            matches,
            score: calculateScore(node, matches, query),
          });
        }

        // Recursively search children
        if (node.children && node.children.length > 0) {
          searchInNodes(node.children, fullPath);
        }
      }
    };

    searchInNodes(nodes);

    // Sort results by score (highest first)
    return results.sort((a, b) => b.score - a.score);
  };

  const filterTreeNodes = (
    nodes: FileNode[],
    query: string,
    options: SearchOptions
  ): FileNode[] => {
    const filtered: FileNode[] = [];

    for (const node of nodes) {
      const matches = findMatches(node.name, query, options);
      const shouldInclude =
        matches.length > 0 ||
        (node.children && filterTreeNodes(node.children, query, options).length > 0);

      if (shouldInclude) {
        filtered.push({
          ...node,
          children: node.children ? filterTreeNodes(node.children, query, options) : undefined,
        });
      }
    }

    return filtered;
  };

  const findMatches = (text: string, query: string, options: SearchOptions): SearchMatch[] => {
    const matches: SearchMatch[] = [];

    if (!text || !query) {
      return matches;
    }

    let searchText = text;
    let searchQuery = query;

    // Apply case sensitivity
    if (!options.caseSensitive) {
      searchText = text.toLowerCase();
      searchQuery = query.toLowerCase();
    }

    // Apply regex or simple search
    if (options.regex) {
      try {
        const regex = new RegExp(searchQuery, options.caseSensitive ? 'g' : 'gi');
        let match;
        while ((match = regex.exec(searchText)) !== null) {
          matches.push({
            start: match.index,
            end: match.index + match[0].length,
            text: match[0],
          });
        }
      } catch (err) {
        debug(`Regex error: ${err}`);
        // Fall back to simple search
        const index = searchText.indexOf(searchQuery);
        if (index !== -1) {
          matches.push({
            start: index,
            end: index + searchQuery.length,
            text: searchQuery,
          });
        }
      }
    } else {
      // Simple search
      let index = 0;
      while ((index = searchText.indexOf(searchQuery, index)) !== -1) {
        matches.push({
          start: index,
          end: index + searchQuery.length,
          text: searchQuery,
        });
        index += searchQuery.length;
      }
    }

    return matches;
  };

  const shouldIncludeNode = (node: FileNode, options: SearchOptions): boolean => {
    // Check if node type should be included
    if (node.is_directory && !options.includeFolders) {
      return false;
    }
    if (!node.is_directory && !options.includeFiles) {
      return false;
    }

    // Check exclude patterns
    for (const pattern of options.excludePatterns) {
      if (node.name.includes(pattern)) {
        return false;
      }
    }

    return true;
  };

  const calculateScore = (node: FileNode, matches: SearchMatch[], query: string): number => {
    let score = 0;

    // Base score from number of matches
    score += matches.length * 10;

    // Bonus for exact matches
    if (node.name.toLowerCase() === query.toLowerCase()) {
      score += 100;
    }

    // Bonus for matches at the beginning of the name
    if (matches.some((match) => match.start === 0)) {
      score += 50;
    }

    // Bonus for file extensions
    if (!node.is_directory && node.name.includes('.')) {
      const extension = node.name.split('.').pop()?.toLowerCase();
      if (extension === query.toLowerCase()) {
        score += 30;
      }
    }

    return score;
  };

  const highlightMatches = (node: FileNode): SearchMatch[] => {
    if (!searchQuery.value.trim()) {
      return [];
    }

    return findMatches(node.name, searchQuery.value, searchOptions.value);
  };

  // ============================
  // Public API
  // ============================

  return {
    searchQuery,
    searchResults,
    isSearching,
    searchOptions,
    filteredTree,
    search,
    clearSearch,
    setSearchOptions,
    highlightMatches,
  };
}
