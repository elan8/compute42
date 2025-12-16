/**
 * File Watching Composable
 * Handles file system watching and real-time updates
 */

import { ref, onMounted, onUnmounted, type Ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { debug, error } from '../../utils/logger';
import type { UseFileWatchingReturn, FileChangeEvent } from '../../types/fileTree';

export function useFileWatching(): UseFileWatchingReturn {
  // ============================
  // State
  // ============================

  const isWatching = ref<boolean>(false);
  const watcherId = ref<string | null>(null);
  const unlistenFn = ref<UnlistenFn | null>(null);

  // ============================
  // File Change Handlers
  // ============================

  const fileChangeHandlers = ref<Set<(event: FileChangeEvent) => void>>(new Set());

  // ============================
  // File Watching Operations
  // ============================

  const startWatching = async (path: string, recursive: boolean = true): Promise<void> => {
    if (isWatching.value) {
      await stopWatching();
    }

    try {
      // Start the file watcher in the backend
      const id = await invoke<string>('start_file_watcher', {
        path,
        recursive,
      });

      watcherId.value = id;
      isWatching.value = true;

      // Listen for file change events from the backend
      unlistenFn.value = await listen<FileChangeEvent>('file:changed', (event) => {
        handleFileChange(event.payload);
      });
    } catch (err) {
      const errorMessage = `Failed to start file watcher: ${err}`;
      error(errorMessage);
      throw new Error(errorMessage);
    }
  };

  const stopWatching = async (): Promise<void> => {
    if (!isWatching.value || !watcherId.value) {
      return;
    }

    try {
      // Stop the file watcher in the backend
      await invoke('stop_file_watcher', {
        watcher_id: watcherId.value,
      });

      // Stop listening for events
      if (unlistenFn.value) {
        unlistenFn.value();
        unlistenFn.value = null;
      }

      watcherId.value = null;
      isWatching.value = false;
    } catch (err) {
      const errorMessage = `Failed to stop file watcher: ${err}`;
      error(errorMessage);
      throw new Error(errorMessage);
    }
  };

  // ============================
  // Event Handling
  // ============================

  const handleFileChange = (event: FileChangeEvent): void => {
    debug(`File change event: ${event.change_type} - ${event.path}`);

    // Notify all registered handlers
    fileChangeHandlers.value.forEach((handler) => {
      try {
        handler(event);
      } catch (err) {
        error(`Error in file change handler: ${err}`);
      }
    });
  };

  const onFileChange = (callback: (event: FileChangeEvent) => void): void => {
    fileChangeHandlers.value.add(callback);

    // Return a cleanup function
    return () => {
      fileChangeHandlers.value.delete(callback);
    };
  };

  // ============================
  // Lifecycle Management
  // ============================

  onMounted(() => {
    debug('File watching composable mounted');
  });

  onUnmounted(async () => {
    debug('File watching composable unmounting');

    if (isWatching.value) {
      try {
        await stopWatching();
      } catch (err) {
        error(`Error stopping file watcher on unmount: ${err}`);
      }
    }

    // Clear all handlers
    fileChangeHandlers.value.clear();
  });

  // ============================
  // Public API
  // ============================

  return {
    isWatching,
    watcherId,
    startWatching,
    stopWatching,
    onFileChange,
  };
}
