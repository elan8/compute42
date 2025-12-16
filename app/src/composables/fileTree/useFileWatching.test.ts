import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useFileWatching } from './useFileWatching';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

describe('useFileWatching', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('initialization', () => {
    it('should initialize with stopped state', () => {
      const { isWatching, watcherId } = useFileWatching();

      expect(isWatching.value).toBe(false);
      expect(watcherId.value).toBeNull();
    });
  });

  describe('startWatching', () => {
    it('should start watching a path', async () => {
      const mockUnlisten = vi.fn();
      (invoke as any).mockResolvedValue('watcher-id-123');
      (listen as any).mockResolvedValue(mockUnlisten);

      const { startWatching, isWatching, watcherId } = useFileWatching();

      await startWatching('/path/to/watch');

      expect(isWatching.value).toBe(true);
      expect(watcherId.value).toBe('watcher-id-123');
      expect(invoke).toHaveBeenCalledWith('start_file_watcher', {
        path: '/path/to/watch',
        recursive: true,
      });
      expect(listen).toHaveBeenCalledWith('file:changed', expect.any(Function));
    });

    it('should start watching with recursive option', async () => {
      const mockUnlisten = vi.fn();
      (invoke as any).mockResolvedValue('watcher-id-123');
      (listen as any).mockResolvedValue(mockUnlisten);

      const { startWatching } = useFileWatching();

      await startWatching('/path/to/watch', false);

      expect(invoke).toHaveBeenCalledWith('start_file_watcher', {
        path: '/path/to/watch',
        recursive: false,
      });
    });

    it('should stop existing watcher before starting new one', async () => {
      const mockUnlisten = vi.fn();
      (invoke as any)
        .mockResolvedValueOnce('watcher-id-1')
        .mockResolvedValueOnce(undefined) // stop_file_watcher
        .mockResolvedValueOnce('watcher-id-2');
      (listen as any).mockResolvedValue(mockUnlisten);

      const { startWatching, watcherId } = useFileWatching();

      await startWatching('/path1');
      expect(watcherId.value).toBe('watcher-id-1');

      await startWatching('/path2');
      expect(invoke).toHaveBeenCalledWith('stop_file_watcher', { watcher_id: 'watcher-id-1' });
      expect(watcherId.value).toBe('watcher-id-2');
    });

    it('should handle errors', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const { startWatching } = useFileWatching();

      await expect(startWatching('/path')).rejects.toThrow();
    });
  });

  describe('stopWatching', () => {
    it('should stop watching', async () => {
      const mockUnlisten = vi.fn();
      (invoke as any)
        .mockResolvedValueOnce('watcher-id-123')
        .mockResolvedValueOnce(undefined);
      (listen as any).mockResolvedValue(mockUnlisten);

      const { startWatching, stopWatching, isWatching } = useFileWatching();

      await startWatching('/path');
      await stopWatching();

      expect(isWatching.value).toBe(false);
      expect(invoke).toHaveBeenCalledWith('stop_file_watcher', { watcher_id: 'watcher-id-123' });
      expect(mockUnlisten).toHaveBeenCalled();
    });

    it('should not throw if not watching', async () => {
      const { stopWatching } = useFileWatching();

      await expect(stopWatching()).resolves.not.toThrow();
    });

    it('should handle errors', async () => {
      const mockUnlisten = vi.fn();
      (invoke as any)
        .mockResolvedValueOnce('watcher-id-123')
        .mockRejectedValueOnce(new Error('Failed'));
      (listen as any).mockResolvedValue(mockUnlisten);

      const { startWatching, stopWatching } = useFileWatching();

      await startWatching('/path');
      await expect(stopWatching()).rejects.toThrow();
    });
  });

  describe('onFileChange', () => {
    it('should register file change handler', async () => {
      const mockUnlisten = vi.fn();
      let eventHandler: ((event: any) => void) | null = null;

      (invoke as any).mockResolvedValue('watcher-id-123');
      (listen as any).mockImplementation((event, handler) => {
        eventHandler = handler;
        return Promise.resolve(mockUnlisten);
      });

      const { startWatching, onFileChange } = useFileWatching();
      await startWatching('/path');

      const handler = vi.fn();
      onFileChange(handler);

      // Simulate file change event
      if (eventHandler) {
        eventHandler({
          payload: {
            change_type: 'created',
            path: '/path/newfile.jl',
          },
        });
      }

      expect(handler).toHaveBeenCalledWith({
        change_type: 'created',
        path: '/path/newfile.jl',
      });
    });

    it('should return cleanup function', () => {
      const { onFileChange } = useFileWatching();

      const handler = vi.fn();
      const cleanup = onFileChange(handler);

      expect(typeof cleanup).toBe('function');
    });
  });
});


