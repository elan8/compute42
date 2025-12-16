<template>
  <!-- Use a div instead of NLayout -->
  <div
    data-component="file-explorer"
    style="
      height: 100%;
      display: flex;
      flex-direction: column;
      background-color: #282828;
      min-height: 0;
    "
  >
    <!-- Top Menu Bar -->
    <n-space
      justify="space-between"
      align="center"
      style="padding: 1px 8px; border-bottom: 1px solid #444; flex-shrink: 0"
    >
      <n-text
        style="
          color: #ccc;
          overflow: hidden;
          text-overflow: ellipsis;
          white-space: nowrap;
          flex-grow: 1;
          margin-right: 10px;
          padding: 1px 0;
        "
        :title="rootFolder"
      >
        {{ rootFolderShort || 'No Folder Selected' }}
      </n-text>
      <n-space>
        <n-tooltip trigger="hover">
          <template #trigger>
            <n-button size="tiny" circle @click="selectFolder">
              <n-icon><FolderOpenOutline /></n-icon>
            </n-button>
          </template>
          Select Root Folder
        </n-tooltip>
        <n-tooltip trigger="hover">
          <template #trigger>
            <n-button size="tiny" circle @click="showNewJuliaProjectDialog">
              <n-icon><AddOutline /></n-icon>
            </n-button>
          </template>
          New Julia Project
        </n-tooltip>
      </n-space>
    </n-space>

    <!-- File Tree Area - Use a div instead of NLayoutContent -->
    <div
      style="
        flex-grow: 1;
        overflow: hidden;
        padding: 2px;
        min-height: 0;
        display: flex;
        flex-direction: column;
      "
      @contextmenu="handleContainerContextMenu"
    >
      <!-- Non-Julia Project Warning -->
      <NonJuliaProjectWarning />
      <n-alert
        v-if="error_state"
        type="error"
        title="Error"
        closable
        @close="error_state = null"
        style="margin-bottom: 2px"
      >
        {{ error_state }}
      </n-alert>
      <!-- Tree Container with proper scrolling -->
      <div style="flex-grow: 1; overflow: auto; min-height: 0">
        <n-spin :show="loading">
          <n-tree
            v-if="fileTree.length > 0"
            block-line
            :data="fileTree"
            key-field="path"
            label-field="name"
            children-field="children"
            selectable
            :expanded-keys="expandedKeys"
            @update:expanded-keys="handleExpandedKeysChange"
            @update:selected-keys="handleNodeSelect"
          >
            <template #prefix="{ option: node }">
              <n-icon
                :style="{
                  color: node.is_directory ? '#68a0d8' : '#a9a9a9',
                  marginRight: '4px',
                  verticalAlign: 'middle',
                }"
              >
                <component :is="getIconDefinition(node)" />
              </n-icon>
            </template>
          </n-tree>
          <n-empty
            v-else-if="!loading && !error"
            description="Select a folder or folder is empty"
            style="margin-top: 10px"
          >
          </n-empty>
        </n-spin>
      </div>
    </div>

    <!-- Context Menu -->
    <div
      v-if="contextMenuVisible"
      class="vscode-context-menu"
      :style="contextMenuStyle"
      @click.stop
      @keydown.escape="closeContextMenu"
      tabindex="0"
    >
      <div class="context-menu-item" @click="handleContextMenuSelect('new-file')">
        <div class="context-menu-icon">
          <n-icon><DocumentOutline /></n-icon>
        </div>
        <span class="context-menu-label">New File</span>
      </div>

      <div class="context-menu-item" @click="handleContextMenuSelect('new-folder')">
        <div class="context-menu-icon">
          <n-icon><FolderOutline /></n-icon>
        </div>
        <span class="context-menu-label">New Folder</span>
      </div>

      <template v-if="contextMenuTarget">
        <div class="context-menu-separator"></div>

        <div class="context-menu-item" @click="handleContextMenuSelect('rename')">
          <div class="context-menu-icon">
            <n-icon><CreateOutline /></n-icon>
          </div>
          <span class="context-menu-label">Rename</span>
        </div>

        <div
          class="context-menu-item context-menu-item-danger"
          @click="handleContextMenuSelect('delete')"
        >
          <div class="context-menu-icon">
            <n-icon><TrashOutline /></n-icon>
          </div>
          <span class="context-menu-label">Delete</span>
        </div>

        <template v-if="!contextMenuTarget.is_directory">
          <div class="context-menu-separator"></div>

          <div class="context-menu-item" @click="handleContextMenuSelect('copy-path')">
            <div class="context-menu-icon">
              <n-icon><CopyOutline /></n-icon>
            </div>
            <span class="context-menu-label">Copy Path</span>
          </div>
        </template>
      </template>
    </div>

    <!-- Create File/Folder Dialog -->
    <n-modal
      v-model:show="createDialogVisible"
      preset="card"
      style="width: 400px"
      :title="createDialogTitle"
    >
      <n-form
        ref="createFormRef"
        :model="createFormValue"
        :rules="createFormRules"
        label-placement="left"
        label-width="auto"
        require-mark-placement="right-hanging"
      >
        <n-form-item label="Name" path="name">
          <n-input v-model:value="createFormValue.name" :placeholder="createFormPlaceholder" />
        </n-form-item>
      </n-form>

      <template #footer>
        <n-space justify="end">
          <n-button @click="createDialogVisible = false">Cancel</n-button>
          <n-button type="primary" @click="handleCreateSubmit" :loading="createLoading">
            Create
          </n-button>
        </n-space>
      </template>
    </n-modal>

    <!-- Rename Dialog -->
    <n-modal
      v-model:show="renameDialogVisible"
      preset="card"
      style="width: 400px"
      title="Rename Item"
    >
      <n-form
        ref="renameFormRef"
        :model="renameFormValue"
        :rules="renameFormRules"
        label-placement="left"
        label-width="auto"
        require-mark-placement="right-hanging"
      >
        <n-form-item label="New Name" path="name">
          <n-input v-model:value="renameFormValue.name" placeholder="Enter new name" />
        </n-form-item>
      </n-form>

      <template #footer>
        <n-space justify="end">
          <n-button @click="renameDialogVisible = false">Cancel</n-button>
          <n-button type="primary" @click="handleRenameSubmit" :loading="renameLoading">
            Rename
          </n-button>
        </n-space>
      </template>
    </n-modal>

    <!-- Delete Confirmation Dialog -->
    <n-modal
      v-model:show="deleteDialogVisible"
      preset="card"
      style="width: 400px"
      title="Confirm Delete"
    >
      <div style="margin-bottom: 16px">
        <p>Are you sure you want to delete this item?</p>
        <p style="color: #999; font-size: 12px; margin-top: 8px">
          <strong>Name:</strong> {{ contextMenuTarget?.name }}
        </p>
        <p style="color: #999; font-size: 12px; margin-top: 4px">
          <strong>Type:</strong> {{ contextMenuTarget?.is_directory ? 'Folder' : 'File' }}
        </p>
        <p style="color: #999; font-size: 12px; margin-top: 4px">
          <strong>Path:</strong> {{ contextMenuTarget?.path }}
        </p>
        <p style="color: #ff6b6b; font-size: 12px; margin-top: 8px; font-weight: bold">
          ⚠️ This action cannot be undone!
        </p>
      </div>

      <template #footer>
        <n-space justify="end">
          <n-button @click="deleteDialogVisible = false">Cancel</n-button>
          <n-button type="error" @click="confirmDelete" :loading="deleteLoading"> Delete </n-button>
        </n-space>
      </template>
    </n-modal>
  </div>

  <!-- Add the dialog component at the end of the template -->
  <NewJuliaProjectDialog
    ref="newJuliaProjectDialogRef"
    @projectRootChanged="handleProjectRootChanged"
  />

  <!-- Project Activation Modal - Removed, now handled by StartupModal -->
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from 'vue';
// Removed NLayout, NLayoutContent from imports
import {
  NSpace,
  NButton,
  NTooltip,
  NText,
  NTree,
  NSpin,
  NEmpty,
  NAlert,
  NModal,
  NForm,
  NFormItem,
  NInput,
  NIcon,
  useMessage,
} from 'naive-ui';
import { useAppStore } from '../../store/appStore'; // Import the store
import { useTerminalStore } from '../../store/terminalStore';
import { invoke } from '@tauri-apps/api/core';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import { basename } from '@tauri-apps/api/path';
import { listen } from '@tauri-apps/api/event';
import { debug, info, error, warn } from '../../utils/logger';
import { createIoniconsFileIconComponent } from '../../utils/ioniconsFileIconUtils';
import NewJuliaProjectDialog from '../shared/NewJuliaProjectDialog.vue';
import NonJuliaProjectWarning from '../shared/NonJuliaProjectWarning.vue';
// ProjectActivationModal import removed - now handled by StartupModal
import {
  DocumentOutline,
  FolderOutline,
  CreateOutline,
  TrashOutline,
  CopyOutline,
  FolderOpenOutline,
  AddOutline,
  CodeOutline,
  CodeSlash,
  DocumentText,
  Settings,
  GitBranch,
} from '@vicons/ionicons5';
// Material icons replaced with ionicons5 equivalents

// defineEmits is a compiler macro, no import needed
const emit = defineEmits(['open-file', 'project-root-changed']);

const message = useMessage();

const rootFolder = ref(null);
const fileTree = ref([]);
const loading = ref(false);
const error_state = ref(null);
const expandedKeys = ref([]);

const rootFolderShort = ref(null);
const appStore = useAppStore(); // Initialize the store
const terminalStore = useTerminalStore(); // Initialize terminal store

const newJuliaProjectDialogRef = ref(null);

// File watching state
const fileWatcherActive = ref(false);
const fileWatcherUnlisten = ref(null);

// Context menu state
const contextMenuVisible = ref(false);
const contextMenuX = ref(0);
const contextMenuY = ref(0);
const contextMenuTarget = ref(null);

// Create dialog state
const createDialogVisible = ref(false);
const createDialogTitle = ref('');
const createDialogType = ref(''); // 'file' or 'folder'
const createFormRef = ref(null);
const createFormValue = ref({ name: '' });
const createFormRules = {
  name: {
    required: true,
    message: 'Please enter a name',
    trigger: 'blur',
  },
};
const createFormPlaceholder = ref('');
const createLoading = ref(false);

// Rename dialog state
const renameDialogVisible = ref(false);
const renameFormRef = ref(null);
const renameFormValue = ref({ name: '' });
const renameFormRules = {
  name: {
    required: true,
    message: 'Please enter a name',
    trigger: 'blur',
  },
};
const renameLoading = ref(false);

// Delete dialog state
const deleteDialogVisible = ref(false);
const deleteLoading = ref(false);

// Project activation modal state
// showProjectActivationModal removed - now handled by StartupModal

// Context menu positioning
const contextMenuStyle = computed(() => {
  const menuWidth = 180;
  const menuHeight = 200; // Approximate height
  const windowWidth = window.innerWidth;
  const windowHeight = window.innerHeight;

  let left = contextMenuX.value;
  let top = contextMenuY.value;

  // Adjust horizontal position if menu would go off-screen
  if (left + menuWidth > windowWidth) {
    left = windowWidth - menuWidth - 10;
  }

  // Adjust vertical position if menu would go off-screen
  if (top + menuHeight > windowHeight) {
    top = windowHeight - menuHeight - 10;
  }

  console.log('Context menu style:', { left, top, visible: contextMenuVisible.value });

  return {
    left: left + 'px',
    top: top + 'px',
  };
});

// Function to update the short path (call when rootFolder changes)
async function updateShortPath(folderPath) {
  if (!folderPath) {
    rootFolderShort.value = null;
    return;
  }
  try {
    rootFolderShort.value = await basename(folderPath);
  } catch (e) {
    debug('Failed to get basename, using fallback:', e);
    // Fallback to simple split if basename fails or isn't available sync
    rootFolderShort.value = folderPath.split(/\\|\//).pop() || folderPath;
  }
}

// --- File Tree Data Fetching ---
async function fetchFileTree(folderPath) {
  if (!folderPath) return;
  loading.value = true;
  error_state.value = null;

  // Store current expanded state before refreshing
  const currentExpandedKeys = [...expandedKeys.value];

  fileTree.value = [];
  try {
    debug(`Invoking get_file_tree with: ${folderPath}`);
    const startTime = Date.now();

    const tree = await invoke('get_file_tree', { rootPath: folderPath });
    const endTime = Date.now();
    debug(`Received tree structure in ${endTime - startTime}ms:`, tree);

    // Check if tree exists and has children before mapping
    // The backend returns the root directory node directly, so we need to check if it has children
    if (tree && tree.is_directory && tree.children) {
      debug(`Processing ${tree.children.length} children for directory: ${folderPath}`);
      fileTree.value = mapNodesForTree(tree.children);
      debug(`Mapped tree has ${fileTree.value.length} top-level items`);
    } else if (tree && tree.is_directory) {
      // Root directory exists but has no children (empty directory)
      debug('Root directory exists but has no children (empty directory):', folderPath);
      fileTree.value = [];
    } else {
      // No tree or invalid structure
      debug('No tree or invalid structure received:', { tree, folderPath });
      fileTree.value = [];
    }

    if (!fileTree.value.length) {
      debug('Final file tree is empty for:', folderPath);
    } else {
      debug(`Final file tree has ${fileTree.value.length} items for:`, folderPath);

      // Note: No need to check for size limits since we're using lazy loading
    }

    // Note: Project activation is now handled at the MainLayout level
    // This component only handles UI state
  } catch (fetchError) {
    error('Failed to fetch file tree:', fetchError);
    error_state.value = `Failed to load folder: ${fetchError?.message || fetchError}`;
  } finally {
    loading.value = false;

    // Restore expanded state after tree refresh (outside try-catch to ensure it runs)
    try {
      expandedKeys.value = currentExpandedKeys;
    } catch (expandError) {
      debug('Failed to restore expanded state:', expandError);
    }
  }
}

function mapNodesForTree(nodes) {
  if (!nodes) return [];
  return nodes.map((node) => ({
    ...node,
    isLeaf: !node.is_directory,
    prefix: createIoniconsFileIconComponent(node.name, node.is_directory),
    children: node.is_directory && node.children ? mapNodesForTree(node.children) : undefined,
  }));
}

// --- Context Menu Handlers ---
async function handleContextMenuSelect(key) {
  debug(`Context menu item selected: ${key}`);
  debug(`Context menu target: ${contextMenuTarget.value ? contextMenuTarget.value.name : 'null'}`);

  // Store the target before closing the menu
  const target = contextMenuTarget.value;

  closeContextMenu();

  switch (key) {
    case 'new-file':
      showCreateDialog('file');
      break;
    case 'new-folder':
      showCreateDialog('folder');
      break;
    case 'rename':
      if (target) {
        contextMenuTarget.value = target;
        showRenameDialog();
      }
      break;
    case 'delete':
      debug('Delete action triggered');
      if (target) {
        contextMenuTarget.value = target;
        await deleteItem();
      }
      break;
    case 'copy-path':
      if (target) {
        contextMenuTarget.value = target;
        await copyPathToClipboard();
      }
      break;
  }
}

// Handle clicking outside the context menu
function handleClickOutside(event) {
  const contextMenu = event.target.closest('.vscode-context-menu');
  const fileExplorer = event.target.closest('[data-component="file-explorer"]');

  // Only close the context menu if clicking outside both the context menu and the FileExplorer component
  if (!contextMenu && !fileExplorer) {
    closeContextMenu();
  }
}

// Close context menu
function closeContextMenu() {
  debug('Closing context menu');
  contextMenuVisible.value = false;
  contextMenuTarget.value = null;
}

function showCreateDialog(type) {
  createDialogType.value = type;
  createDialogTitle.value = type === 'file' ? 'Create New File' : 'Create New Folder';
  createFormPlaceholder.value = type === 'file' ? 'filename.ext' : 'folder name';
  createFormValue.value.name = '';
  createDialogVisible.value = true;

  nextTick(() => {
    createFormRef.value?.focus();
  });
}

async function handleCreateSubmit() {
  try {
    await createFormRef.value?.validate();
    createLoading.value = true;

    // Determine the target path for creation
    let targetPath;
    if (contextMenuTarget.value && contextMenuTarget.value.is_directory) {
      // If right-clicking on a folder, create inside that folder
      targetPath = contextMenuTarget.value.path;
    } else if (contextMenuTarget.value && !contextMenuTarget.value.is_directory) {
      // If right-clicking on a file, create in the same directory as the file
      const filePath = contextMenuTarget.value.path;
      const lastSlashIndex = Math.max(filePath.lastIndexOf('/'), filePath.lastIndexOf('\\'));
      targetPath = lastSlashIndex !== -1 ? filePath.substring(0, lastSlashIndex) : filePath;
    } else {
      // If right-clicking on empty space, create in root
      targetPath = rootFolder.value;
    }

    debug('Creating item in target path:', targetPath);

    // Use the same path separator as the target path
    const separator = targetPath.includes('\\') ? '\\' : '/';
    const newPath = targetPath + separator + createFormValue.value.name;
    debug('Full new path:', newPath);

    if (createDialogType.value === 'file') {
      await invoke('create_file_item', { path: newPath });
    } else {
      await invoke('create_folder_item', { path: newPath });
    }

    message.success(
      `${createDialogType.value === 'file' ? 'File' : 'Folder'} created successfully`
    );
    createDialogVisible.value = false;

    // Refresh the file tree
    await fetchFileTree(rootFolder.value);
  } catch (err) {
    message.error(err.toString());
  } finally {
    createLoading.value = false;
  }
}

function showRenameDialog() {
  if (!contextMenuTarget.value) return;

  renameFormValue.value.name = contextMenuTarget.value.name;
  renameDialogVisible.value = true;

  nextTick(() => {
    renameFormRef.value?.focus();
  });
}

async function handleRenameSubmit() {
  try {
    await renameFormRef.value?.validate();
    renameLoading.value = true;

    const oldPath = contextMenuTarget.value.path;

    // Handle both Windows and Unix path separators
    const lastSlashIndex = Math.max(oldPath.lastIndexOf('/'), oldPath.lastIndexOf('\\'));
    const parentPath = lastSlashIndex !== -1 ? oldPath.substring(0, lastSlashIndex) : oldPath;

    // Use the same path separator as the original path
    const separator = oldPath.includes('\\') ? '\\' : '/';
    const newPath = parentPath + separator + renameFormValue.value.name;

    console.log('Rename operation:', { oldPath, parentPath, newPath, separator });

    await invoke('rename_item', { oldPath, newPath });

    message.success('Item renamed successfully');
    renameDialogVisible.value = false;

    // Refresh the file tree
    await fetchFileTree(rootFolder.value);
  } catch (err) {
    console.error('Rename error:', err);
    message.error(err.toString());
  } finally {
    renameLoading.value = false;
  }
}

async function deleteItem() {
  debug(
    `deleteItem called, contextMenuTarget: ${contextMenuTarget.value ? contextMenuTarget.value.name : 'null'}`
  );

  if (!contextMenuTarget.value) {
    error('No context menu target for delete operation');
    message.error('No item selected for deletion');
    return;
  }

  debug(
    `Showing delete confirmation dialog for: ${contextMenuTarget.value.name} (${contextMenuTarget.value.path})`
  );

  // Show confirmation dialog instead of deleting immediately
  deleteDialogVisible.value = true;
  debug(`deleteDialogVisible set to: ${deleteDialogVisible.value}`);
}

async function confirmDelete() {
  if (!contextMenuTarget.value) return;

  try {
    deleteLoading.value = true;

    debug(`Attempting to delete item: ${contextMenuTarget.value.path}`);

    await invoke('delete_item', { path: contextMenuTarget.value.path });

    debug('Delete operation completed successfully');
    message.success('Item deleted successfully');
    deleteDialogVisible.value = false;

    // Refresh the file tree
    await fetchFileTree(rootFolder.value);
  } catch (err) {
    error(`Delete operation failed: ${err}`);
    message.error(`Failed to delete item: ${err.toString()}`);
  } finally {
    deleteLoading.value = false;
  }
}

async function copyPathToClipboard() {
  if (!contextMenuTarget.value) return;

  try {
    await navigator.clipboard.writeText(contextMenuTarget.value.path);
    message.success('Path copied to clipboard');
  } catch (err) {
    message.error('Failed to copy path to clipboard');
  }
}

// --- Top Menu Actions ---
async function selectFolder() {
  debug('selectFolder function called');
  error_state.value = null;
  appStore.setInitialProjectLoadAttempted(false); // Reset for new selection process
  let previousRoot = rootFolder.value; // Store previous root
  try {
    const result = await openDialog({ directory: true, multiple: false });
    debug('Dialog result:', result);

    let folderToLoad = null;
    if (result && !Array.isArray(result)) {
      folderToLoad = result;
    } else if (Array.isArray(result) && result.length > 0) {
      folderToLoad = result[0];
    }

    if (folderToLoad) {
      debug('Folder selected:', folderToLoad);

      // Save the selected folder - this will trigger the selected-directory event
      try {
        await invoke('set_last_opened_folder', { path: folderToLoad });
        debug('Successfully saved last opened folder:', folderToLoad);
      } catch (saveError) {
        error('Failed to save last opened folder:', saveError);
        error_state.value = `Failed to save selection: ${saveError?.message || saveError}`;
      }

      // The selected-directory event will update the app store
      // We just need to update the local UI state
      rootFolder.value = folderToLoad;
      await updateShortPath(folderToLoad);
      debug('Fetching file tree for:', folderToLoad);
      await fetchFileTree(folderToLoad);
      debug('File tree fetch initiated');

      // Start file watcher for the new folder
      await startFileWatcher(folderToLoad);

      if (rootFolder.value !== previousRoot) {
        emit('project-root-changed', rootFolder.value);
      }
    } else {
      debug('No folder selected or dialog cancelled.');
      // Only set to null if there's no current project path to avoid unnecessary changes
      if (!appStore.projectPath) {
        appStore.setProjectPath(null);
        appStore.setIsJuliaProject(false);
      }
    }
  } catch (dialogError) {
    error('Error selecting folder:', dialogError);
    error_state.value = `Failed to open folder dialog: ${dialogError?.message || dialogError}`;
    // Only set to null if there's no current project path to avoid unnecessary changes
    if (!appStore.projectPath) {
      appStore.setProjectPath(null); // Ensure path is cleared on dialog error too
      appStore.setIsJuliaProject(false);
    }
  } finally {
    appStore.setInitialProjectLoadAttempted(true); // New selection attempt complete
  }
}

// --- Tree Interaction ---
function handleNodeSelect(keys, nodes) {
  if (keys.length > 0 && nodes.length > 0) {
    const selectedNode = nodes[0];
    if (!selectedNode.is_directory) {
      emit('open-file', selectedNode.path);
      error_state.value = null;
    }
  }
}

async function handleExpandedKeysChange(keys) {
  const oldKeys = expandedKeys.value;
  expandedKeys.value = keys;

  // Find newly expanded keys
  const newlyExpanded = keys.filter((key) => !oldKeys.includes(key));

  // Load contents for newly expanded directories
  for (const key of newlyExpanded) {
    await loadDirectoryContents(key);
  }
}

// Function to load directory contents when a folder is expanded
async function loadDirectoryContents(directoryPath) {
  try {
    debug(`Loading contents for directory: ${directoryPath}`);

    // Check if this directory has a placeholder child
    const findNodeByPath = (nodes, path) => {
      for (const node of nodes) {
        if (node.path === path) {
          return node;
        }
        if (node.children && node.children.length > 0) {
          const found = findNodeByPath(node.children, path);
          if (found) return found;
        }
      }
      return null;
    };

    const targetNode = findNodeByPath(fileTree.value, directoryPath);
    if (!targetNode || !targetNode.is_directory) {
      debug(`Directory node not found or not a directory: ${directoryPath}`);
      return;
    }

    // Check if it has a placeholder child
    const hasPlaceholder =
      targetNode.children &&
      targetNode.children.some((child) => child.name.includes('... (click to load contents)'));

    if (!hasPlaceholder) {
      debug(`Directory ${directoryPath} already has contents loaded`);
      return;
    }

    // Load the actual contents
    const contents = await invoke('load_directory_contents', { path: directoryPath });
    debug(`Loaded ${contents.length} items for directory: ${directoryPath}`);

    // Replace the placeholder with actual contents
    if (targetNode.children) {
      targetNode.children = mapNodesForTree(contents);
    }
  } catch (error) {
    error(`Failed to load directory contents for ${directoryPath}:`, error);
    message.error(`Failed to load directory contents: ${error}`);
  }
}

// --- Icon Definition Logic (for Vicons) ---
const getIconDefinition = (node) => {
  if (node.is_directory) {
    return FolderOutline;
  }
  const extension = node.name.split('.').pop()?.toLowerCase();
  switch (extension) {
    case 'js':
    case 'jsx':
      return CodeSlash; // JavaScript - using code slash icon
    case 'py':
      return CodeSlash; // Python - using code slash icon
    case 'html':
      return CodeSlash; // HTML - using code slash icon
    case 'css':
      return CodeSlash; // CSS - using code slash icon
    case 'rs':
      return CodeSlash; // Rust - using code slash icon
    case 'vue':
      return CodeSlash; // Vue - using code slash icon
    case 'md':
      return DocumentText; // Markdown - using document text icon
    case 'json':
      return CodeOutline;
    case 'toml':
      return Settings; // TOML - using settings icon
    case 'gitignore':
      return GitBranch; // Git files - using git branch icon
    // Add more cases here
    default:
      return DocumentOutline;
  }
};

// Removed renderPrefix function

// --- File Watching Functions ---
async function startFileWatcher(folderPath) {
  if (!folderPath || fileWatcherActive.value) return;

  try {
    debug('Starting file watcher for:', folderPath);
    await invoke('start_file_watcher', { path: folderPath });
    fileWatcherActive.value = true;

    // Listen for file system changes
    fileWatcherUnlisten.value = await listen('file:changed', (event) => {
      debug('File system change detected:', event);
      // Debounce the refresh to avoid too many updates
      setTimeout(() => {
        if (rootFolder.value) {
          fetchFileTree(rootFolder.value);
        }
      }, 500);
    });

    debug('File watcher started successfully');
  } catch (err) {
    error('Failed to start file watcher:', err);
  }
}

async function stopFileWatcher() {
  if (!fileWatcherActive.value) return;

  try {
    debug('Stopping file watcher');
    await invoke('stop_file_watcher');
    fileWatcherActive.value = false;

    if (fileWatcherUnlisten.value) {
      fileWatcherUnlisten.value();
      fileWatcherUnlisten.value = null;
    }

    debug('File watcher stopped successfully');
  } catch (err) {
    error('Failed to stop file watcher:', err);
  }
}

// --- Lifecycle ---
onMounted(async () => {
  debug('FileExplorer mounted, current project path in store:', appStore.projectPath);

  // Set up project activation event listeners
  try {
    // Project activation events now handled by StartupModal
    // await listen('project-activation-started', (event) => {
    //   debug('Project activation started:', event.payload);
    //   showProjectActivationModal.value = true;
    // });
    // await listen('project-activation-complete', (event) => {
    //   debug('Project activation completed:', event.payload);
    //   showProjectActivationModal.value = false;
    // });
  } catch (err) {
    error('Failed to set up project activation event listeners:', err);
  }

  // Initialize file tree if project path is already set in store
  if (appStore.projectPath) {
    debug('FileExplorer: Project path already set in store, initializing:', appStore.projectPath);
    await initializeFileExplorer(appStore.projectPath);
  } else {
    debug('FileExplorer: No project path in store, waiting for selected-directory event');
  }

  // Add event listener for creating new Julia project from warning component
  const createNewProjectHandler = () => {
    debug('FileExplorer: Received create-new-julia-project event');
    showNewJuliaProjectDialog();
  };
  window.addEventListener('create-new-julia-project', createNewProjectHandler);

  // Store the handler for cleanup
  window.createNewProjectHandler = createNewProjectHandler;

  // Add click outside listener for context menu
  document.addEventListener('click', handleClickOutside);
  document.addEventListener('contextmenu', handleClickOutside);
});

// Cleanup on unmount
onUnmounted(async () => {
  debug('FileExplorer unmounting, stopping file watcher');
  await stopFileWatcher();

  // Remove event listener
  if (window.createNewProjectHandler) {
    window.removeEventListener('create-new-julia-project', window.createNewProjectHandler);
    delete window.createNewProjectHandler;
  }

  // Remove context menu event listeners
  document.removeEventListener('click', handleClickOutside);
  document.removeEventListener('contextmenu', handleClickOutside);
});

// Watch for project path changes in the store
watch(
  () => appStore.projectPath,
  async (newPath, oldPath) => {
    debug('FileExplorer: Project path changed in store:', {
      oldPath,
      newPath,
      currentRoot: rootFolder.value,
    });

    // Only reinitialize if the path actually changed and is different from current root
    // Also check if we're not in the middle of a backend busy state transition
    if (newPath && newPath !== rootFolder.value && !appStore.backendBusy) {
      debug('FileExplorer: Initializing file explorer for new path:', newPath);
      await initializeFileExplorer(newPath);
    } else if (!newPath && rootFolder.value) {
      debug('FileExplorer: Clearing file explorer');
      // Clear the file explorer when project is cleared
      rootFolder.value = null;
      fileTree.value = [];
      await stopFileWatcher();
    } else {
      debug('FileExplorer: No action needed for path change:', {
        newPath,
        oldPath,
        rootFolder: rootFolder.value,
        backendBusy: appStore.backendBusy,
      });
    }
  },
  { immediate: false }
); // Changed from true to false to prevent immediate execution

// Helper function to initialize file explorer for a project (UI only)
async function initializeFileExplorer(projectPath) {
  debug('FileExplorer: Starting initialization for project path:', projectPath);
  try {
    loading.value = true;
    error_state.value = null;

    rootFolder.value = projectPath;
    debug('FileExplorer: Set root folder to:', projectPath);

    await updateShortPath(projectPath);
    debug('FileExplorer: Updated short path');

    await fetchFileTree(projectPath);
    debug('FileExplorer: Fetched file tree, tree length:', fileTree.value.length);

    await startFileWatcher(projectPath);
    debug('FileExplorer: Started file watcher');

    // Note: Project activation is now handled at the MainLayout level
    // This component only handles UI state
  } catch (error) {
    error('FileExplorer: Failed to initialize file explorer:', error);
    error_state.value = `Failed to initialize file explorer: ${error?.message || error}`;
  } finally {
    loading.value = false;
    debug('FileExplorer: Initialization complete');
  }
}

const showNewJuliaProjectDialog = () => {
  newJuliaProjectDialogRef.value.show = true;
};

const handleProjectRootChanged = async (newRoot) => {
  appStore.setInitialProjectLoadAttempted(false); // Reset for new project process
  try {
    loading.value = true;
    error_state.value = null;
    rootFolder.value = newRoot;

    // Save the new project path to backend state - this will trigger the selected-directory event
    try {
      await invoke('set_last_opened_folder', { path: newRoot });
      debug('Successfully saved new project path:', newRoot);
    } catch (saveError) {
      error('Failed to save new project path:', saveError);
      error_state.value = `Failed to save project path: ${saveError?.message || saveError}`;
    }

    await updateShortPath(newRoot);

    // Clear expanded state for new project
    expandedKeys.value = [];

    const tree = await invoke('get_file_tree', { rootPath: newRoot });
    // The backend returns the root directory node directly, so we need to check if it has children
    if (tree && tree.is_directory && tree.children) {
      fileTree.value = mapNodesForTree(tree.children);
    } else if (tree && tree.is_directory) {
      // Root directory exists but has no children (empty directory)
      fileTree.value = [];
    } else {
      // No tree or invalid structure
      fileTree.value = [];
    }

    // Start file watcher for the new folder
    await startFileWatcher(newRoot);

    // Emit the project root changed event
    emit('project-root-changed', newRoot);
  } catch (error) {
    error('Failed to load file tree:', error);
    error_state.value = `Failed to load file tree: ${error}`;
    // The selected-directory event will handle setting the project path and Julia project status
  } finally {
    loading.value = false;
    appStore.setInitialProjectLoadAttempted(true); // New project processing attempt complete
  }
};

// Handle context menu on empty space
function handleContainerContextMenu(e) {
  e.preventDefault();
  e.stopPropagation();

  //debug('=== CONTEXT MENU DEBUG ===');
  //debug(`Context menu triggered at: ${e.clientX}, ${e.clientY}`);
  //debug(`Event target: ${e.target.tagName} with classes: ${Array.from(e.target.classList).join(', ')}`);

  // Check if we're clicking on a tree node
  const treeNode = e.target.closest('.n-tree-node');

  if (treeNode) {
    //debug('Found tree node element');
    //debug(`Tree node classes: ${Array.from(treeNode.classList).join(', ')}`);

    // Try multiple ways to get the node data
    let nodeKey = treeNode.getAttribute('data-key');
    //debug(`data-key attribute: ${nodeKey || 'not found'}`);

    if (!nodeKey) {
      // Try getting it from the tree node's data attributes
      nodeKey = treeNode.getAttribute('data-path');
      //debug(`data-path attribute: ${nodeKey || 'not found'}`);
    }

    if (!nodeKey) {
      // Try getting the node text and finding by name
      const nodeText = treeNode.querySelector('.n-tree-node-content__text')?.textContent?.trim();
      //debug(`Node text: ${nodeText || 'not found'}`);

      if (nodeText) {
        // Find by name instead
        const findNodeByName = (nodes, name) => {
          for (const node of nodes) {
            if (node.name === name) {
              return node;
            }
            if (node.children && node.children.length > 0) {
              const found = findNodeByName(node.children, name);
              if (found) return found;
            }
          }
          return null;
        };

        const targetNode = findNodeByName(fileTree.value, nodeText);
        if (targetNode) {
          //debug(`Found target node by name: ${targetNode.name} (${targetNode.path})`);
          contextMenuTarget.value = targetNode;
          contextMenuX.value = e.clientX;
          contextMenuY.value = e.clientY;
          contextMenuVisible.value = true;
          //debug(`Context menu target set to: ${contextMenuTarget.value.name}`);
          return;
        } else {
          debug(`Could not find node by name: ${nodeText}`);
          //debug(`Available names in fileTree: ${fileTree.value.map(n => n.name).join(', ')}`);
        }
      }
    }

    if (nodeKey) {
      // Find the node in our file tree data by path
      const findNodeByPath = (nodes, path) => {
        for (const node of nodes) {
          if (node.path === path) {
            return node;
          }
          if (node.children && node.children.length > 0) {
            const found = findNodeByPath(node.children, path);
            if (found) return found;
          }
        }
        return null;
      };

      const targetNode = findNodeByPath(fileTree.value, nodeKey);

      if (targetNode) {
        debug(`Found target node for context menu: ${targetNode.name} (${targetNode.path})`);
        contextMenuTarget.value = targetNode;
        contextMenuX.value = e.clientX;
        contextMenuY.value = e.clientY;
        contextMenuVisible.value = true;
        //debug(`Context menu target set to: ${contextMenuTarget.value.name}`);
        return;
      } else {
        debug(`Could not find node with path: ${nodeKey}`);
        //debug(`Available paths: ${fileTree.value.map(n => n.path).join(', ')}`);
      }
    }
  }

  // If we get here, we're clicking on empty space
  //debug('Context menu on empty space - no tree node found');
  contextMenuTarget.value = null;
  contextMenuX.value = e.clientX;
  contextMenuY.value = e.clientY;
  contextMenuVisible.value = true;
  debug('Context menu target set to null');
}
</script>

<style scoped>
:deep(.n-tree-node-content__text) {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

:deep(.n-tree .n-tree-node) {
  padding-top: 1px;
  padding-bottom: 1px;
}

/* Adjust icon size/alignment if needed */
:deep(.n-tree-node-content__prefix) .svg-inline--fa {
  width: 1em;
  height: 1em;
}

/* Ensure tree takes full height and enables scrolling */
:deep(.n-tree) {
  height: 100% !important;
  display: flex !important;
  flex-direction: column !important;
}

:deep(.n-tree .n-tree-wrapper) {
  flex-grow: 1 !important;
  overflow: visible !important;
}

/* Ensure tree nodes don't overflow horizontally */
:deep(.n-tree-node-content) {
  min-width: 0 !important;
  overflow: hidden !important;
}

/* Styles for Project.toml Configuration Section */
.project-config-actions {
  padding: 1px 8px 8px 8px;
  background-color: transparent;
  flex-shrink: 0; /* Prevent shrinking */
}

.project-config-action-row {
  display: flex;
  justify-content: space-around; /* Distributes buttons evenly */
  margin-bottom: 2px; /* Adds a little space between rows */
}

.project-config-action-row .n-button {
  flex-grow: 1; /* Allows buttons to take available space */
  margin: 0 2px; /* Small margin between buttons */
}

/* VS Code-like Context Menu Styles */
.vscode-context-menu {
  position: fixed;
  z-index: 99999;
  background: #2d2d30;
  border: 1px solid #3e3e42;
  border-radius: 6px;
  box-shadow:
    0 8px 16px rgba(0, 0, 0, 0.4),
    0 2px 4px rgba(0, 0, 0, 0.2);
  padding: 4px 0;
  min-width: 180px;
  font-family:
    -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Oxygen', 'Ubuntu', 'Cantarell',
    sans-serif;
  font-size: 13px;
  color: #cccccc;
  user-select: none;
  outline: none;
  backdrop-filter: blur(10px);
  pointer-events: auto;
}

.context-menu-item {
  display: flex;
  align-items: center;
  padding: 6px 12px;
  cursor: pointer;
  transition: all 0.15s ease;
  position: relative;
  border-radius: 3px;
  margin: 0 4px;
}

.context-menu-item:hover {
  background-color: #094771;
  transform: translateX(1px);
}

.context-menu-item:active {
  background-color: #007acc;
  transform: translateX(1px);
}

.context-menu-item-danger:hover {
  background-color: #a1260d;
  transform: translateX(1px);
}

.context-menu-item-danger:active {
  background-color: #c42d1c;
  transform: translateX(1px);
}

.context-menu-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  margin-right: 8px;
  color: #cccccc;
  flex-shrink: 0;
}

.context-menu-item:hover .context-menu-icon {
  color: #ffffff;
}

.context-menu-item .n-icon {
  width: 16px;
  height: 16px;
}

.context-menu-label {
  flex: 1;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.context-menu-separator {
  height: 1px;
  background-color: #3e3e42;
  margin: 4px 0;
}
</style>
