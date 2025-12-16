// Package search and recommendation types for SearchPackages integration

export interface SearchPackagesResult {
  name: string;
  uuid: string;
  repository_url?: string;
  description?: string;
  stars?: number;
  topics: string[];
}

export interface SearchPackagesResponse {
  packages: SearchPackagesResult[];
  total: number;
  query: string;
}

export interface SuggestionsResponse {
  suggestions: string[];
  query: string;
}

export interface SearchFilters {
  categories?: string[];
  min_stars?: number;
  max_stars?: number;
  topics?: string[];
}

export interface PackageSearchOptions {
  query: string;
  filters?: SearchFilters;
  limit?: number;
}

export interface TrendingPackagesOptions {
  limit?: number;
}

export interface RecommendationOptions {
  project_path: string;
  limit?: number;
}

// UI-specific types
export interface PackageCardData {
  package: SearchPackagesResult;
  relevance_score?: number;
  reason?: string;
  category?: string;
  is_installed?: boolean;
  is_direct?: boolean; // true if it's a direct dependency, false if transitive, undefined if not installed
  is_trending?: boolean;
}

export interface PackageDetailsData extends SearchPackagesResult {
  full_description?: string;
  documentation_url?: string;
  license?: string;
  last_updated?: string;
  dependencies?: string[];
  related_packages?: string[];
}

// Error types
export interface PackageSearchError {
  message: string;
  code?: string;
  details?: any;
}

// Cache types
export interface SearchCache {
  [query: string]: {
    results: SearchPackagesResponse;
    timestamp: number;
    ttl: number;
  };
}
