<template>
  <div class="file-explorer" data-component="file-explorer">
    <!-- Toolbar -->
    <FileTreeToolbar
      :project-path="composableRootPath"
      :is-installing="isInstallingDependencies"
      @new-project="handleNewProject"
      @select-folder="handleSelectFolder"
      @project-config="handleProjectConfig"
      @instantiate-dependencies="handleInstantiateDependencies"
    />

    <!-- Search and Breadcrumbs temporarily removed for debugging -->

    <!-- Main Tree -->
    <div class="tree-container">
      <n-spin :show="loading">
        <FileTree
          :data="fileTree"
          :expanded-keys="expandedKeys"
          :selected-keys="selectedKeys"
          :loading="loading"
          :error="error"
          @node-select="handleNodeSelect"
          @node-expand="handleNodeExpand"
          @node-load="handleNodeLoad"
          @node-context-menu="handleContextMenu"
          @expanded-keys-change="handleExpandedKeysChange"
        />
      </n-spin>
    </div>

    <!-- Context Menu -->
    <FileTreeContextMenu
      :visible="contextMenuVisible"
      :target="contextMenuTarget"
      :position="contextMenuPosition"
      @action="handleContextAction"
      @close="contextMenuVisible = false"
    />

    <!-- Dialogs -->
    <FileTreeDialogs
      :create-dialog-visible="createDialogVisible"
      :create-dialog-type="createDialogType"
      :create-dialog-title="createDialogTitle"
      :create-form-value="createFormValue"
      :create-loading="createLoading"
      :rename-dialog-visible="renameDialogVisible"
      :rename-form-value="renameFormValue"
      :rename-loading="renameLoading"
      :delete-dialog-visible="deleteDialogVisible"
      :delete-target="contextMenuTarget"
      :delete-loading="deleteLoading"
      @create-submit="handleCreateSubmit"
      @create-cancel="createDialogVisible = false"
      @create-dialog-update="createDialogVisible = $event"
      @rename-submit="handleRenameSubmit"
      @rename-cancel="renameDialogVisible = false"
      @rename-dialog-update="renameDialogVisible = $event"
      @delete-confirm="handleDeleteConfirm"
      @delete-cancel="deleteDialogVisible = false"
      @delete-dialog-update="deleteDialogVisible = $event"
    />

    <!-- New Julia Project Dialog -->
    <NewJuliaProjectDialog
      ref="newJuliaProjectDialogRef"
      @projectRootChanged="handleProjectRootChanged"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { NSpin, useMessage } from 'naive-ui';
import { useAppStore } from '../../store/appStore';
import { useTerminalStore } from '../../store/terminalStore';
import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { basename } from '@tauri-apps/api/path';
import { debug, error } from '../../utils/logger';

// Import new components
import FileTreeToolbar from './components/FileTreeToolbar.vue';
import FileTree from './components/FileTree.vue';
import FileTreeContextMenu from './components/FileTreeContextMenu.vue';
import FileTreeDialogs from './components/FileTreeDialogs.vue';
import NewJuliaProjectDialog from '../shared/NewJuliaProjectDialog.vue';

// Import composables
import { useFileTree, useFileOperations, useFileWatching } from '../../composables/fileTree';
import { tabService } from '../../services/tabService';
import { createEmptyJuliaNotebook } from '../../utils/notebookUtils';

// Import types
import type { FileNode, FileChangeEvent } from '../../types/fileTree';

// ============================
// Props and Emits
// ============================

const emit = defineEmits<{
  'open-file': [path: string];
  'project-root-changed': [path: string];
  'open-package-settings': [projectPath: string];
}>();

// ============================
// Composables
// ============================

const message = useMessage();
const appStore = useAppStore();
// terminalStore temporarily removed for debugging

// File tree composables
const {
  fileTree,
  expandedKeys: expandedKeysSet,
  selectedKeys: selectedKeysSet,
  loading,
  error: treeError,
  rootPath: composableRootPath,
  expandNode,
  selectNode,
  refreshTree,
  loadDirectoryContents,
} = useFileTree();

// Convert Sets to Arrays for the template
const expandedKeys = computed(() => Array.from(expandedKeysSet.value));
const selectedKeys = computed(() => Array.from(selectedKeysSet.value));

const { createFile, createFolder, deleteItem, renameItem } = useFileOperations();

// Search functionality temporarily removed for debugging

const { isWatching, startWatching, stopWatching, onFileChange } = useFileWatching();

// ============================
// State
// ============================

// Loading state for instantiate dependencies
const isInstallingDependencies = ref(false);

const currentPath = ref<string | null>(null);
const rootFolderShort = ref<string | null>(null);

// Context menu state
const contextMenuVisible = ref<boolean>(false);
const contextMenuTarget = ref<FileNode | null>(null);
const contextMenuPosition = ref<{ x: number; y: number }>({ x: 0, y: 0 });

// Dialog state
const createDialogVisible = ref<boolean>(false);
const createDialogType = ref<'julia-file' | 'jupyter-notebook' | 'folder'>('julia-file');
const createDialogTitle = ref<string>('');
const createFormValue = ref<{ name: string }>({ name: '' });
const createLoading = ref<boolean>(false);

const renameDialogVisible = ref<boolean>(false);
const renameFormValue = ref<{ name: string }>({ name: '' });
const renameLoading = ref<boolean>(false);

const deleteDialogVisible = ref<boolean>(false);
const deleteLoading = ref<boolean>(false);

// New Julia Project Dialog ref
const newJuliaProjectDialogRef = ref<InstanceType<typeof NewJuliaProjectDialog> | null>(null);

// ============================
// Computed Properties
// ============================

// Combine tree error and file server error
const error = computed(() => {
  const errors: string[] = [];
  if (treeError.value) {
    errors.push(treeError.value);
  }
  if (appStore.fileServerError) {
    errors.push(appStore.fileServerError);
  }
  return errors.length > 0 ? errors.join(' | ') : null;
});

// ============================
// Event Handlers
// ============================

const handleNewProject = () => {
  if (newJuliaProjectDialogRef.value) {
    newJuliaProjectDialogRef.value.show = true;
  }
};

const handleProjectRootChanged = async (newRoot: string) => {
  try {
    await setRootPath(newRoot);
    emit('project-root-changed', newRoot);
  } catch (err) {
    error('Error changing project root:', err);
    message.error(`Failed to change project root: ${err}`);
  }
};

const handleSelectFolder = async () => {
  try {
    const result = await openDialog({ directory: true, multiple: false });
    let folderToLoad = null;

    if (result && !Array.isArray(result)) {
      folderToLoad = result;
    } else if (Array.isArray(result) && result.length > 0) {
      folderToLoad = result[0];
    }

    if (folderToLoad) {
      await setRootPath(folderToLoad);
      emit('project-root-changed', folderToLoad);
    }
  } catch (err) {
    error('Error selecting folder:', err);
    message.error(`Failed to open folder dialog: ${err}`);
  }
};

const handleProjectConfig = () => {
  if (composableRootPath.value) {
    emit('open-package-settings', composableRootPath.value);
  }
};

const handleInstantiateDependencies = async () => {
  if (!composableRootPath.value) {
    message.error('No project is currently open');
    return;
  }

  isInstallingDependencies.value = true;

  try {
    debug('Installing project dependencies with Pkg.instantiate()');
    const result = await invoke('instantiate_julia_project');
    message.success('Project dependencies installed successfully!');
    debug('Pkg.instantiate() completed:', result);
  } catch (error) {
    debug('Failed to install dependencies:', error);
    message.error(`Failed to install dependencies: ${error}`);
  } finally {
    isInstallingDependencies.value = false;
  }
};

// Search and navigation functions temporarily removed for debugging

const handleNodeSelect = (node: FileNode) => {
  selectNode(node.path);
  if (!node.is_directory) {
    emit('open-file', node.path);
  }
};

const handleNodeExpand = (node: FileNode) => {
  const nodeKey = node.key || node.path;
  expandNode(nodeKey);
};

const handleExpandedKeysChange = (keys: string[]) => {
  // Update the expandedKeys Set with the new keys
  expandedKeysSet.value.clear();
  keys.forEach((key) => expandedKeysSet.value.add(key));
};

const handleNodeLoad = async (node: FileNode, resolve: () => void) => {
  try {
    if (node.is_directory) {
      await loadDirectoryContents(node.path);
    }
  } finally {
    // Always call resolve to complete the promise
    resolve();
  }
};

const handleContextMenu = (node: FileNode | null, event: MouseEvent) => {
  contextMenuTarget.value = node;
  contextMenuPosition.value = { x: event.clientX, y: event.clientY };
  contextMenuVisible.value = true;
};

const handleContextAction = async (action: string) => {
  contextMenuVisible.value = false;

  switch (action) {
    case 'rename':
      if (contextMenuTarget.value) {
        renameFormValue.value.name = contextMenuTarget.value.name;
        renameDialogVisible.value = true;
      }
      break;
    case 'delete':
      if (contextMenuTarget.value) {
        deleteDialogVisible.value = true;
      }
      break;
    case 'copy-path':
      if (contextMenuTarget.value) {
        await navigator.clipboard.writeText(contextMenuTarget.value.path);
        message.success('Path copied to clipboard');
      }
      break;
    case 'new-julia-file':
      createDialogType.value = 'julia-file';
      createDialogTitle.value = 'Create New Julia File';
      createDialogVisible.value = true;
      break;
    case 'new-jupyter-notebook':
      createDialogType.value = 'jupyter-notebook';
      createDialogTitle.value = 'Create New Jupyter Notebook';
      createDialogVisible.value = true;
      break;
    case 'new-folder':
      createDialogType.value = 'folder';
      createDialogTitle.value = 'Create New Folder';
      createDialogVisible.value = true;
      break;
  }
};

const handleCreateSubmit = async () => {
  if (!createFormValue.value.name.trim()) {
    message.error('Please enter a name');
    return;
  }

  createLoading.value = true;

  try {
    let targetPath: string;
    if (contextMenuTarget.value && contextMenuTarget.value.is_directory) {
      targetPath = contextMenuTarget.value.path;
    } else if (contextMenuTarget.value && !contextMenuTarget.value.is_directory) {
      const filePath = contextMenuTarget.value.path;
      const lastSlashIndex = Math.max(filePath.lastIndexOf('/'), filePath.lastIndexOf('\\'));
      targetPath = lastSlashIndex !== -1 ? filePath.substring(0, lastSlashIndex) : filePath;
    } else {
      targetPath = composableRootPath.value || '';
    }

    const separator = targetPath.includes('\\') ? '\\' : '/';
    let fileName = createFormValue.value.name.trim();

    // Handle file extensions
    if (createDialogType.value === 'julia-file') {
      // Ensure .jl extension
      if (!fileName.toLowerCase().endsWith('.jl')) {
        // Remove any existing extension and add .jl
        const lastDotIndex = fileName.lastIndexOf('.');
        if (lastDotIndex > 0) {
          fileName = fileName.substring(0, lastDotIndex) + '.jl';
        } else {
          fileName = fileName + '.jl';
        }
      }
    } else if (createDialogType.value === 'jupyter-notebook') {
      // Ensure .ipynb extension
      if (!fileName.toLowerCase().endsWith('.ipynb')) {
        // Remove any existing extension and add .ipynb
        const lastDotIndex = fileName.lastIndexOf('.');
        if (lastDotIndex > 0) {
          fileName = fileName.substring(0, lastDotIndex) + '.ipynb';
        } else {
          fileName = fileName + '.ipynb';
        }
      }
    }

    const newPath = targetPath + separator + fileName;

    if (createDialogType.value === 'folder') {
      await createFolder(newPath);
      message.success('Folder created successfully');
    } else if (createDialogType.value === 'jupyter-notebook') {
      // Create notebook with proper structure
      const notebookContent = createEmptyJuliaNotebook();
      const result = await createFile(newPath, notebookContent);
      if (!result.success) {
        throw new Error(result.error || 'Failed to create notebook');
      }
      message.success('Jupyter notebook created successfully');
    } else {
      // Julia file
      await createFile(newPath);
      message.success('Julia file created successfully');
    }

    createDialogVisible.value = false;
    await refreshTree();
  } catch (err) {
    message.error(err.toString());
  } finally {
    createLoading.value = false;
  }
};

const handleRenameSubmit = async () => {
  if (!contextMenuTarget.value || !renameFormValue.value.name.trim()) {
    message.error('Please enter a new name');
    return;
  }

  renameLoading.value = true;

  try {
    const oldPath = contextMenuTarget.value.path;
    const lastSlashIndex = Math.max(oldPath.lastIndexOf('/'), oldPath.lastIndexOf('\\'));
    const parentPath = lastSlashIndex !== -1 ? oldPath.substring(0, lastSlashIndex) : oldPath;
    const separator = oldPath.includes('\\') ? '\\' : '/';
    const newPath = parentPath + separator + renameFormValue.value.name;

    await renameItem(oldPath, newPath);
    message.success('Item renamed successfully');
    renameDialogVisible.value = false;
    await refreshTree();
  } catch (err) {
    message.error(err.toString());
  } finally {
    renameLoading.value = false;
  }
};

const handleDeleteConfirm = async () => {
  if (!contextMenuTarget.value) return;

  deleteLoading.value = true;

  try {
    const deletedPath = contextMenuTarget.value.path;
    const isFile = !contextMenuTarget.value.is_directory;

    await deleteItem(deletedPath);
    message.success('Item deleted successfully');
    deleteDialogVisible.value = false;
    await refreshTree();

    // If a file was deleted and it's open as a tab, close the tab
    if (isFile) {
      const tab = tabService.getTabByPath(deletedPath);
      if (tab) {
        // Dispatch event to close the tab in EditorView
        window.dispatchEvent(
          new CustomEvent('file-deleted', { detail: { filePath: deletedPath } })
        );
      }
    }
  } catch (err) {
    message.error(err.toString());
  } finally {
    deleteLoading.value = false;
  }
};

// ============================
// Helper Functions
// ============================

const setRootPath = async (path: string) => {
  composableRootPath.value = path;
  currentPath.value = path;

  try {
    await updateShortPath(path);
    await invoke('set_last_opened_folder', { path });
    await refreshTree();
    await startWatching(path);
  } catch (err) {
    error('Failed to set root path:', err);
  }
};

const updateShortPath = async (folderPath: string) => {
  if (!folderPath) {
    rootFolderShort.value = null;
    return;
  }

  try {
    rootFolderShort.value = await basename(folderPath);
  } catch (e) {
    debug('Failed to get basename, using fallback:', e);
    rootFolderShort.value = folderPath.split(/\\|\//).pop() || folderPath;
  }
};

// ============================
// Lifecycle
// ============================

onMounted(async () => {
  // Set up file change handling
  onFileChange((event: FileChangeEvent) => {
    refreshTree();

    // If a file was deleted externally, close its tab if open
    if (event.change_type === 'deleted') {
      const tab = tabService.getTabByPath(event.path);
      if (tab) {
        // Dispatch event to close the tab in EditorView
        window.dispatchEvent(new CustomEvent('file-deleted', { detail: { filePath: event.path } }));
      }
    }
  });

  // Initialize if project path is already set
  if (appStore.projectPath) {
    await setRootPath(appStore.projectPath);
  }
});

onUnmounted(async () => {
  if (isWatching.value) {
    await stopWatching();
  }
});

// ============================
// Watchers
// ============================

watch(
  () => appStore.projectPath,
  async (newPath) => {
    if (newPath && newPath !== composableRootPath.value) {
      await setRootPath(newPath);
    } else if (!newPath && composableRootPath.value) {
      composableRootPath.value = null;
      currentPath.value = null;
      await stopWatching();
    }
  }
);
</script>

<style scoped>
.file-explorer {
  height: 100%;
  display: flex;
  flex-direction: column;
  background-color: #282828;
  min-height: 0;
}

.tree-container {
  flex-grow: 1;
  overflow: hidden;
  padding: 2px;
  min-height: 0;
  display: flex;
  flex-direction: column;
}
</style>
