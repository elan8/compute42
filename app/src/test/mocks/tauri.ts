import { vi } from 'vitest';
import type { Mock } from 'vitest';

/**
 * Tauri API Mocks
 * Provides mock implementations and helper functions for Tauri APIs
 */

// Mock implementations
export const mockInvoke = vi.fn() as Mock;
export const mockListen = vi.fn() as Mock;

/**
 * Reset all Tauri mocks
 */
export function resetTauriMocks() {
  mockInvoke.mockReset();
  mockListen.mockReset();
}

/**
 * Set up default mock implementations
 */
export function setupTauriMocks() {
  // Default invoke mock - returns undefined
  mockInvoke.mockResolvedValue(undefined);

  // Default listen mock - returns a cleanup function
  mockListen.mockResolvedValue(() => {});
}

/**
 * Helper to create a mock event listener that can be triggered
 */
export function createMockEventListener() {
  let handler: ((payload: any) => void) | null = null;

  const unlisten = vi.fn(() => {
    handler = null;
  });

  mockListen.mockImplementation(async (event: string, callback: (event: any) => void) => {
    handler = callback;
    return unlisten;
  });

  return {
    trigger: (payload: any) => {
      if (handler) {
        handler({ payload });
      }
    },
    unlisten,
  };
}

/**
 * Helper to set up a mock invoke response for a specific command
 */
export function mockInvokeCommand<T>(command: string, response: T) {
  mockInvoke.mockImplementation(async (cmd: string, ...args: any[]) => {
    if (cmd === command) {
      return response;
    }
    return undefined;
  });
}

/**
 * Helper to set up a mock invoke that throws an error for a specific command
 */
export function mockInvokeError(command: string, error: Error | string) {
  mockInvoke.mockImplementation(async (cmd: string, ...args: any[]) => {
    if (cmd === command) {
      throw typeof error === 'string' ? new Error(error) : error;
    }
    return undefined;
  });
}


