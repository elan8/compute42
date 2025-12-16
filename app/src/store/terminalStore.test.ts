import { describe, it, expect, beforeEach, vi } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useTerminalStore } from './terminalStore';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

describe('terminalStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  describe('initialization', () => {
    it('should initialize with default state', () => {
      const store = useTerminalStore();

      expect(store.activeTerminalId).toBeNull();
      expect(store.globalStreamInitialized).toBe(false);
      expect(store.juliaOutputBuffer).toBe('');
      expect(store.isListening).toBe(false);
      expect(store.isBusy).toBe(false);
    });
  });

  describe('activeTerminalId', () => {
    it('should set active terminal id', () => {
      const store = useTerminalStore();
      store.setActiveTerminalId('terminal-1');

      expect(store.activeTerminalId).toBe('terminal-1');
    });

    it('should clear active terminal id when expected id matches', () => {
      const store = useTerminalStore();
      store.setActiveTerminalId('terminal-1');
      store.clearActiveTerminalId('terminal-1');

      expect(store.activeTerminalId).toBeNull();
    });

    it('should not clear active terminal id when expected id does not match', () => {
      const store = useTerminalStore();
      store.setActiveTerminalId('terminal-1');
      store.clearActiveTerminalId('terminal-2');

      expect(store.activeTerminalId).toBe('terminal-1');
    });
  });

  describe('busy state', () => {
    it('should set and get busy state', () => {
      const store = useTerminalStore();
      store.setBusy(true);

      expect(store.isBusy).toBe(true);
      expect(store.getBusy()).toBe(true);
    });
  });

  describe('output buffer', () => {
    it('should get output buffer', () => {
      const store = useTerminalStore();
      store.juliaOutputBuffer = 'test output';

      expect(store.getOutputBuffer()).toBe('test output');
    });

    it('should clear output buffer', () => {
      const store = useTerminalStore();
      store.juliaOutputBuffer = 'test output';
      store.clearOutputBuffer();

      expect(store.juliaOutputBuffer).toBe('');
    });
  });

  describe('executeJuliaCode', () => {
    it('should execute Julia code', async () => {
      (invoke as any).mockResolvedValue('result');

      const store = useTerminalStore();
      const result = await store.executeJuliaCode('println("hello")');

      expect(result).toBe('result');
      expect(invoke).toHaveBeenCalledWith('execute_julia_code', {
        code: 'println("hello")',
      });
    });

    it('should initialize global stream if not initialized', async () => {
      (invoke as any).mockResolvedValue('result');
      const mockUnlisten = vi.fn();
      (listen as any).mockResolvedValue(mockUnlisten);

      const store = useTerminalStore();
      await store.executeJuliaCode('code');

      expect(store.globalStreamInitialized).toBe(true);
    });

    it('should handle errors', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const store = useTerminalStore();
      await expect(store.executeJuliaCode('code')).rejects.toThrow();
    });
  });

  describe('executeJuliaFile', () => {
    it('should execute Julia file', async () => {
      (invoke as any).mockResolvedValue('result');

      const store = useTerminalStore();
      const result = await store.executeJuliaFile('/path/to/file.jl', 'file content');

      expect(result).toBe('result');
      expect(invoke).toHaveBeenCalledWith('execute_julia_file', {
        filePath: '/path/to/file.jl',
        fileContent: 'file content',
      });
    });
  });

  describe('terminal serialized state', () => {
    it('should set and get terminal serialized state', () => {
      const store = useTerminalStore();
      store.setTerminalSerializedState('serialized-state');

      expect(store.getTerminalSerializedState()).toBe('serialized-state');
    });

    it('should clear terminal serialized state', () => {
      const store = useTerminalStore();
      store.setTerminalSerializedState('serialized-state');
      store.clearTerminalSerializedState();

      expect(store.getTerminalSerializedState()).toBeNull();
    });
  });

  describe('initial prompt', () => {
    it('should set and get has shown initial prompt', () => {
      const store = useTerminalStore();
      store.setHasShownInitialPrompt(true);

      expect(store.getHasShownInitialPrompt()).toBe(true);
    });
  });
});


