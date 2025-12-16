import { defineStore } from 'pinia';
import { ref } from 'vue';
import { debug } from '../utils/logger';
import { invoke } from '@tauri-apps/api/core';
// import { invoke } from '@tauri-apps/api/core';

// Define the interface for the range object used in navigation requests
interface IRange {
  startLineNumber: number;
  startColumn: number;
  endLineNumber: number;
  endColumn: number;
}

// Interface for open file objects
interface OpenFile {
  path: string;
  name: string;
  content: string | null;
  language: string;
  loading: boolean;
  error: boolean;
  readOnly: boolean;
  isDirty: boolean;
  viewerType: 'monaco' | 'image' | 'document' | 'binary' | 'notebook';
}

// LSP Status interface
export interface LspStatus {
  status:
    | 'unknown'
    | 'starting'
    | 'started'
    | 'initialized'
    | 'loading-cache'
    | 'ready'
    | 'failed'
    | 'stopped';
  message: string;
  error?: string;
  projectPath?: string;
}

export const useAppStore = defineStore('app', () => {
  const projectPath = ref<string | null>(null);
  const fileToOpen = ref<string | null>(null);
  const isJuliaProject = ref<boolean>(false);
  const initialProjectLoadAttempted = ref<boolean>(false);
  const pendingNavigationRequest = ref<IRange | null>(null);
  const juliaDaemonReady = ref<boolean>(false);
  const backendBusy = ref<boolean>(false); // Track when backend is executing Julia code
  const fileServerPort = ref<number | null>(null); // File server port for CSV parsing
  const fileServerError = ref<string | null>(null); // File server error message (non-fatal)
  const lspStatus = ref<LspStatus>({
    status: 'unknown',
    message: 'Waiting for Language Server status...',
  });

  // Workspace variables (persisted globally so they're available even if Variables panel isn't mounted)
  const workspaceVariables = ref<Record<string, any>>({});

  // Editor tab state management
  const openFiles = ref<OpenFile[]>([]);
  const activeTab = ref<string | null>(null);

  async function setProjectPath(path: string | null) {
    projectPath.value = path;
    debug(`appStore: projectPath set to ${projectPath.value}`);

    // Note: Project changes are now handled entirely by the backend
    // The frontend only updates its display state
  }

  function setFileToOpen(path: string | null) {
    fileToOpen.value = path;
    debug(`appStore: fileToOpen set to ${fileToOpen.value}`);
  }

  function setIsJuliaProject(status: boolean) {
    isJuliaProject.value = status;
    debug(`appStore: isJuliaProject set to ${isJuliaProject.value}`);
  }

  function setInitialProjectLoadAttempted(status: boolean) {
    initialProjectLoadAttempted.value = status;
    debug(`appStore: initialProjectLoadAttempted set to ${initialProjectLoadAttempted.value}`);
  }

  function setPendingNavigationRequest(range: IRange | null) {
    pendingNavigationRequest.value = range;
    debug(
      `appStore: pendingNavigationRequest set to ${JSON.stringify(pendingNavigationRequest.value)}`
    );
  }

  function setJuliaDaemonReady(status: boolean) {
    juliaDaemonReady.value = status;
    debug(`appStore: juliaDaemonReady set to ${juliaDaemonReady.value}`);
  }

  function setBackendBusy(status: boolean) {
    backendBusy.value = status;
    debug(`appStore: backendBusy set to ${backendBusy.value}`);
  }

  async function syncBackendBusyStatus() {
    try {
      const isBusy = await invoke('get_backend_busy_status');
      if (isBusy !== backendBusy.value) {
        debug(`appStore: Syncing busy status from backend: ${isBusy}`);
        setBackendBusy(isBusy as boolean);
      }
      // Only log when there's a change to avoid cluttering logs
    } catch (error) {
      debug(`appStore: Failed to sync busy status: ${error}`);
    }
  }

  // Centralized busy state management - this is the single source of truth
  function getBackendBusyStatus(): boolean {
    return backendBusy.value;
  }

  // Force sync busy state (useful for debugging or manual correction)
  async function forceSyncBackendBusyStatus() {
    try {
      const isBusy = await invoke('get_backend_busy_status');
      debug(`appStore: Force syncing busy status from backend: ${isBusy}`);
      setBackendBusy(isBusy as boolean);
    } catch (error) {
      debug(`appStore: Failed to force sync busy status: ${error}`);
    }
  }

  // Debug function to log current busy state
  function debugBusyState() {
    debug(`appStore: Current busy state - frontend: ${backendBusy.value}`);
  }

  function setFileServerPort(port: number | null) {
    fileServerPort.value = port;
    debug(`appStore: fileServerPort set to ${fileServerPort.value}`);
  }

  function setFileServerError(error: string | null) {
    fileServerError.value = error;
    debug(`appStore: fileServerError set to ${fileServerError.value}`);
  }

  function setLspStatus(status: LspStatus) {
    lspStatus.value = status;
    debug(`appStore: LSP status set to ${status.status} - ${status.message}`);
  }

  function updateLspStatus(status: Partial<LspStatus>) {
    lspStatus.value = { ...lspStatus.value, ...status };
    debug(`appStore: LSP status updated to ${lspStatus.value.status} - ${lspStatus.value.message}`);
  }

  function setWorkspaceVariables(variables: Record<string, any>) {
    workspaceVariables.value = variables;
    debug(`appStore: Workspace variables updated, count: ${Object.keys(variables).length}`);
  }

  // Editor tab management functions
  function addOpenFile(file: OpenFile) {
    const existingIndex = openFiles.value.findIndex((f) => f.path === file.path);
    if (existingIndex === -1) {
      openFiles.value.push(file);
      debug(`appStore: Added open file ${file.path}`);
    } else {
      openFiles.value[existingIndex] = file;
      debug(`appStore: Updated open file ${file.path}`);
    }
  }

  function removeOpenFile(filePath: string) {
    const index = openFiles.value.findIndex((f) => f.path === filePath);
    if (index !== -1) {
      openFiles.value.splice(index, 1);
      debug(`appStore: Removed open file ${filePath}`);

      // If the active tab was closed, switch to the previous tab
      if (activeTab.value === filePath) {
        if (openFiles.value.length > 0) {
          const nextIndex = Math.max(0, index - 1);
          activeTab.value = openFiles.value[nextIndex].path;
        } else {
          activeTab.value = null;
        }
      }
    }
  }

  function setActiveTab(filePath: string | null) {
    activeTab.value = filePath;
    debug(`appStore: Active tab set to ${filePath}`);
  }

  function clearAllTabs() {
    openFiles.value = [];
    activeTab.value = null;
    debug(`appStore: Cleared all tabs`);
  }

  function updateOpenFile(filePath: string, updates: Partial<OpenFile>) {
    const index = openFiles.value.findIndex((f) => f.path === filePath);
    if (index !== -1) {
      openFiles.value[index] = { ...openFiles.value[index], ...updates };
      debug(`appStore: Updated open file ${filePath}`);
    }
  }

  function getActiveProjectPath(): string | null {
    return projectPath.value;
  }

  function isJuliaProjectActive(): boolean {
    return isJuliaProject.value && projectPath.value !== null;
  }

  return {
    projectPath,
    setProjectPath,
    fileToOpen,
    setFileToOpen,
    isJuliaProject,
    setIsJuliaProject,
    initialProjectLoadAttempted,
    setInitialProjectLoadAttempted,
    pendingNavigationRequest,
    setPendingNavigationRequest,
    juliaDaemonReady,
    setJuliaDaemonReady,
    backendBusy,
    setBackendBusy,
    syncBackendBusyStatus,
    getBackendBusyStatus,
    forceSyncBackendBusyStatus,
    debugBusyState,
    fileServerPort,
    setFileServerPort,
    fileServerError,
    setFileServerError,
    lspStatus,
    setLspStatus,
    updateLspStatus,
    workspaceVariables,
    setWorkspaceVariables,
    getActiveProjectPath,
    isJuliaProjectActive,
    // Editor tab state
    openFiles,
    activeTab,
    addOpenFile,
    removeOpenFile,
    setActiveTab,
    clearAllTabs,
    updateOpenFile,
  };
});
