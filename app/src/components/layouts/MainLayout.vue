<template>
  <div style="height: 100vh; width: 100vw; display: flex; flex-direction: row">
    <div style="width: 60px; flex-shrink: 0; background-color: #282828">
      <NavigationRail ref="navigationRail" @navigate="handleNavigation" />
    </div>
    <n-layout style="height: 100%; flex-grow: 1; position: relative; border-left: 1px solid #444">
      <Splitpanes class="default-theme" :horizontal="false" style="height: 100%; width: 100%">
        <Pane
          :size="leftPaneSize"
          min-size="15"
          style="background-color: #282828"
          v-if="showLeftPane"
        >
          <div style="display: flex; flex-direction: column; height: 100%">
            <div style="flex: 1; overflow: hidden">
              <LeftPanelAccordion
                @open-file="handleOpenFile"
                @open-package-settings="handleOpenPackageSettings"
                @project-root-changed="handleProjectRootChanged"
              />
            </div>
          </div>
        </Pane>
        <Pane :size="centerPaneSizeComputed">
          <!-- Main content will be rendered by Vue Router -->
          <router-view />
          <div
            v-if="
              projectChangeStatus &&
              (projectChangeStatus.message || projectChangeStatus.progress_percentage !== undefined)
            "
            class="status-chip"
          >
            <span>{{ projectChangeStatus.message || 'Working…' }}</span>
            <span v-if="projectChangeStatus.progress_percentage !== undefined">
              &nbsp;({{ projectChangeStatus.progress_percentage }}%)
            </span>
          </div>

          <!-- Welcome screen when no project is selected -->
          <div
            v-if="!appStore.projectPath && appStore.initialProjectLoadAttempted"
            class="welcome-screen"
          >
            <div class="welcome-content">
              <div class="welcome-icon">
                <n-icon size="64" color="#4fc3f7">
                  <FolderOpenOutline />
                </n-icon>
              </div>
              <h1 class="welcome-title">Welcome to Compute42</h1>
              <p class="welcome-subtitle">Your Julia Development Environment</p>

              <div class="welcome-actions">
                <n-space vertical size="large">
                  <n-card title="Get Started" size="small">
                    <template #default>
                      <p>Choose one of the following options to begin:</p>
                      <n-space vertical size="medium" style="margin-top: 16px">
                        <n-button
                          type="primary"
                          size="large"
                          @click="handleOpenFolder"
                          style="width: 100%"
                        >
                          <template #icon>
                            <n-icon><FolderOpenOutline /></n-icon>
                          </template>
                          Open Existing Julia Project
                        </n-button>
                        <n-button
                          type="info"
                          size="large"
                          @click="handleCreateProject"
                          style="width: 100%"
                        >
                          <template #icon>
                            <n-icon><AddOutline /></n-icon>
                          </template>
                          Create New Julia Project
                        </n-button>
                      </n-space>
                    </template>
                  </n-card>

                  <n-card title="Features" size="small">
                    <template #default>
                      <n-space vertical size="small">
                        <div class="feature-item">
                          <n-icon color="#4fc3f7"><CodeOutline /></n-icon>
                          <span>Julia Language Server integration</span>
                        </div>
                        <div class="feature-item">
                          <n-icon color="#4fc3f7"><TerminalOutline /></n-icon>
                          <span>Integrated terminal</span>
                        </div>
                        <div class="feature-item">
                          <n-icon color="#4fc3f7"><GitBranchOutline /></n-icon>
                          <span>Package management</span>
                        </div>
                        <div class="feature-item">
                          <n-icon color="#4fc3f7"><AnalyticsOutline /></n-icon>
                          <span>Code analysis and reports</span>
                        </div>
                      </n-space>
                    </template>
                  </n-card>
                </n-space>
              </div>
            </div>
          </div>
        </Pane>
        <Pane
          :size="rightPaneSizeComputed"
          min-size="0"
          style="background-color: #282828"
          v-if="shouldShowRightPane"
        >
          <div style="display: flex; flex-direction: column; height: 100%">
            <!-- References Panel -->
            <div
              v-if="referencesPanelData"
              style="flex: 1; overflow: hidden; border-top: 1px solid #444"
            >
              <ReferencesPanel
                :uri="referencesPanelData.uri"
                :line="referencesPanelData.line"
                :character="referencesPanelData.character"
                @close="handleReferencesPanelClose"
                @navigate="handleReferenceNavigate"
              />
            </div>
          </div>
        </Pane>
      </Splitpanes>
    </n-layout>
    <!-- Right Navigation Rail - hidden when right panel is open -->
    <div
      v-if="!shouldShowRightPane"
      style="width: 60px; flex-shrink: 0; background-color: #282828; border-left: 1px solid #444"
    >
      <RightNavigationRail />
    </div>
  </div>
</template>

<script setup>
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { NLayout, useMessage, NAlert, NIcon, NSpace, NCard, NButton } from 'naive-ui';
import { Splitpanes, Pane } from 'splitpanes';
import 'splitpanes/dist/splitpanes.css';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { debug, info, error, warn } from '../../utils/logger';
import { useProjectChangeStatus } from '../../features/orchestrator/useProjectChangeStatus';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import {
  FolderOpenOutline,
  AddOutline,
  CodeOutline,
  TerminalOutline,
  GitBranchOutline,
  AnalyticsOutline,
} from '@vicons/ionicons5';

import NavigationRail from './NavigationRail.vue';
import RightNavigationRail from './RightNavigationRail.vue';
import LeftPanelAccordion from './LeftPanelAccordion.vue';
import ReferencesPanel from '../shared/ReferencesPanel.vue';
import { useAppStore } from '../../store/appStore';
import { storeToRefs } from 'pinia';
import { useTerminalStore } from '../../store/terminalStore';

const router = useRouter();
const route = useRoute();
const message = useMessage();
const appStore = useAppStore();
const terminalStore = useTerminalStore();
const { activeTerminalId } = storeToRefs(terminalStore);

const showLeftPane = ref(true);
const showRightPane = ref(false);
const leftPaneSize = ref(25);
const rightPaneSize = ref(0); // Start with 0 since right panel should be hidden by default
const rightPanelMode = ref('none'); // 'none' | 'references'
const referencesPanelData = ref(null);
const selectedDirectoryListenerSetup = ref(false);
const { status: projectChangeStatus } = useProjectChangeStatus();

// Initialize right pane to be fully collapsed
const rightPaneSizeComputed = computed(() => {
  const size = showRightPane.value ? rightPaneSize.value : 0; // Use 0 when not showing to fully collapse
  return size;
});

// Show right pane when there are references
const shouldShowRightPane = computed(() => {
  return referencesPanelData.value !== null;
});

// Single watcher to handle right pane visibility changes
watch(
  shouldShowRightPane,
  (newValue, oldValue) => {
    if (newValue) {
      // When showing the right pane, set a default size if it's currently 0
      if (rightPaneSize.value === 0) {
        rightPaneSize.value = 25;
      }
      showRightPane.value = true;
    } else {
      // When hiding the right pane, force the size to 0
      rightPaneSize.value = 0;
      showRightPane.value = false;
    }
  },
  { immediate: true }
);

const handleOpenFolder = async () => {
  try {
    const result = await openDialog({ directory: true, multiple: false });
    if (result && !Array.isArray(result)) {
      appStore.setProjectPath(result);
    } else if (Array.isArray(result) && result.length > 0) {
      appStore.setProjectPath(result[0]);
    }
  } catch (error) {
    error('MainLayout: Failed to open folder from welcome screen:', error);
    message.error('Failed to open folder');
  }
};

const handleCreateProject = () => {
  // Navigate to home view and trigger project creation
  router.push({ name: 'Home' });
  // The file explorer has the new project button
  message.info('Use the "+" button in the file explorer to create a new Julia project');
};

const activeProjectRoot = ref(appStore.projectPath);

// Initialize project when component mounts
onMounted(async () => {
  // Set left pane visibility based on current route (not localStorage)
  if (route.name === 'Home') {
    showLeftPane.value = true;
  } else if (['About', 'PackageManagement', 'Settings'].includes(route.name)) {
    showLeftPane.value = false;
  }

  // Ensure right panel is hidden by default
  showRightPane.value = false;
  referencesPanelData.value = null;

  // Set up listener for selected-directory events (unified)
  let unlistenSelectedDirectory;
  try {
    // Check if we already have a listener to prevent duplicate setup
    if (selectedDirectoryListenerSetup.value) {
      return;
    }
    selectedDirectoryListenerSetup.value = true;
    unlistenSelectedDirectory = await listen('orchestrator:selected-directory', (event) => {
      if (event.payload && typeof event.payload === 'object') {
        const { path, is_julia_project } = event.payload;

        if (path && typeof path === 'string') {
          appStore.setProjectPath(path);
          appStore.setIsJuliaProject(is_julia_project || false);

          // Note: Project is already activated during backend startup, so we don't need to activate it again
          // The backend startup process handles project activation automatically
        } else {
        }
      } else {
      }

      // Mark that we've attempted to load the project
      appStore.setInitialProjectLoadAttempted(true);
    });
  } catch (err) {
    error('MainLayout.vue: Failed to set up selected-directory event listener:', err);
  }

  // Signal that frontend is ready to receive the selected directory
  // Note: frontend_ready is now handled by the backend-ready handshake mechanism
  appStore.setInitialProjectLoadAttempted(true);

  // Clean up listener when component unmounts
  onUnmounted(() => {
    if (unlistenSelectedDirectory) {
      unlistenSelectedDirectory();
      selectedDirectoryListenerSetup.value = false;
    }
  });
});

watch(
  () => appStore.projectPath,
  async (newPath) => {
    activeProjectRoot.value = newPath;

    // Note: Project activation is handled during backend startup, so we don't need to activate again here
    // The selected-directory event already provides the project status, and the backend startup process
    // handles the actual Julia project activation
  }
);

const centerPaneSizeComputed = computed(() => {
  return 100 - (showLeftPane.value ? leftPaneSize.value : 0) - rightPaneSizeComputed.value;
});

const handleNavigation = (viewKey) => {
  if (viewKey === 'explorer') {
    router.push({ name: 'Home' });
  } else if (viewKey === 'about') {
    router.push({ name: 'About' });
  } else if (viewKey === 'packages') {
    router.push({ name: 'PackageManagement' });
  } else if (viewKey === 'settings') {
    router.push({ name: 'Settings' });
  }
};

const handleOpenFile = (filePath) => {
  appStore.setFileToOpen(filePath);
  if (route.name !== 'Home') {
    router.push({ name: 'Home' });
  }
};

const handleProjectRootChanged = async (newRoot) => {
  appStore.setProjectPath(newRoot);

  if (activeTerminalId.value && newRoot) {
    try {
      // Only create a new terminal if the directory actually changed
      if (newRoot !== appStore.projectPath) {
        // Close the current terminal
        await invoke('close_terminal_session', {
          id: activeTerminalId.value,
        });

        // Initialize a new terminal with the new directory
        const newTerminalId = await invoke('init_terminal_session', {
          initialDirectory: newRoot,
        });

        // Update the active terminal ID
        terminalStore.setActiveTerminalId(newTerminalId);
      }
    } catch (err) {
      error('Failed to initialize new terminal:', err);
      message.error(`Failed to initialize new terminal: ${err}`);
    }
  }
  appStore.setFileToOpen(null);
  router.push({ name: 'Home' });
};

const handleOpenPackageSettings = (projectPath) => {
  if (projectPath) {
    // Navigate to the full-screen package management view
    router.push({ name: 'PackageManagement' });
  } else {
    message.error('Project path is missing, cannot open package manager.');
  }
};

// Derive left pane visibility from current route (not from localStorage)
watch(
  route,
  (to, from) => {
    // Only clear right panel data when actually changing routes, not on initial load
    if (from && from.name !== to.name) {
      showRightPane.value = false;
      referencesPanelData.value = null;
      rightPanelMode.value = 'none';
    }

    // Left pane visibility is derived purely from the current route
    if (to.name === 'Home') {
      // Always show left pane when on Home route
      showLeftPane.value = true;
    } else if (['About', 'PackageManagement', 'Settings'].includes(to.name)) {
      // Hide left pane on full-screen views
      showLeftPane.value = false;
      // Clear right panel data when leaving Home route
      showRightPane.value = false;
      referencesPanelData.value = null;
      rightPanelMode.value = 'none';
    }
  },
  { immediate: true }
);

if (appStore.projectPath) {
  activeProjectRoot.value = appStore.projectPath;
}

// Reference to the NavigationRail component
const navigationRail = ref(null);

// Methods to control right pane visibility
const showRightPaneContent = (content) => {
  showRightPane.value = true;
  // Emit event to notify child components that right pane is now visible
  window.dispatchEvent(new CustomEvent('right-pane-show', { detail: content }));
};

const hideRightPane = () => {
  showRightPane.value = false;
  // Clear the references panel data
  referencesPanelData.value = null;
  // Emit event to notify child components that right pane is now hidden
  window.dispatchEvent(new CustomEvent('right-pane-hide'));
};

// Handle references panel close
const handleReferencesPanelClose = () => {
  referencesPanelData.value = null;
};

// Handle reference navigation from ReferencesPanel
const handleReferenceNavigate = async (location) => {
  try {
    // Convert URI to file path - properly decode the URI and handle Windows paths
    let filePath = location.uri;

    // First decode any URL-encoded characters
    filePath = decodeURIComponent(filePath);

    // Remove file:// protocol
    filePath = filePath.replace('file://', '');

    // Handle Windows paths with leading slash
    if (filePath.startsWith('/') && filePath.match(/^\/[A-Z]:/i)) {
      filePath = filePath.substring(1); // Remove leading slash
    }

    // Open the file if not already open
    appStore.setFileToOpen(filePath);

    // Set up navigation to the specific location
    appStore.setPendingNavigationRequest({
      startLineNumber: location.range.start.line + 1,
      startColumn: location.range.start.character + 1,
      endLineNumber: location.range.end.line + 1,
      endColumn: location.range.end.character + 1,
    });

    // Navigate to home view if not already there
    if (route.name !== 'Home') {
      router.push({ name: 'Home' });
    }

    // Don't hide the right pane - let user close it manually
    // hideRightPane();
  } catch (err) {
    error('MainLayout: Failed to navigate to reference');
  }
};

// Expose methods for child components
defineExpose({
  showRightPaneContent,
  hideRightPane,
  showRightPane: () => (showRightPane.value = true),
});

// Add event listener for source code icon clicks from the Function and Method Analysis table
const handleOpenFileAndNavigate = (event) => {
  // First, make sure we're on the home view to access the editor
  if (route.name !== 'Home') {
    router.push({ name: 'Home' });
  }

  // Set the file to open
  appStore.setFileToOpen(event.detail.filePath);

  // Store the navigation request in the app store for the editor to handle
  appStore.setPendingNavigationRequest(event.detail.range);

  // Update the navigation rail to show explorer as active
  // Use the exposed selectView method directly
  if (navigationRail.value) {
    navigationRail.value.selectView('explorer');
  } else {
    // Fallback to the handleNavigation function
    handleNavigation('explorer');
  }
};

// Add the event listener when the component is mounted
window.addEventListener('open-file-and-navigate', handleOpenFileAndNavigate);

// Add event listener for showing references panel in right pane
const handleShowReferencesPanel = (event) => {
  console.log('MainLayout: Received show-references-panel event:', event.detail);
  showRightPane.value = true;
  // Store the references request data for the slot content
  referencesPanelData.value = event.detail;
  console.log('MainLayout: Set referencesPanelData to:', referencesPanelData.value);
};

window.addEventListener('show-references-panel', handleShowReferencesPanel);

// Remove the event listeners when the component is unmounted
onUnmounted(() => {
  window.removeEventListener('open-file-and-navigate', handleOpenFileAndNavigate);
  window.removeEventListener('show-references-panel', handleShowReferencesPanel);
});
</script>

<style>
.splitpanes.default-theme .splitpanes__splitter {
  background-color: #333;
}
.splitpanes.default-theme.splitpanes--vertical .splitpanes__splitter {
  width: 3px;
  border-left: 1px solid #444;
}
.splitpanes.default-theme.splitpanes--vertical .splitpanes__splitter:hover {
  border-left: 1px solid #555;
}
.splitpanes.default-theme.splitpanes--horizontal .splitpanes__splitter {
  height: 3px;
  width: 100%;
  border-top: 1px solid #444;
}
.splitpanes.default-theme.splitpanes--horizontal .splitpanes__splitter:hover {
  border-top: 1px solid #555;
}

.error-actions,
.success-actions {
  display: flex;
  gap: 1rem;
  justify-content: center;
}

.welcome-screen {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 100%;
  background-color: #1e1e1e;
}

.welcome-content {
  text-align: center;
  max-width: 600px;
  padding: 40px;
}

.welcome-icon {
  margin-bottom: 24px;
}

.welcome-title {
  font-size: 2.5rem;
  font-weight: 600;
  margin: 0 0 8px 0;
  color: #ffffff;
}

.welcome-subtitle {
  font-size: 1.2rem;
  color: #b0b0b0;
  margin: 0 0 40px 0;
}

.welcome-actions {
  max-width: 400px;
  margin: 0 auto;
}

.feature-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 0;
}

.feature-item span {
  color: #e0e0e0;
  font-size: 0.95rem;
}
</style>
<style scoped>
.status-chip {
  position: absolute;
  top: 8px;
  right: 8px;
  background: rgba(40, 40, 40, 0.9);
  color: #ddd;
  border: 1px solid #444;
  border-radius: 14px;
  padding: 4px 10px;
  font-size: 12px;
  z-index: 5;
}
</style>
