import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { debug, error } from '../utils/logger';
// Prefer generated types from ts-rs when available
import type { LspHover as GenHover } from '../types/bindings/shared/LspHover';
import type { LspCompletionItem as GenCompletionItem } from '../types/bindings/shared/LspCompletionItem';
import type { LspLocation as GenLocation } from '../types/bindings/shared/LspLocation';

export interface LSPDiagnostic {
  range: {
    start: { line: number; character: number };
    end: { line: number; character: number };
  };
  severity: number;
  message: string;
  source?: string;
  code?: string;
}

export type LSPHover = GenHover;

export type LSPCompletionItem = GenCompletionItem;

export interface LSPCompletionList {
  isIncomplete: boolean;
  items: LSPCompletionItem[];
}

export type LSPLocation = GenLocation;

class LSPService {
  private isInitialized = false;
  private projectPath: string | null = null;
  private eventListeners: Array<() => void> = [];
  private isRestarting = false;
  private pendingRequests: Map<string, Promise<any>> = new Map();

  constructor() {
    this.setupEventListeners();
  }

  private setupEventListeners() {
    // Listen for LSP diagnostics (not yet implemented in Rust LSP)
    listen('lsp-diagnostics', (event) => {
      debug('LSP: Received diagnostics:', event.payload as any);
      // Note: Diagnostics are not yet implemented in the Rust LSP
      // This listener is kept for compatibility but will not receive events
    }).then((unlisten) => this.eventListeners.push(unlisten));

    // Listen for LSP show message (notifications disabled)
    listen('lsp-show-message', (event) => {
      debug('LSP: Show message:', event.payload as any);
      // Handle show message - notifications disabled
    }).then((unlisten) => this.eventListeners.push(unlisten));

    // Listen for LSP log message
    listen('lsp-log-message', (event) => {
      debug('LSP: Log message:', event.payload as any);
      // Handle log message - could add to log output
    }).then((unlisten) => this.eventListeners.push(unlisten));

    // Listen for LSP hover
    listen('lsp-hover', (event) => {
      debug('LSP: Hover:', event.payload as any);
      // Handle hover - could update Monaco editor hover
    }).then((unlisten) => this.eventListeners.push(unlisten));

    // Listen for LSP completion
    listen('lsp-completion', (event) => {
      debug('LSP: Completion:', event.payload as any);
      // Handle completion - could update Monaco editor completion
    }).then((unlisten) => this.eventListeners.push(unlisten));

    // Listen for LSP status events to update initialization state
    listen('lsp:status', (event) => {
      const payload = event.payload as any;
      if (payload.status === 'ready' || payload.status === 'initialized') {
        debug('LSP Service: LSP server is ready, setting initialized flag');
        this.isInitialized = true;
      } else if (payload.status === 'stopped' || payload.status === 'failed') {
        debug('LSP Service: LSP server stopped/failed, clearing initialized flag');
        this.isInitialized = false;
      }
    }).then((unlisten) => this.eventListeners.push(unlisten));

    // Also listen for lsp:ready events (emitted by emit_lsp_ready)
    listen('lsp:ready', (event) => {
      debug('LSP Service: LSP ready event received, setting initialized flag');
      this.isInitialized = true;
    }).then((unlisten) => this.eventListeners.push(unlisten));

    // Listen for lsp:initialized events
    listen('lsp:initialized', (event) => {
      debug('LSP Service: LSP initialized event received, setting initialized flag');
      this.isInitialized = true;
    }).then((unlisten) => this.eventListeners.push(unlisten));

    // Listen for lsp:server-started events
    listen('lsp:server-started', (event) => {
      debug('LSP Service: LSP server started event received, setting initialized flag');
      this.isInitialized = true;
    }).then((unlisten) => this.eventListeners.push(unlisten));

    // Listen for events that should clear the initialized flag
    listen('lsp:server-stopped', (event) => {
      debug('LSP Service: LSP server stopped event received, clearing initialized flag');
      this.isInitialized = false;
    }).then((unlisten) => this.eventListeners.push(unlisten));

    listen('lsp:server-error', (event) => {
      debug('LSP Service: LSP server error event received, clearing initialized flag');
      this.isInitialized = false;
    }).then((unlisten) => this.eventListeners.push(unlisten));

    listen('lsp:shutdown', (event) => {
      debug('LSP Service: LSP shutdown event received, clearing initialized flag');
      this.isInitialized = false;
    }).then((unlisten) => this.eventListeners.push(unlisten));
  }

  async shutdown(): Promise<void> {
    try {
      if (this.isInitialized) {
        debug('LSP Service: Shutting down');
        await invoke('lsp_shutdown');
        this.isInitialized = false;
        this.projectPath = null;
        debug('LSP Service: Shutdown complete');
      }
    } catch (err) {
      error(`LSP Service: Failed to shutdown: ${err}`);
    }
  }

  async restart(projectPath: string): Promise<void> {
    if (this.isRestarting) {
      debug('LSP Service: Restart already in progress, waiting...');
      // Wait for current restart to complete
      while (this.isRestarting) {
        await new Promise((resolve) => setTimeout(resolve, 100));
      }
      return;
    }

    try {
      this.isRestarting = true;
      debug('LSP Service: Restarting LSP server');

      // Clear any pending requests
      this.pendingRequests.clear();

      await invoke('lsp_restart', { projectPath });
      this.isInitialized = true;
      this.projectPath = projectPath;
      debug('LSP Service: Restart complete');
    } catch (err) {
      error(`LSP Service: Failed to restart: ${err}`);
      throw err;
    } finally {
      this.isRestarting = false;
    }
  }

  async isRunning(): Promise<boolean> {
    try {
      return await invoke('lsp_is_running');
    } catch (err) {
      error(`LSP Service: Failed to check running status: ${err}`);
      return false;
    }
  }

  async requestCompletion(
    uri: string,
    line: number,
    character: number,
    fileContent?: string
  ): Promise<LSPCompletionList | null> {
    try {
      const startedAt = Date.now();
      debug(`LSP Service: requestCompletion uri=${uri} pos=${line}:${character}`);
      const params: any = { uri, line, character };
      if (typeof fileContent === 'string') params.fileContent = fileContent;
      const result = await invoke('lsp_get_completions', params);
      const elapsedMs = Date.now() - startedAt;
      // Normalize backend response: it may be Vec<LspCompletionItem> or { items, isIncomplete }
      let normalized: LSPCompletionList;
      if (Array.isArray(result)) {
        normalized = { isIncomplete: false, items: result as any };
      } else {
        const obj = result as any;
        normalized = { isIncomplete: !!obj?.isIncomplete, items: obj?.items ?? [] };
      }
      debug(
        `LSP Service: requestCompletion done items=${normalized.items.length} timeMs=${elapsedMs}`
      );
      return normalized;
    } catch (err) {
      error(`LSP Service: Failed to get completion: ${err}`);
      return null;
    }
  }

  async requestHover(uri: string, line: number, character: number): Promise<LSPHover | null> {
    const requestKey = `hover-${uri}-${line}-${character}`;

    // Check if request is already pending
    if (this.pendingRequests.has(requestKey)) {
      debug('LSP Service: Request already pending, returning existing promise');
      return this.pendingRequests.get(requestKey);
    }

    const requestPromise = this.requestHoverWithRetry(uri, line, character);
    this.pendingRequests.set(requestKey, requestPromise);

    try {
      const result = await requestPromise;
      return result;
    } finally {
      this.pendingRequests.delete(requestKey);
    }
  }

  private async requestHoverWithRetry(
    uri: string,
    line: number,
    character: number,
    maxRetries: number = 3
  ): Promise<LSPHover | null> {
    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        if (!this.isInitialized) {
          debug('LSP Service: Not initialized, skipping hover request');
          return null;
        }

        debug(
          `LSP Service: Requesting hover for ${uri} at ${line}:${character} (attempt ${attempt}/${maxRetries})`
        );
        const result = await invoke('lsp_hover', { uri, line, character });
        debug('LSP Service: Hover result:', result);
        return result as LSPHover;
      } catch (err) {
        error(`LSP Service: Failed to get hover (attempt ${attempt}/${maxRetries}): ${err}`);

        if (attempt < maxRetries) {
          // Wait before retrying
          const delay = 500 * attempt;
          debug(`LSP Service: Waiting ${delay}ms before retry`);
          await new Promise((resolve) => setTimeout(resolve, delay));
        } else {
          return null;
        }
      }
    }
    return null;
  }

  async requestDefinition(uri: string, line: number, character: number): Promise<LSPLocation[]> {
    try {
      await debug(
        `LSP Service: Requesting definition uri=${uri} line=${line} character=${character} initialized=${this.isInitialized} projectPath=${this.projectPath}`
      );

      const result = await invoke('lsp_get_definition', { uri, line, character });

      await debug(
        `LSP Service: Definition response received uri=${uri} type=${typeof result} length=${Array.isArray(result) ? result.length : 'n/a'}`
      );

      const locations = result as LSPLocation[];

      await debug(`LSP Service: Parsed definition locations count=${locations.length}`);

      return locations;
    } catch (err) {
      await error(
        `LSP Service: Failed to get definition uri=${uri} line=${line} character=${character}: ${err instanceof Error ? err.message : String(err)}`
      );
      return [];
    }
  }

  async requestReferences(uri: string, line: number, character: number): Promise<LSPLocation[]> {
    try {
      await debug(
        `LSP Service: Requesting references uri=${uri} line=${line} character=${character} initialized=${this.isInitialized} projectPath=${this.projectPath}`
      );
      debug('LSP Service: About to invoke lsp_get_references with:', { uri, line, character });

      const result = await invoke('lsp_get_references', { uri, line, character });
      debug('LSP Service: Backend returned:', result);

      await debug(
        `LSP Service: References response received uri=${uri} type=${typeof result} length=${Array.isArray(result) ? result.length : 'n/a'}`
      );

      const locations = result as LSPLocation[];

      await debug(`LSP Service: Parsed reference locations count=${locations.length}`);

      return locations;
    } catch (err) {
      await error(
        `LSP Service: Failed to get references uri=${uri} line=${line} character=${character}: ${err instanceof Error ? err.message : String(err)}`
      );
      console.error('LSP Service: Error in requestReferences:', err);
      return [];
    }
  }

  async requestDiagnostics(uri: string): Promise<LSPDiagnostic[]> {
    try {
      const result = await invoke('lsp_get_diagnostics', { uri });
      return (result as LSPDiagnostic[]) || [];
    } catch (err) {
      error(`LSP Service: Failed to get diagnostics: ${err}`);
      return [];
    }
  }

  async notifyDidChange(uri: string, content: string): Promise<void> {
    try {
      await invoke('lsp_notify_did_change', { uri, content });
    } catch (err) {
      error(`LSP Service: Failed to notify document change: ${err}`);
    }
  }

  async notifyDidOpen(uri: string, content: string, language: string): Promise<void> {
    try {
      await invoke('lsp_notify_did_open', { uri, content, language });
    } catch (err) {
      error(`LSP Service: Failed to notify document open: ${err}`);
    }
  }

  async notifyDidClose(uri: string): Promise<void> {
    try {
      await invoke('lsp_notify_did_close', { uri });
    } catch (err) {
      error(`LSP Service: Failed to notify document close: ${err}`);
    }
  }

  async notifyDidSave(uri: string): Promise<void> {
    try {
      await invoke('lsp_notify_did_save', { uri });
    } catch (err) {
      error(`LSP Service: Failed to notify document save: ${err}`);
    }
  }

  cleanup() {
    // Remove all event listeners
    this.eventListeners.forEach((unlisten) => unlisten());
    this.eventListeners = [];
  }
}

// Export singleton instance
export const lspService = new LSPService();
