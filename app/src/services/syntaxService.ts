import { invoke } from '@tauri-apps/api/core';
import { debug, error } from '../utils/logger';

export interface SyntaxDiagnostic {
  range: {
    start: { line: number; character: number };
    end: { line: number; character: number };
  };
  severity: number;
  message: string;
  source?: string;
  code?: string;
}

/**
 * User-friendly syntax service that waits for users to stop typing before checking syntax.
 * This prevents distracting syntax errors from appearing while users are actively writing code.
 */
class SyntaxService {
  private parseCache = new Map<string, { diagnostics: SyntaxDiagnostic[]; content: string }>();
  private debounceTimeouts = new Map<string, NodeJS.Timeout>();
  private defaultDebounceMs = 1500; // Increased to 1.5 seconds for better UX
  private typingIndicators = new Map<string, boolean>(); // Track if user is actively typing
  private stopTypingTimeouts = new Map<string, NodeJS.Timeout>(); // Auto-stop typing after inactivity

  constructor() {
    debug('SyntaxService: Initialized');
  }

  /**
   * Mark that the user is actively typing in a file
   * @param uri File URI
   */
  markTyping(uri: string): void {
    this.typingIndicators.set(uri, true);

    // Clear any existing stop-typing timeout
    const existingTimeout = this.stopTypingTimeouts.get(uri);
    if (existingTimeout) {
      clearTimeout(existingTimeout);
    }

    // Set a timeout to automatically stop typing after 2 seconds of inactivity
    const timeout = setTimeout(() => {
      this.markStoppedTyping(uri);
      this.stopTypingTimeouts.delete(uri);
    }, 2000);

    this.stopTypingTimeouts.set(uri, timeout);
  }

  /**
   * Mark that the user has stopped typing in a file
   * @param uri File URI
   */
  markStoppedTyping(uri: string): void {
    this.typingIndicators.set(uri, false);

    // Clear any pending stop-typing timeout since user explicitly stopped
    const existingTimeout = this.stopTypingTimeouts.get(uri);
    if (existingTimeout) {
      clearTimeout(existingTimeout);
      this.stopTypingTimeouts.delete(uri);
    }
  }

  /**
   * Check if the user is currently typing in a file
   * @param uri File URI
   * @returns True if user is typing, false otherwise
   */
  isTyping(uri: string): boolean {
    return this.typingIndicators.get(uri) || false;
  }

  /**
   * Parse Julia code and return syntax diagnostics with user-friendly debouncing
   * This method waits for the user to stop typing before performing syntax checks
   * @param uri File URI
   * @param content File content
   * @param debounceMs Debounce delay in milliseconds (default: 1500)
   * @returns Promise with syntax diagnostics
   */
  async parseSyntax(
    uri: string,
    content: string,
    debounceMs: number = this.defaultDebounceMs
  ): Promise<SyntaxDiagnostic[]> {
    debug(`SyntaxService: Parsing syntax for ${uri} (${content.length} chars)`);

    try {
      // Clear any existing timeout for this URI
      const existingTimeout = this.debounceTimeouts.get(uri);
      if (existingTimeout) {
        clearTimeout(existingTimeout);
      }

      // Set up debounced parsing that waits for user to stop typing
      return new Promise((resolve) => {
        const timeout = setTimeout(async () => {
          try {
            // Only parse if user has stopped typing
            if (!this.isTyping(uri)) {
              const diagnostics = await invoke<SyntaxDiagnostic[]>('parse_julia_syntax', {
                uri,
                content,
              });

              // Cache the results with content
              this.parseCache.set(uri, { diagnostics, content });
              this.debounceTimeouts.delete(uri);

              debug(
                `SyntaxService: Parsed ${diagnostics.length} syntax diagnostics for ${uri} (user stopped typing)`
              );
              resolve(diagnostics);
            } else {
              // User is still typing, don't parse yet
              debug(`SyntaxService: Skipping parse for ${uri} - user still typing`);
              this.debounceTimeouts.delete(uri);
              resolve([]);
            }
          } catch (err) {
            error(`SyntaxService: Failed to parse syntax for ${uri}: ${err}`);
            this.debounceTimeouts.delete(uri);
            resolve([]);
          }
        }, debounceMs);

        this.debounceTimeouts.set(uri, timeout);
      });
    } catch (err) {
      error(`SyntaxService: Error in parseSyntax for ${uri}: ${err}`);
      return [];
    }
  }

  /**
   * Parse Julia code immediately without debouncing (for save events)
   * @param uri File URI
   * @param content File content
   * @returns Promise with syntax diagnostics
   */
  async parseSyntaxImmediate(uri: string, content: string): Promise<SyntaxDiagnostic[]> {
    debug(`SyntaxService: Immediate parsing for ${uri}`);

    try {
      // Mark that user has stopped typing (since they're saving)
      this.markStoppedTyping(uri);

      // Clear any pending debounced parse for this URI
      const existingTimeout = this.debounceTimeouts.get(uri);
      if (existingTimeout) {
        clearTimeout(existingTimeout);
        this.debounceTimeouts.delete(uri);
      }

      const diagnostics = await invoke<SyntaxDiagnostic[]>('parse_julia_syntax', {
        uri,
        content,
      });

      // Cache the results
      this.parseCache.set(uri, { diagnostics, content });

      debug(
        `SyntaxService: Immediate parse completed - ${diagnostics.length} diagnostics for ${uri}`
      );
      return diagnostics;
    } catch (err) {
      error(`SyntaxService: Failed immediate parse for ${uri}: ${err}`);
      return [];
    }
  }

  /**
   * Get cached syntax diagnostics for a file
   * @param uri File URI
   * @returns Promise with cached diagnostics
   */
  async getCachedDiagnostics(uri: string): Promise<SyntaxDiagnostic[]> {
    debug(`SyntaxService: Getting cached diagnostics for ${uri}`);

    try {
      const diagnostics = await invoke<SyntaxDiagnostic[]>('get_syntax_diagnostics', {
        uri,
      });

      // Update cache
      this.parseCache.set(uri, { diagnostics, content: '' });

      debug(`SyntaxService: Retrieved ${diagnostics.length} cached diagnostics for ${uri}`);
      return diagnostics;
    } catch (err) {
      error(`SyntaxService: Failed to get cached diagnostics for ${uri}: ${err}`);
      return [];
    }
  }

  /**
   * Clear cached diagnostics for a file
   * @param uri File URI
   */
  async clearCache(uri: string): Promise<void> {
    debug(`SyntaxService: Clearing cache for ${uri}`);

    try {
      // Clear local cache
      this.parseCache.delete(uri);

      // Clear any pending timeout
      const timeout = this.debounceTimeouts.get(uri);
      if (timeout) {
        clearTimeout(timeout);
        this.debounceTimeouts.delete(uri);
      }

      // Clear typing indicator
      this.typingIndicators.delete(uri);

      // Clear stop-typing timeout
      const stopTypingTimeout = this.stopTypingTimeouts.get(uri);
      if (stopTypingTimeout) {
        clearTimeout(stopTypingTimeout);
        this.stopTypingTimeouts.delete(uri);
      }

      // Clear backend cache
      await invoke('clear_syntax_cache', { uri });

      debug(`SyntaxService: Cache cleared for ${uri}`);
    } catch (err) {
      error(`SyntaxService: Failed to clear cache for ${uri}: ${err}`);
    }
  }

  /**
   * Check if syntax service is available
   * @returns Promise with availability status
   */
  async isAvailable(): Promise<boolean> {
    try {
      const available = await invoke<boolean>('is_syntax_service_available');
      debug(`SyntaxService: Availability check - ${available}`);
      return available;
    } catch (err) {
      error(`SyntaxService: Failed to check availability: ${err}`);
      return false;
    }
  }

  /**
   * Set the default debounce delay
   * @param debounceMs Debounce delay in milliseconds
   */
  setDefaultDebounceMs(debounceMs: number): void {
    this.defaultDebounceMs = debounceMs;
    debug(`SyntaxService: Default debounce set to ${debounceMs}ms`);
  }

  /**
   * Clear all caches and timeouts
   */
  clearAll(): void {
    debug('SyntaxService: Clearing all caches and timeouts');

    // Clear all timeouts
    for (const timeout of this.debounceTimeouts.values()) {
      clearTimeout(timeout);
    }
    this.debounceTimeouts.clear();

    // Clear all caches
    this.parseCache.clear();

    // Clear all typing indicators
    this.typingIndicators.clear();

    // Clear all stop-typing timeouts
    for (const timeout of this.stopTypingTimeouts.values()) {
      clearTimeout(timeout);
    }
    this.stopTypingTimeouts.clear();
  }
}

// Export singleton instance
export const syntaxService = new SyntaxService();
