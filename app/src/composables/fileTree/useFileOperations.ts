/**
 * File Operations Composable
 * Handles file and folder creation, deletion, renaming, and other operations
 */

import { ref, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { debug, error } from '../../utils/logger';
import type { UseFileOperationsReturn, FileOperationResult } from '../../types/fileTree';

export function useFileOperations(): UseFileOperationsReturn {
  // ============================
  // State
  // ============================

  const operationLoading = ref<boolean>(false);
  const lastError = ref<string | null>(null);

  // ============================
  // Helper Functions
  // ============================

  const handleOperation = async <T>(
    operation: () => Promise<T>,
    operationName: string
  ): Promise<FileOperationResult> => {
    operationLoading.value = true;
    lastError.value = null;

    try {
      debug(`Starting ${operationName}`);
      const result = await operation();
      debug(`${operationName} completed successfully`);

      return {
        success: true,
        data: result,
      };
    } catch (err) {
      const errorMessage = `${operationName} failed: ${err}`;
      error(errorMessage);
      lastError.value = errorMessage;

      return {
        success: false,
        error: errorMessage,
      };
    } finally {
      operationLoading.value = false;
    }
  };

  // ============================
  // File Operations
  // ============================

  const createFile = async (path: string, content: string = ''): Promise<FileOperationResult> => {
    return handleOperation(async () => {
      debug(`Creating file: ${path}`);
      await invoke('create_file_item', { path });

      // If content is provided, write it to the file
      if (content) {
        await invoke('write_file_content', { path, content });
      }

      return { path, content };
    }, 'create file');
  };

  const createFolder = async (path: string): Promise<FileOperationResult> => {
    return handleOperation(async () => {
      debug(`Creating folder: ${path}`);
      await invoke('create_folder_item', { path });
      return { path };
    }, 'create folder');
  };

  const deleteItem = async (path: string): Promise<FileOperationResult> => {
    return handleOperation(async () => {
      debug(`Deleting item: ${path}`);
      await invoke('delete_item', { path });
      return { path };
    }, 'delete item');
  };

  const renameItem = async (oldPath: string, newPath: string): Promise<FileOperationResult> => {
    return handleOperation(async () => {
      debug(`Renaming item: ${oldPath} -> ${newPath}`);
      await invoke('rename_item', { oldPath, newPath });
      return { oldPath, newPath };
    }, 'rename item');
  };

  const moveItem = async (oldPath: string, newPath: string): Promise<FileOperationResult> => {
    return handleOperation(async () => {
      debug(`Moving item: ${oldPath} -> ${newPath}`);
      // For now, we'll use rename for move operations
      // In a more sophisticated implementation, we might have a separate move command
      await invoke('rename_item', { oldPath, newPath });
      return { oldPath, newPath };
    }, 'move item');
  };

  const copyItem = async (oldPath: string, newPath: string): Promise<FileOperationResult> => {
    return handleOperation(async () => {
      debug(`Copying item: ${oldPath} -> ${newPath}`);
      // For now, we'll read the content and create a new file
      // In a more sophisticated implementation, we might have a separate copy command
      const content = await invoke('read_file_content', { path: oldPath });
      await invoke('create_file_item', { path: newPath });
      await invoke('write_file_content', { path: newPath, content });
      return { oldPath, newPath };
    }, 'copy item');
  };

  const readFile = async (path: string): Promise<string> => {
    try {
      debug(`Reading file: ${path}`);
      const content = await invoke('read_file_content', { path });
      return content;
    } catch (err) {
      const errorMessage = `Failed to read file: ${err}`;
      error(errorMessage);
      throw new Error(errorMessage);
    }
  };

  const writeFile = async (path: string, content: string): Promise<FileOperationResult> => {
    return handleOperation(async () => {
      debug(`Writing file: ${path}`);
      await invoke('write_file_content', { path, content });
      return { path, content };
    }, 'write file');
  };

  // ============================
  // Public API
  // ============================

  return {
    createFile,
    createFolder,
    deleteItem,
    renameItem,
    moveItem,
    copyItem,
    readFile,
    writeFile,
  };
}
