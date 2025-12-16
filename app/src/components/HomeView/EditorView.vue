<template>
  <div style="height: 100%; background-color: #1e1e1e; display: flex">
    <!-- Main Editor Area -->
    <div style="width: 100%">
      <n-tabs
        v-if="openFiles.length > 0"
        v-model:value="nTabsValue"
        type="card"
        closable
        tab-style="min-width: 100px;"
        style="height: 100%"
        pane-style="flex-grow: 1; padding: 0; overflow: hidden;"
        @close="handleCloseTab"
        @update:value="handleTabChange"
      >
        <n-tab-pane
          v-for="file in openFiles"
          :key="file.path"
          :name="file.path"
          :tab="file.name + (file.isDirty ? '*' : '')"
          display-directive="show"
        >
          <div style="display: flex; flex-direction: column; height: 100%">
            <!-- Editor Tab Menu -->
            <EditorTabMenu
              :filePath="file.path"
              :fileContent="file.content || ''"
              :language="file.language"
              :isDirty="file.isDirty"
              :viewerType="file.viewerType"
              @save="handleSaveFile(file.path)"
              :onRunFile="() => handleRunFile(file.path)"
              :onRunAllCells="
                file.viewerType === 'notebook' ? () => handleRunAllCells(file.path) : undefined
              "
              @runStarted="handleRunStarted(file.path)"
              @runCompleted="handleRunCompleted(file.path)"
              @runError="handleRunError"
              @runOutput="handleRunOutput"
            />

            <!-- File Content -->
            <div style="flex: 1; overflow: hidden">
              <n-spin :show="file.loading" style="width: 100%; height: 100%">
                <!-- Monaco Editor for text files -->
                <MonacoEditorInstance
                  v-if="file.viewerType === 'monaco' && !file.loading && file.content !== null"
                  :value="file.content!"
                  :filePath="file.path"
                  :language="file.language"
                  :readOnly="file.readOnly"
                  :currentProjectRoot="appStore.projectPath || undefined"
                  :pendingNavigation="activeTab === file.path ? pendingNavigationRequest : null"
                  :theme="settingsStore.getEditorColorScheme()"
                  style="width: 100%; height: 100%"
                  :ref="
                    (el) => {
                      if (el && 'focus' in el && 'getCurrentValue' in el) {
                        editorRefs[file.path] = el as MonacoEditorInstance;
                      }
                    }
                  "
                  @contentChanged="handleContentChanged(file.path)"
                  @openFileAndNavigate="handleOpenFileAndNavigate"
                  @navigationComplete="handleNavigationComplete"
                  @findReferences="handleFindReferences"
                  @gotoDefinition="handleGotoDefinition"
                  @save="handleSaveCurrentFile"
                ></MonacoEditorInstance>

                <!-- Image Viewer for image files -->
                <ImageViewer
                  v-if="file.viewerType === 'image'"
                  :filePath="file.path"
                  :fileName="file.name"
                  :projectPath="appStore.projectPath || undefined"
                  style="width: 100%; height: 100%"
                />

                <!-- CSV Viewer for CSV files -->
                <CsvViewer
                  v-if="file.viewerType === 'csv'"
                  :filePath="file.path"
                  :fileName="file.name"
                  :projectPath="appStore.projectPath || undefined"
                  style="width: 100%; height: 100%"
                />

                <!-- Notebook Viewer for Jupyter notebook files -->
                <NotebookViewer
                  v-if="file.viewerType === 'notebook'"
                  :filePath="file.path"
                  :fileName="file.name"
                  :projectPath="appStore.projectPath || undefined"
                  style="width: 100%; height: 100%"
                  @dirty="(dirty) => handleNotebookDirty(file.path, dirty)"
                  :ref="
                    (el) => {
                      if (el) {
                        notebookRefs[file.path] = el as InstanceType<typeof NotebookViewer>;
                      }
                    }
                  "
                />

                <!-- Document Viewer for document files -->
                <div
                  v-if="file.viewerType === 'document'"
                  style="
                    width: 100%;
                    height: 100%;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                  "
                >
                  <n-empty description="Document viewer not yet implemented">
                    <template #extra>
                      <n-text depth="3"
                        >PDF and other document formats will be supported in a future
                        update.</n-text
                      >
                    </template>
                  </n-empty>
                </div>

                <!-- Binary file viewer -->
                <div
                  v-if="file.viewerType === 'binary'"
                  style="
                    width: 100%;
                    height: 100%;
                    display: flex;
                    align-items: center;
                    justify-content: center;
                  "
                >
                  <n-empty description="Binary file">
                    <template #extra>
                      <n-text depth="3"
                        >This is a binary file and cannot be displayed as text.</n-text
                      >
                    </template>
                  </n-empty>
                </div>

                <!-- Error state -->
                <n-empty
                  v-if="file.error"
                  description="Failed to load file content."
                  style="height: 100%; display: flex; justify-content: center; align-items: center"
                >
                  <template #extra>
                    <n-button size="small" @click="retryLoad(file.path)">Retry</n-button>
                  </template>
                </n-empty>
              </n-spin>
            </div>
          </div>
        </n-tab-pane>
      </n-tabs>
      <n-empty
        v-else
        description="No files open"
        style="height: 100%; display: flex; justify-content: center; align-items: center"
      >
        <template #extra> Select a file from the explorer to open it. </template>
      </n-empty>
    </div>

    <!-- References Panel - Now handled by right pane -->
  </div>
</template>

<script setup lang="ts">
import { ref, watch, nextTick, onMounted, onUnmounted, onActivated, computed } from 'vue';
import { NTabs, NTabPane, NSpin, NEmpty, NButton, NText } from 'naive-ui';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
// import { basename } from '@tauri-apps/api/path'; // Removed basename import
import * as monaco from 'monaco-editor';
import type { IRange } from 'monaco-editor';
import MonacoEditorInstance from './MonacoEditorInstance.vue';
import EditorTabMenu from './EditorTabMenu.vue';
import ImageViewer from './ImageViewer.vue';
import CsvViewer from './CsvViewer.vue';
import NotebookViewer from './NotebookViewer.vue';
// import ReferencesPanel from '../shared/ReferencesPanel.vue';
import { getViewerType } from '../../utils/fileTypeUtils';
import { useAppStore } from '../../store/appStore'; // <-- NEW: Import appStore
import { useSettingsStore } from '../../store/settingsStore';
import { debug, error, logObject } from '../../utils/logger';
import { primaryColor } from '../../theme';
import { lspService } from '../../services/lspService';
import { tabService } from '../../services/tabService';
import { useRoute } from 'vue-router';
import { debounce } from 'lodash-es';

const appStore = useAppStore(); // <-- NEW: Initialize appStore
const settingsStore = useSettingsStore();
const route = useRoute();

const runningFile = ref<string | null>(null);

// (NEW) Interface for open file objects
interface OpenFile {
  path: string;
  name: string;
  content: string | null;
  language: string;
  loading: boolean;
  error: boolean;
  readOnly: boolean;
  isDirty: boolean;
  viewerType: 'monaco' | 'image' | 'document' | 'binary' | 'csv' | 'notebook';
}

// Method to close all tabs
const closeAllTabs = async () => {
  debug('EditorView: closeAllTabs called');
  await tabService.clearTabs();
  openFiles.value = [];
  activeTab.value = null;
  debug('EditorView: All tabs closed');
};

// Close tabs that belong to a specific project root
const closeTabsForProject = async (projectRoot: string | null): Promise<void> => {
  if (!projectRoot) {
    return;
  }

  // Normalize and ensure project root ends with a path separator for accurate matching
  let normalizedProjectRoot = normalizePath(projectRoot);
  if (!normalizedProjectRoot.endsWith('/')) {
    normalizedProjectRoot += '/';
  }

  const tabsToClose: string[] = [];

  // Find all tabs that belong to the old project (create a snapshot to avoid modification during iteration)
  for (const file of [...openFiles.value]) {
    if (file.path) {
      const normalizedFilePath = normalizePath(file.path);
      // Check if file path starts with project root (with separator) to avoid false matches
      // e.g., "c:/projects/myproject" should not match "c:/projects/myproject2/file.jl"
      if (normalizedFilePath.startsWith(normalizedProjectRoot)) {
        tabsToClose.push(file.path);
      }
    }
  }

  if (tabsToClose.length === 0) {
    debug(`EditorView: No tabs to close for project ${projectRoot}`);
    return;
  }

  debug(`EditorView: Closing ${tabsToClose.length} tabs for project ${projectRoot}`);

  // Close each tab (this will handle both frontend and backend cleanup)
  // We close tabs in reverse order to avoid index shifting issues
  for (let i = tabsToClose.length - 1; i >= 0; i--) {
    const filePath = tabsToClose[i];
    // Check if the tab still exists before closing (it might have been closed already)
    const stillExists = openFiles.value.some((f) => pathsMatch(f.path, filePath));
    if (stillExists) {
      await handleCloseTab(filePath);
    }
  }

  debug(`EditorView: Closed tabs for project ${projectRoot}`);
};

const openFiles = ref<OpenFile[]>([]); // Typed array
const activeTab = ref<string | null>(null); // Holds the path of the active tab

// Computed property for n-tabs v-model
const nTabsValue = computed({
  get: () => activeTab.value || undefined,
  set: (val) => {
    activeTab.value = val || null;
  },
});

interface MonacoEditorInstance {
  focus: () => void;
  getCurrentValue: () => string;
  setExecutionLine?: (lineNumber: number | null) => void;
  navigateToLine?: (lineNumber: number) => void;
}

const editorRefs = ref<Record<string, MonacoEditorInstance | null>>({}); // Refs to editor instances
const notebookRefs = ref<Record<string, InstanceType<typeof NotebookViewer> | null>>({}); // Refs to notebook instances

// (NEW) Ref to store pending navigation requests for "Go to Definition"
const pendingNavigationRequest = ref<{ filePath: string; range: IRange } | null>(null); // Used IRange from monaco

// References panel state
const showReferencesPanel = ref(false);
const referencesRequest = ref<{ uri: string; line: number; character: number } | null>(null);

// Watch for route changes to restore tabs when returning to Home
watch(
  () => route.name,
  async (newRouteName, oldRouteName) => {
    // When returning to Home route, restore tabs from service
    // oldRouteName can be undefined on initial mount, or a route name when navigating
    if (newRouteName === 'Home') {
      if (oldRouteName === undefined || oldRouteName !== 'Home') {
        // Always reload tabs when returning to Home (component may have been unmounted)
        await loadTabsFromService();
      }
    }
  },
  { immediate: true }
);

// Also restore tabs when component is activated (in case keep-alive is used)
onActivated(async () => {
  if (route.name === 'Home') {
    await loadTabsFromService();
  }
});

// LSP related setup
onMounted(async () => {
  // Load existing tabs from the service
  // Note: This is also handled by the route watcher, but we keep it here for safety
  if (route.name === 'Home') {
    await loadTabsFromService();
  }

  // Initial file open if present in store (e.g. app was reloaded with a file already intended to be open)
  if (appStore.fileToOpen) {
    await openFile(appStore.fileToOpen);

    // Check if there's a pending navigation request in the app store
    if (appStore.pendingNavigationRequest) {
      pendingNavigationRequest.value = {
        filePath: appStore.fileToOpen,
        range: appStore.pendingNavigationRequest,
      };
      // Clear the navigation request from the app store
      appStore.setPendingNavigationRequest(null);
    }

    appStore.setFileToOpen(null); // Clear after attempting to open
  }

  // Set up listener for Julia execution completion events
  const unlistenExecutionComplete = await listen('system:backend-done', (event) => {
    if (event.payload && typeof event.payload === 'object') {
      const payload = event.payload as any; // Type assertion for dynamic payload
      const requestId = payload.request_id;

      debug(`EditorView: Received backend done event - requestId: ${requestId}`);

      // Reset the running file state when backend is done
      debug('EditorView: Backend execution completed, resetting button state');
      runningFile.value = null; // Reset the running file
    }
  });

  // Set up listener for file deletion events from FileExplorer
  const handleFileDeleted = async (event: CustomEvent<{ filePath: string }>) => {
    const deletedFilePath = event.detail.filePath;

    // Check if the file is open as a tab
    const fileIndex = openFiles.value.findIndex((f) => pathsMatch(f.path, deletedFilePath));
    if (fileIndex !== -1) {
      await handleCloseTab(deletedFilePath);
    }
  };

  window.addEventListener('file-deleted', handleFileDeleted as EventListener);

  // Clean up all listeners on unmount
  onUnmounted(() => {
    unlistenExecutionComplete();
    window.removeEventListener('file-deleted', handleFileDeleted as EventListener);
  });

  // Let DiagnosticsPanel know initial active file
  if (activeTab.value) {
    window.dispatchEvent(
      new CustomEvent('active-file-changed', { detail: { filePath: activeTab.value } })
    );
  }
});

onUnmounted(() => {
  // Cancel any pending debounced tab content updates
  debouncedUpdateTabContent.cancel();
  // Cleanup tab service listeners
  tabService.cleanup();
});

// Watch for project changes from Pinia store
watch(
  () => appStore.projectPath,
  async (newProjectRoot, oldProjectRoot) => {
    debug(
      `EditorView LSP Watcher: Project changed from '${oldProjectRoot}' to '${newProjectRoot}'`
    );

    // Close tabs from the old project when switching to a new project
    if (oldProjectRoot && newProjectRoot && oldProjectRoot !== newProjectRoot) {
      await closeTabsForProject(oldProjectRoot);
    } else if (oldProjectRoot && !newProjectRoot) {
      // Project was closed, close all tabs
      await closeTabsForProject(oldProjectRoot);
    }

    // No need to explicitly shut down or initialize - backend handles this automatically
    if (newProjectRoot) {
      const serviceActiveProject = appStore.projectPath; // Use appStore.projectPath directly
      if (serviceActiveProject !== newProjectRoot) {
        debug(
          `LSP Watcher: New project detected: ${newProjectRoot}. (lspService active: ${serviceActiveProject})`
        );
        debug(`LSP Watcher: Waiting for 'lsp_server_initialized' event from backend.`);
      } else {
        debug(
          `LSP Watcher: New project ${newProjectRoot} is already considered active by lspService.`
        );
      }
    } else {
      // No new project (project was closed)
      debug('LSP Watcher: Project closed. Backend will handle LSP client cleanup automatically.');
    }
  },
  { immediate: true }
);

// Watch for file open requests from Pinia store
watch(
  () => appStore.fileToOpen,
  async (newFilePath) => {
    if (newFilePath) {
      await openFile(newFilePath);

      // Check if there's a pending navigation request in the app store
      if (appStore.pendingNavigationRequest) {
        pendingNavigationRequest.value = {
          filePath: newFilePath,
          range: appStore.pendingNavigationRequest,
        };
        // Clear the navigation request from the app store
        appStore.setPendingNavigationRequest(null);
      }

      // It's important to clear this so it can be triggered again with the same path if needed
      // or if the user closes the tab and clicks the file in explorer again.
      appStore.setFileToOpen(null);
    }
  }
);

// Watch for activeTab changes
watch(
  () => activeTab.value,
  async (newActiveTab, oldActiveTab) => {
    // Emit global event so DiagnosticsPanel knows current file
    if (newActiveTab) {
      window.dispatchEvent(
        new CustomEvent('active-file-changed', { detail: { filePath: newActiveTab } })
      );
    }
  }
);

// Map file extensions to Monaco language IDs
const getLanguageFromPath = (filePath: string): string => {
  const extension = filePath?.split('.').pop()?.toLowerCase();
  switch (extension) {
    case 'js':
      return 'javascript';
    case 'ts':
      return 'typescript';
    case 'json':
      return 'json';
    case 'html':
      return 'html';
    case 'css':
      return 'css';
    case 'py':
      return 'python';
    case 'rs':
      return 'rust';
    case 'jl':
      return 'julia';
    case 'md':
      return 'markdown';
    case 'java':
      return 'java';
    case 'c':
      return 'c';
    case 'cpp':
      return 'cpp';
    case 'go':
      return 'go';
    case 'php':
      return 'php';
    case 'sh':
      return 'shell';
    case 'yaml':
    case 'yml':
      return 'yaml';
    case 'toml':
      return 'toml';
    default:
      return 'plaintext';
  }
};

// Helper function for normalized path comparison
const normalizePath = (path: string): string => {
  return path.toLowerCase().replace(/\\/g, '/');
};

const pathsMatch = (path1: string, path2: string): boolean => {
  return normalizePath(path1) === normalizePath(path2);
};

// Function to fetch file content
async function loadFileContent(filePath: string): Promise<void> {
  const fileIndex = openFiles.value.findIndex((f) => pathsMatch(f.path, filePath));
  if (fileIndex === -1) return;

  // Safety check: Never reload content for dirty files (would lose unsaved changes)
  if (openFiles.value[fileIndex].isDirty) {
    await logObject('warn', 'EditorView: Skipping loadFileContent for dirty file', { filePath });
    return;
  }

  // Skip loading content for image and notebook files - their viewers handle this directly
  const viewerType = getViewerType(filePath);
  if (viewerType === 'image' || viewerType === 'notebook') {
    openFiles.value[fileIndex].loading = false;
    openFiles.value[fileIndex].error = false;
    openFiles.value[fileIndex].isDirty = false;
    return;
  }

  openFiles.value[fileIndex].loading = true;
  openFiles.value[fileIndex].error = false;
  openFiles.value[fileIndex].isDirty = false;

  try {
    const content = await invoke<string>('read_file_content', { path: filePath });
    openFiles.value[fileIndex].content = content;

    // Update tab content in backend if tab exists
    // Backend will automatically check if content matches file and set is_dirty accordingly
    const tab = tabService.getTabByPath(filePath);
    if (tab) {
      await tabService.updateTabContent(tab.id, content);
      // Sync dirty state from backend after update
      await nextTick();
      const updatedTab = tabService.getTabByPath(filePath);
      if (updatedTab) {
        const fileEntry = openFiles.value.find((f) => pathsMatch(f.path, filePath));
        if (fileEntry) {
          fileEntry.isDirty = updatedTab.is_dirty;
        }
      }
    }
  } catch (err) {
    await logObject('error', 'EditorView: Failed to load content for', { filePath, error: err });
    openFiles.value[fileIndex].content = null;
    openFiles.value[fileIndex].error = true;
  } finally {
    openFiles.value[fileIndex].loading = false;
  }
}

// Flag to prevent concurrent calls to loadTabsFromService
let isLoadingTabs = false;

// Function to load tabs from the service
async function loadTabsFromService(): Promise<void> {
  // Prevent concurrent calls
  if (isLoadingTabs) {
    await logObject('warn', 'EditorView: loadTabsFromService already in progress, skipping', {});
    return;
  }

  isLoadingTabs = true;
  try {
    const tabs = await tabService.getTabs();

    // Track which files we've already processed to avoid duplicates
    const processedPaths = new Set<string>();
    const newOpenFiles: OpenFile[] = [];

    for (const tab of tabs) {
      if (tab.path) {
        // Normalize path for comparison
        const normalizedPath = normalizePath(tab.path);

        // Skip if we've already processed this file path
        if (processedPaths.has(normalizedPath)) {
          continue;
        }
        processedPaths.add(normalizedPath);

        const fileName = tab.path.split(/[\/\\]/).pop() || tab.path;
        const language = getLanguageFromPath(tab.path);
        const viewerType = getViewerType(tab.path);

        const hasContent = tab.content && tab.content.length > 0;
        const needsReload = !hasContent && !tab.is_dirty;

        const openFile: OpenFile = {
          path: tab.path,
          name: fileName,
          content: tab.content || null, // Use null if content is empty, so we reload it
          language: language,
          loading: needsReload, // Only mark as loading if we need to reload from disk
          error: false,
          readOnly: false,
          isDirty: tab.is_dirty,
          viewerType: viewerType,
        };

        newOpenFiles.push(openFile);
      }
    }

    // Replace openFiles atomically to prevent race conditions
    // Do this BEFORE calling loadFileContent so it can find the files in the new array
    openFiles.value = newOpenFiles;

    // Now load content for tabs that need it (after openFiles is set)
    // Process each unique tab (skip duplicates)
    for (const tab of tabs) {
      if (!tab.path) continue;

      const normalizedPath = normalizePath(tab.path);
      // Only process if this path was actually added (not a duplicate)
      if (!processedPaths.has(normalizedPath)) {
        continue;
      }

      const hasContent = tab.content && tab.content.length > 0;
      const needsReload = !hasContent && !tab.is_dirty;

      // If content is empty or missing and tab is NOT dirty, reload it from file
      // For dirty tabs, we always use the backend content (even if empty - user might have cleared all text)
      if (needsReload) {
        // Load content from disk and update backend
        // Backend will automatically check if content matches file and set is_dirty accordingly
        await loadFileContent(tab.path);
      }
    }

    // Set active tab if there are tabs
    if (openFiles.value.length > 0) {
      const activeTabId = tabService.getActiveTabId();
      if (activeTabId) {
        // Try to find tab by ID first
        let foundTab = tabs.find((tab) => tab.id === activeTabId);

        // If not found by ID, it might be a file path (legacy)
        if (!foundTab) {
          foundTab = tabs.find((tab) => tab.path && pathsMatch(tab.path, activeTabId));
        }

        if (foundTab && foundTab.path) {
          const fileEntry = openFiles.value.find((f) => pathsMatch(f.path, foundTab.path!));
          if (fileEntry) {
            activeTab.value = fileEntry.path;
            // Update active tab ID to use the proper tab ID
            tabService.setActiveTab(foundTab.id);
          }
        } else {
          // Fallback to first tab if active tab not found
          activeTab.value = openFiles.value[0].path;
        }
      } else {
        // No active tab set, use first tab
        activeTab.value = openFiles.value[0].path;
      }
    }

    debug(`EditorView: Loaded ${openFiles.value.length} tabs from service`);
  } catch (err) {
    error(`EditorView: Failed to load tabs from service: ${err}`);
  } finally {
    isLoadingTabs = false;
  }
}

// Function to open a file (or switch to it if already open)
async function openFile(filePath: string): Promise<void> {
  if (!filePath) return;

  const existingFileIndex = openFiles.value.findIndex((f) => pathsMatch(f.path, filePath));
  let fileEntry = existingFileIndex !== -1 ? openFiles.value[existingFileIndex] : null;

  if (fileEntry) {
    // Use the existing file's path format to prevent component recreation
    activeTab.value = fileEntry.path;
    tabService.setActiveTab(fileEntry.path);
    await nextTick();
    editorRefs.value[fileEntry.path]?.focus();
    await syncFileDirtyState(fileEntry.path);
    return; // Add return statement to prevent fall-through
  } else {
    let fileName = 'Untitled';
    fileName = filePath.split(/[\/\\]/).pop() || filePath;

    const language = getLanguageFromPath(filePath);
    const viewerType = getViewerType(filePath);

    // Add tab to the service first
    const tabId = await tabService.addTab({
      title: fileName,
      path: filePath,
      content: '',
      is_dirty: false,
    });

    const newFile = {
      path: filePath,
      name: fileName,
      content: null,
      language: language,
      loading: true,
      error: false,
      readOnly: false,
      isDirty: false,
      viewerType: viewerType,
    };
    openFiles.value.push(newFile);
    activeTab.value = filePath;
    tabService.setActiveTab(tabId);

    await loadFileContent(filePath);
    await syncFileDirtyState(filePath);

    // Notify LSP server that the file has been opened (skip image and notebook files)
    const fileData = openFiles.value.find((f) => pathsMatch(f.path, filePath));
    const fileViewerType = getViewerType(filePath);
    if (fileData?.content && fileViewerType !== 'image' && fileViewerType !== 'notebook') {
      await notifyFileOpened(filePath, fileData.language, fileData.content);
    }

    await nextTick();
    editorRefs.value[filePath]?.focus();
  }

  if (openFiles.value.length === 0) {
    activeTab.value = null;
  }
}

// (NEW) Function to notify backend that a file was opened
async function notifyFileOpened(
  filePath: string,
  languageId: string,
  content: string
): Promise<void> {
  // Convert to Monaco file URI so future hover/definition requests match the same identifier
  const uri = monaco.Uri.file(filePath).toString();
  try {
    await lspService.notifyDidOpen(uri, content, languageId);
  } catch (err) {
    await logObject('error', '[LSP DEBUG] Failed to notify LSP of document open', {
      uri,
      error: String(err),
    });
  }
}

// Handle closing a tab
const handleCloseTab = async (filePath: string): Promise<void> => {
  const index = openFiles.value.findIndex((f) => pathsMatch(f.path, filePath));
  if (index !== -1) {
    const closedFile = openFiles.value[index];

    // Remove from tab service
    const tab = tabService.getTabByPath(filePath);
    if (tab) {
      await tabService.removeTab(tab.id);
    }

    openFiles.value.splice(index, 1);
    delete editorRefs.value[filePath];

    if (activeTab.value === filePath) {
      if (openFiles.value.length > 0) {
        const nextIndex = Math.max(0, index - 1);
        activeTab.value = openFiles.value[nextIndex].path;
        tabService.setActiveTab(openFiles.value[nextIndex].path);
      } else {
        activeTab.value = null;
        tabService.setActiveTab(null);
      }
    }
  }
};

// Handle tab switching to potentially focus editor
const handleTabChange = async (newActiveTabPath: string | null): Promise<void> => {
  if (!newActiveTabPath) {
    activeTab.value = null;
    tabService.setActiveTab(null);
    return;
  }

  activeTab.value = newActiveTabPath;
  tabService.setActiveTab(newActiveTabPath);

  // Only sync dirty state if we have a valid file path
  if (typeof newActiveTabPath === 'string') {
    await syncFileDirtyState(newActiveTabPath);
  }

  // Focus the editor after tab change
  await nextTick();
  const editorRef = editorRefs.value[newActiveTabPath];
  if (editorRef?.focus) {
    editorRef.focus();
  }
};

// Retry loading file content
const retryLoad = async (filePath: string): Promise<void> => {
  const fileEntry = openFiles.value.find((f) => pathsMatch(f.path, filePath));
  if (fileEntry) {
    fileEntry.error = false;
    fileEntry.loading = true;
    await loadFileContent(filePath);
  } else {
    await logObject('error', 'EditorView: Cannot retry load, file not found in openFiles', {
      filePath,
    });
  }
};

// Placeholder for a save function that could be triggered
// Removed unused handleFileSave

// Function to mark a file as dirty
const markFileDirty = async (filePath: string): Promise<void> => {
  const file = openFiles.value.find((f) => pathsMatch(f.path, filePath));
  if (file && !file.isDirty) {
    file.isDirty = true;
  }
};

// Function to mark a file as clean
const markFileClean = async (filePath: string): Promise<void> => {
  const file = openFiles.value.find((f) => pathsMatch(f.path, filePath));
  if (file && file.isDirty) {
    file.isDirty = false;
  }
};

// Function to sync dirty state with backend
// Gets dirty state from tabService which already has the latest state from the backend
const syncFileDirtyState = async (filePath: string | null): Promise<void> => {
  if (!filePath) return;

  try {
    // Get dirty state from tabService - it already has the latest state from backend
    const tab = tabService.getTabByPath(filePath);
    if (tab) {
      const file = openFiles.value.find((f) => pathsMatch(f.path, filePath));
      if (file) {
        file.isDirty = tab.is_dirty;
      }
    } else {
      // Tab not found in service, refresh tabs to get latest state
      await tabService.getTabs();
      const refreshedTab = tabService.getTabByPath(filePath);
      if (refreshedTab) {
        const file = openFiles.value.find((f) => pathsMatch(f.path, filePath));
        if (file) {
          file.isDirty = refreshedTab.is_dirty;
        }
      }
    }
  } catch (err) {
    await logObject('error', 'EditorView: Failed to sync dirty state for', {
      filePath,
      error: err,
    });
  }
};

// Debounced function to update tab content in backend
// This prevents excessive backend calls during rapid typing
const debouncedUpdateTabContent = debounce(async (filePath: string) => {
  const tab = tabService.getTabByPath(filePath);
  if (tab) {
    const fileIndex = openFiles.value.findIndex((f) => pathsMatch(f.path, filePath));
    if (fileIndex !== -1) {
      const editorRef = editorRefs.value[filePath];
      if (editorRef && editorRef.getCurrentValue) {
        const newContent = editorRef.getCurrentValue();
        await tabService.updateTabContent(tab.id, newContent);
      }
    }
  }
}, 500); // 500ms debounce - same as LSP didChange notifications

// Handle contentChanged event from MonacoEditorInstance
// This is called immediately on every keystroke
const handleContentChanged = async (filePath: string): Promise<void> => {
  // Mark as dirty immediately (for UI feedback)
  await markFileDirty(filePath);

  // Update in-memory content immediately (so tab switching works correctly)
  const fileIndex = openFiles.value.findIndex((f) => pathsMatch(f.path, filePath));
  if (fileIndex !== -1) {
    const editorRef = editorRefs.value[filePath];
    if (editorRef && editorRef.getCurrentValue) {
      const newContent = editorRef.getCurrentValue();
      // Keep in-memory openFiles content in sync so unsaved edits persist across tab switches
      openFiles.value[fileIndex].content = newContent;
    }
  }

  // Debounce the backend update to avoid excessive calls during rapid typing
  debouncedUpdateTabContent(filePath);
};

// Handle saving the current file
const handleSaveCurrentFile = async (): Promise<void> => {
  if (!activeTab.value) return;
  const currentFile = openFiles.value.find((f) => {
    // Normalize both paths for comparison
    const normalizedExistingPath = normalizePath(f.path);
    const normalizedActivePath = activeTab.value ? normalizePath(activeTab.value) : '';
    return normalizedExistingPath === normalizedActivePath;
  });
  if (!currentFile || currentFile.readOnly) {
    await logObject('info', 'EditorView: No active file to save or file is read-only', {});
    return;
  }

  try {
    const editorComponent = activeTab.value ? editorRefs.value[activeTab.value] : null;
    if (!editorComponent || typeof editorComponent.getCurrentValue !== 'function') {
      await logObject('error', 'EditorView: Cannot get current editor value to save', {});
      return;
    }
    const contentToSave = editorComponent.getCurrentValue();

    if (activeTab.value) {
      await logObject('info', 'EditorView: Saving file', { filePath: activeTab.value });
      await invoke('write_file_content', { path: activeTab.value, content: contentToSave });
      await markFileClean(activeTab.value);

      // Notify LSP of document save to trigger diagnostics
      try {
        const uri = monaco.Uri.file(activeTab.value).toString();
        await invoke('lsp_notify_did_save', { uri });
      } catch (e) {
        await logObject('warn', 'EditorView: Failed to notify LSP of document save', { error: e });
      }

      // Update tab in service
      const tab = tabService.getTabByPath(activeTab.value);
      if (tab) {
        await tabService.saveTabToFile(tab.id);
      }
    }
  } catch (err) {
    if (activeTab.value) {
      await logObject('error', 'EditorView: Failed to save file', {
        filePath: activeTab.value,
        error: err,
      });
    }
  }
};

// Example: Listen for Ctrl+S or Cmd+S for saving
onMounted(() => {
  const handleKeyDown = (event: KeyboardEvent): void => {
    if ((event.ctrlKey || event.metaKey) && event.key === 's') {
      event.preventDefault();
      handleSaveCurrentFile();
    }
  };
  window.addEventListener('keydown', handleKeyDown);
  onUnmounted(() => {
    window.removeEventListener('keydown', handleKeyDown);
  });
});

// Expose methods to parent
defineExpose({
  openFile,
  closeAllTabs,
});

// Handle the openFileAndNavigate event from MonacoEditorInstance
const handleOpenFileAndNavigate = async (event: {
  filePath: string;
  range: IRange;
}): Promise<void> => {
  await openFile(event.filePath);
  await nextTick();
  pendingNavigationRequest.value = { filePath: event.filePath, range: event.range };
};

// Handle the navigationComplete event from MonacoEditorInstance
const handleNavigationComplete = async (): Promise<void> => {
  pendingNavigationRequest.value = null;
};

// Handle notebook dirty state changes
const handleNotebookDirty = async (filePath: string, dirty: boolean): Promise<void> => {
  const file = openFiles.value.find((f) => pathsMatch(f.path, filePath));
  if (file) {
    file.isDirty = dirty;
    // Update tab service
    const tab = tabService.getTabByPath(filePath);
    if (tab) {
      await tabService.updateTab(tab.id, { ...tab, is_dirty: dirty });
    }
  }
};

// Handle run all cells for notebook
const handleRunAllCells = async (filePath: string): Promise<void> => {
  const notebook = notebookRefs.value[filePath];
  if (notebook && 'runAllCells' in notebook) {
    await (notebook as any).runAllCells();
  }
};

// Handle save file from EditorTabMenu
const handleSaveFile = async (filePath: string): Promise<void> => {
  await logObject('info', 'EditorView: Save requested from menu for', { filePath });
  const file = openFiles.value.find((f) => pathsMatch(f.path, filePath));
  if (!file) {
    await logObject('error', 'EditorView: File not found for save', { filePath });
    return;
  }

  // Skip save for notebook files - they handle saving themselves
  if (file.viewerType === 'notebook') {
    await logObject(
      'info',
      'EditorView: Skipping save for notebook file (handled by NotebookViewer)',
      { filePath }
    );
    return;
  }

  try {
    const editorComponent = editorRefs.value[filePath];
    if (!editorComponent || typeof editorComponent.getCurrentValue !== 'function') {
      await logObject('error', 'EditorView: Cannot get current editor value to save', { filePath });
      return;
    }
    const contentToSave = editorComponent.getCurrentValue();

    await invoke('write_file_content', { path: filePath, content: contentToSave });
    await markFileClean(filePath);

    // Notify LSP of document save to trigger diagnostics
    try {
      const uri = monaco.Uri.file(filePath).toString();
      await invoke('lsp_notify_did_save', { uri });
    } catch (e) {
      await logObject('warn', 'EditorView: Failed to notify LSP of document save', { error: e });
    }

    // Update tab in service
    const tab = tabService.getTabByPath(filePath);
    if (tab) {
      await tabService.saveTabToFile(tab.id);
    }
  } catch (err) {
    await logObject('error', 'EditorView: Failed to save file from menu', { filePath, error: err });
  }
};

// Handle run started from EditorTabMenu
const handleRunStarted = async (filePath: string): Promise<void> => {
  // Could add visual feedback here if needed
};

// Handle run completed from EditorTabMenu
const handleRunCompleted = async (filePath: string): Promise<void> => {
  // Could add success notification here if needed
};

// Handle run error from EditorTabMenu
const handleRunError = async (errorMsg: string): Promise<void> => {
  await logObject('error', 'EditorView: Run error', { errorMsg });
  // Could add error notification here if needed
};

// Handle run output from EditorTabMenu
const handleRunOutput = async (output: string): Promise<void> => {
  await logObject('info', 'EditorView: Run output received', { output });

  // Parse the output to extract stdout and stderr
  let stdoutOutput = '';
  let stderrOutput = '';

  // Try to parse as JSON first (new stream capture format)
  try {
    const resultObj = JSON.parse(output);
    if (resultObj && typeof resultObj === 'object') {
      stdoutOutput = resultObj.stdout || '';
      stderrOutput = resultObj.stderr || '';
      // If no stdout/stderr but we have a result, use that as stdout
      if (!stdoutOutput && !stderrOutput && resultObj.result) {
        stdoutOutput = resultObj.result;
      }
    } else {
      stdoutOutput = output; // Fallback to treating as plain string
    }
  } catch (e) {
    // Not JSON, try old format parsing
    if (output.startsWith('STDOUT:')) {
      stdoutOutput = output.substring(7); // Remove "STDOUT:" prefix
    } else if (output.startsWith('STDERR:')) {
      stderrOutput = output.substring(7); // Remove "STDERR:" prefix
    } else if (output.includes('STDERR:') && output.includes('STDOUT:')) {
      // Both stderr and stdout present
      const parts = output.split('\nSTDOUT:');
      if (parts.length === 2) {
        stderrOutput = parts[0].substring(7); // Remove "STDERR:" prefix
        stdoutOutput = parts[1];
      }
    } else if (output.startsWith('ERROR:')) {
      stderrOutput = output; // Treat as error output
    } else {
      stdoutOutput = output; // Default to stdout
    }
  }

  // Send the parsed output to the REPL terminal
  console.log('EditorView: Dispatching julia-file-output event with parsed output:', {
    stdoutOutput,
    stderrOutput,
  });
  window.dispatchEvent(
    new CustomEvent('julia-file-output', {
      detail: {
        stdoutOutput: stdoutOutput.trim(),
        stderrOutput: stderrOutput.trim(),
        filePath: window.currentExecutingFile || 'unknown file',
      },
    })
  );
};

// Function to handle running a file
const handleRunFile = async (filePath: string) => {
  const file = openFiles.value.find((f) => pathsMatch(f.path, filePath));
  if (!file || file.content === null) {
    error(`EditorView: Cannot run file, content not available for ${filePath}`);
    return;
  }

  debug(`EditorView: Running file ${filePath}`);
  runningFile.value = filePath; // Set the running file

  try {
    debug(`EditorView: Sending execute_julia_file command for ${filePath}`);
    await invoke('execute_julia_file', { filePath: file.path, fileContent: file.content });
    debug(`EditorView: Successfully requested to run file ${filePath}`);
    // Note: We don't reset runningFile.value here anymore - it will be reset when the completion event is received
  } catch (err) {
    const error = err as Error;
    logObject('error', `EditorView: Failed to run file ${filePath}`, {
      message: error.message,
      stack: error.stack,
    });
    runningFile.value = null; // Reset on error
  }
  // Removed the finally block that was immediately resetting runningFile.value
};

// Handle reference navigation from ReferencesPanel
// Removed handleReferenceNavigate body (unused)

// Handle find references request from Monaco editor
const handleFindReferences = async (request: {
  uri: string;
  line: number;
  character: number;
}): Promise<void> => {
  console.log('=== FIND REFERENCES CALLED ===', request);
  try {
    debug('EditorView: Find references requested');
    console.log('EditorView: handleFindReferences called with:', request);

    // Set up the references request
    referencesRequest.value = {
      uri: request.uri,
      line: request.line,
      character: request.character,
    };
    console.log('EditorView: referencesRequest set to:', referencesRequest.value);

    // Show the references panel in the right pane
    showReferencesPanel.value = true;
    console.log('EditorView: showReferencesPanel set to:', showReferencesPanel.value);

    // Emit event to show right pane with references
    console.log('EditorView: About to dispatch show-references-panel event');
    window.dispatchEvent(
      new CustomEvent('show-references-panel', {
        detail: {
          uri: request.uri,
          line: request.line,
          character: request.character,
        },
      })
    );
    console.log('EditorView: show-references-panel event dispatched successfully');
  } catch (err) {
    error('EditorView: Failed to handle find references request');
    console.error('EditorView: Error in handleFindReferences:', err);
  }
};

// Handle go to definition request from Monaco editor
const handleGotoDefinition = async (request: {
  uri: string;
  line: number;
  character: number;
}): Promise<void> => {
  try {
    const definitions = await lspService.requestDefinition(
      request.uri,
      request.line,
      request.character
    );

    if (definitions.length > 0) {
      // Navigate to the first definition
      const definition = definitions[0];

      // Fix URI handling - properly decode the URI and handle Windows paths
      let filePath = definition.uri;

      // First decode any URL-encoded characters
      filePath = decodeURIComponent(filePath);

      // Remove file:// protocol
      filePath = filePath.replace('file://', '');

      // Handle Windows paths with leading slash
      if (filePath.startsWith('/') && filePath.match(/^\/[A-Z]:/i)) {
        filePath = filePath.substring(1); // Remove leading slash
      }

      // Open the file if not already open
      await openFile(filePath);

      // Set up navigation to the specific location
      await nextTick();
      pendingNavigationRequest.value = {
        filePath: filePath,
        range: {
          startLineNumber: definition.range.start.line + 1,
          startColumn: definition.range.start.character + 1,
          endLineNumber: definition.range.end.line + 1,
          endColumn: definition.range.end.character + 1,
        },
      };
    }
  } catch (err) {
    await logObject('error', 'EditorView: Failed to handle go to definition request', {
      error: err,
      request: request,
    });
  }
};
</script>

<style scoped>
/* Ensure tabs and panes fill height */
:deep(.n-tabs .n-tabs-nav) {
  flex-shrink: 0; /* Prevent nav from shrinking */
}

:deep(.n-tabs .n-tab-pane) {
  height: 100%; /* Ensure pane fills space */
  width: 100%;
  /* Ensure direct children of tab-pane also expand if they are the primary content */
  /* This helps if n-spin is the direct child and needs to fill the pane */
  display: flex;
  flex-direction: column;
}

:deep(.n-tabs .n-tab-pane > .n-spin) {
  flex-grow: 1; /* Allow spin to grow and fill the pane */
  /* width: 100% is already applied via style attribute, height: 100% is also there */
}

/* Force the content wrapper inside n-spin to take full height and be a flex container */
:deep(.n-spin-container > .n-spin-content) {
  height: 100%;
  display: flex; /* Make it a flex container */
  flex-direction: column; /* Align children (MonacoEditorInstance) vertically */
}

/* Ensure MonacoEditorInstance itself (if it becomes a direct child of n-spin-content) expands */
:deep(.n-spin-content > *) {
  /* Target direct child of n-spin-content */
  flex-grow: 1; /* Allow it to grow */
  min-height: 0; /* Important for flex children to shrink properly if needed and not overflow */
}

/* Add any specific styles for EditorView here */
.n-tabs .n-tabs-pane-wrapper {
  overflow: hidden; /* Ensure editor pane does not cause outer scrollbars */
}

/* Ensure active tabs use our theme color for background */
:deep(.n-tabs .n-tabs-tab--active) {
  background-color: v-bind(primaryColor) !important;
  color: #ffffff !important;
}

:deep(.n-tabs .n-tabs-tab--active .n-tabs-tab__label) {
  color: #ffffff !important;
}
</style>
