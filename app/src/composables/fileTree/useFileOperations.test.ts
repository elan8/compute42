import { describe, it, expect, beforeEach, vi } from 'vitest';
import { useFileOperations } from './useFileOperations';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('useFileOperations', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('createFile', () => {
    it('should create a file', async () => {
      (invoke as any).mockResolvedValue(undefined);

      const { createFile } = useFileOperations();
      const result = await createFile('/path/to/file.jl', 'content');

      expect(result.success).toBe(true);
      expect(invoke).toHaveBeenCalledWith('create_file_item', { path: '/path/to/file.jl' });
      expect(invoke).toHaveBeenCalledWith('write_file_content', {
        path: '/path/to/file.jl',
        content: 'content',
      });
    });

    it('should create a file without content', async () => {
      (invoke as any).mockResolvedValue(undefined);

      const { createFile } = useFileOperations();
      const result = await createFile('/path/to/file.jl');

      expect(result.success).toBe(true);
      expect(invoke).toHaveBeenCalledWith('create_file_item', { path: '/path/to/file.jl' });
      expect(invoke).not.toHaveBeenCalledWith('write_file_content', expect.anything());
    });

    it('should handle errors', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const { createFile } = useFileOperations();
      const result = await createFile('/path/to/file.jl');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });
  });

  describe('createFolder', () => {
    it('should create a folder', async () => {
      (invoke as any).mockResolvedValue(undefined);

      const { createFolder } = useFileOperations();
      const result = await createFolder('/path/to/folder');

      expect(result.success).toBe(true);
      expect(invoke).toHaveBeenCalledWith('create_folder_item', { path: '/path/to/folder' });
    });

    it('should handle errors', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const { createFolder } = useFileOperations();
      const result = await createFolder('/path/to/folder');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });
  });

  describe('deleteItem', () => {
    it('should delete an item', async () => {
      (invoke as any).mockResolvedValue(undefined);

      const { deleteItem } = useFileOperations();
      const result = await deleteItem('/path/to/item');

      expect(result.success).toBe(true);
      expect(invoke).toHaveBeenCalledWith('delete_item', { path: '/path/to/item' });
    });

    it('should handle errors', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const { deleteItem } = useFileOperations();
      const result = await deleteItem('/path/to/item');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });
  });

  describe('renameItem', () => {
    it('should rename an item', async () => {
      (invoke as any).mockResolvedValue(undefined);

      const { renameItem } = useFileOperations();
      const result = await renameItem('/old/path', '/new/path');

      expect(result.success).toBe(true);
      expect(invoke).toHaveBeenCalledWith('rename_item', {
        oldPath: '/old/path',
        newPath: '/new/path',
      });
    });

    it('should handle errors', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const { renameItem } = useFileOperations();
      const result = await renameItem('/old/path', '/new/path');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });
  });

  describe('moveItem', () => {
    it('should move an item', async () => {
      (invoke as any).mockResolvedValue(undefined);

      const { moveItem } = useFileOperations();
      const result = await moveItem('/old/path', '/new/path');

      expect(result.success).toBe(true);
      expect(invoke).toHaveBeenCalledWith('rename_item', {
        oldPath: '/old/path',
        newPath: '/new/path',
      });
    });

    it('should handle errors', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const { moveItem } = useFileOperations();
      const result = await moveItem('/old/path', '/new/path');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });
  });

  describe('copyItem', () => {
    it('should copy an item', async () => {
      (invoke as any)
        .mockResolvedValueOnce('file content')
        .mockResolvedValueOnce(undefined)
        .mockResolvedValueOnce(undefined);

      const { copyItem } = useFileOperations();
      const result = await copyItem('/old/path', '/new/path');

      expect(result.success).toBe(true);
      expect(invoke).toHaveBeenCalledWith('read_file_content', { path: '/old/path' });
      expect(invoke).toHaveBeenCalledWith('create_file_item', { path: '/new/path' });
      expect(invoke).toHaveBeenCalledWith('write_file_content', {
        path: '/new/path',
        content: 'file content',
      });
    });

    it('should handle errors', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const { copyItem } = useFileOperations();
      const result = await copyItem('/old/path', '/new/path');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });
  });

  describe('readFile', () => {
    it('should read a file', async () => {
      (invoke as any).mockResolvedValue('file content');

      const { readFile } = useFileOperations();
      const content = await readFile('/path/to/file.jl');

      expect(content).toBe('file content');
      expect(invoke).toHaveBeenCalledWith('read_file_content', { path: '/path/to/file.jl' });
    });

    it('should throw error on failure', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const { readFile } = useFileOperations();
      await expect(readFile('/path/to/file.jl')).rejects.toThrow();
    });
  });

  describe('writeFile', () => {
    it('should write to a file', async () => {
      (invoke as any).mockResolvedValue(undefined);

      const { writeFile } = useFileOperations();
      const result = await writeFile('/path/to/file.jl', 'content');

      expect(result.success).toBe(true);
      expect(invoke).toHaveBeenCalledWith('write_file_content', {
        path: '/path/to/file.jl',
        content: 'content',
      });
    });

    it('should handle errors', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const { writeFile } = useFileOperations();
      const result = await writeFile('/path/to/file.jl', 'content');

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });
  });
});


