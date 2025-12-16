<template>
  <div class="modal-overlay">
    <div class="modal-container compact">
      <!-- Header with Logo -->
      <div class="header-content">
        <div class="logo-container">
          <img src="/icon.png" alt="Compute42 Logo" class="logo" />
        </div>
        <h1 class="app-title">Switching Project</h1>
        <p class="app-subtitle">Julia Development Environment</p>
      </div>

      <!-- Current Step Message -->
      <div class="current-step">
        <div class="step-icon" :class="statusClass">
          <n-icon v-if="errorMessage" size="32" class="error-icon">
            <CloseOutline />
          </n-icon>
          <n-icon v-else-if="isComplete" size="32" class="success-icon">
            <CheckmarkOutline />
          </n-icon>
          <n-spin v-else size="large" />
        </div>
        <div class="step-message">{{ mainStatusMessage }}</div>
      </div>

      <!-- Progress Bar -->
      <div class="progress-section">
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: progressPercentage + '%' }"></div>
        </div>
      </div>

      <!-- Error Message -->
      <div v-if="errorMessage" class="error-message compact">
        <p>{{ errorMessage }}</p>
        <button @click="retrySwitch" class="retry-button">Retry</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from 'vue';
import { NIcon, NSpin } from 'naive-ui';
import { CloseOutline, CheckmarkOutline } from '@vicons/ionicons5';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { debug, error } from '../../utils/logger';

interface ProjectChangeStatusPayload {
  message: string;
  progress_percentage: number;
}

interface ProjectChangeCompletePayload {
  project_path: string;
}

interface LspStatusPayload {
  status: string;
  message: string;
  error?: string;
  project_path?: string;
}

interface LspPackagesLoadedPayload {
  // Empty payload for now
}

const emit = defineEmits<{
  projectSwitchComplete: [];
}>();

// Project switch state
const mainStatusMessage = ref('Switching project...');
const isError = ref(false);
const isComplete = ref(false);
const errorMessage = ref('');

// Progress bar state
const progressPercentage = ref(0);

// Track LSP initialization state
const lspPackagesLoaded = ref(false);
const projectChangeCompleted = ref(false);

// Track all unlisten functions for cleanup
let unlistenFunctions: (() => void)[] = [];

// statusClass for the step icon
const statusClass = computed(() => {
  if (errorMessage.value) return 'error';
  if (isComplete.value) return 'success';
  return 'loading';
});

const retrySwitch = () => {
  // Reset state and restart switch
  errorMessage.value = '';
  mainStatusMessage.value = 'Switching project...';
  progressPercentage.value = 0;
  isComplete.value = false;
  isError.value = false;
  lspPackagesLoaded.value = false;
  projectChangeCompleted.value = false;
  // For now, just complete the switch - in a real implementation, you might want to retry the actual switch
  setTimeout(() => {
    emit('projectSwitchComplete');
  }, 500);
};

const setupEventListeners = async () => {
  try {
    // Set up event listeners for project switching events
    const eventListeners = [
      // Project change status event
      listen<ProjectChangeStatusPayload>('project-change-status', (event) => {
        debug('ProjectSwitchModal: Received project-change-status event.');
        debug(`Payload: ${JSON.stringify(event.payload, null, 2)}`);

        // Update status message and progress
        mainStatusMessage.value = event.payload.message;
        progressPercentage.value = event.payload.progress_percentage;
      }),

      // Project change complete event
      listen<ProjectChangeCompletePayload>('project-change-complete', (event) => {
        debug('ProjectSwitchModal: Received project-change-complete event.');
        debug(`Payload: ${JSON.stringify(event.payload, null, 2)}`);

        // Mark project change as complete
        projectChangeCompleted.value = true;
        mainStatusMessage.value = `Project switched to: ${event.payload.project_path}`;
        progressPercentage.value = 90; // Project change complete, waiting for LSP

        // Check if we can complete the switch (both project change and LSP ready)
        checkAndCompleteSwitch();
      }),

      // LSP packages loaded event
      listen<LspPackagesLoadedPayload>('lsp-packages-loaded', (event) => {
        debug('ProjectSwitchModal: Received lsp-packages-loaded event.');
        debug(`Payload: ${JSON.stringify(event.payload, null, 2)}`);

        // Mark LSP as fully initialized
        lspPackagesLoaded.value = true;
        mainStatusMessage.value = 'Language Server packages loaded';
        progressPercentage.value = 100;

        // Check if we can complete the switch (both project change and LSP ready)
        checkAndCompleteSwitch();
      }),

      // LSP status event
      listen<LspStatusPayload>('lsp-status', (event) => {
        debug('ProjectSwitchModal: Received lsp-status event.');
        debug(`Payload: ${JSON.stringify(event.payload, null, 2)}`);

        // Update status message based on LSP status
        if (event.payload.status === 'failed') {
          // Show error but don't fail the switch
          errorMessage.value = `Language Server failed to start: ${event.payload.error}`;
          mainStatusMessage.value = `Project switched but Language Server failed: ${event.payload.error}`;
          debug(
            `ProjectSwitchModal: LSP failed but project switch completed: ${event.payload.error}`
          );

          // If LSP failed, we can still complete the switch
          projectChangeCompleted.value = true;
          checkAndCompleteSwitch();
        } else if (event.payload.status === 'starting') {
          mainStatusMessage.value = event.payload.message;
          progressPercentage.value = 80; // LSP starting is near the end
          debug('ProjectSwitchModal: LSP starting');
        } else if (event.payload.status === 'ready') {
          mainStatusMessage.value = event.payload.message;
          progressPercentage.value = 95; // LSP ready is almost complete
          debug('ProjectSwitchModal: LSP ready');
        } else if (event.payload.status === 'stopped') {
          mainStatusMessage.value = event.payload.message;
          progressPercentage.value = 100; // LSP stopped (for non-Julia projects) - complete
          debug('ProjectSwitchModal: LSP stopped');

          // If LSP is stopped (non-Julia project), we can complete the switch immediately
          // since no LSP packages need to be loaded
          projectChangeCompleted.value = true;
          lspPackagesLoaded.value = true; // Mark as loaded since no packages need loading
          checkAndCompleteSwitch();
        } else {
          // Other LSP statuses
          mainStatusMessage.value = event.payload.message;
          debug(`ProjectSwitchModal: LSP status: ${event.payload.status}`);
        }
      }),
    ];

    // Set up all listeners in parallel
    const unlistenFns = await Promise.all(eventListeners);
    unlistenFunctions = unlistenFns.map((fn) => fn as UnlistenFn);

    debug('ProjectSwitchModal: All event listeners set up successfully');
  } catch (err) {
    error(
      `ProjectSwitchModal: Failed to set up event listeners: ${err instanceof Error ? err.message : String(err)}`
    );
  }
};

// Function to check if both project change and LSP initialization are complete
const checkAndCompleteSwitch = () => {
  // Complete the switch if:
  // 1. Project change is complete AND
  // 2. Either LSP packages are loaded OR LSP failed/stopped (non-Julia project)
  const canComplete =
    projectChangeCompleted.value &&
    (lspPackagesLoaded.value ||
      errorMessage.value ||
      mainStatusMessage.value.includes('LSP stopped') ||
      mainStatusMessage.value.includes('non-Julia project'));

  if (canComplete) {
    debug(
      'ProjectSwitchModal: Both project change and LSP initialization complete, finishing switch'
    );

    // Mark as complete and show final message
    isComplete.value = true;

    // Complete switch after a short delay
    setTimeout(() => {
      emit('projectSwitchComplete');
    }, 1000); // 1 second delay
  } else {
    debug(
      `ProjectSwitchModal: Waiting for completion - Project change: ${projectChangeCompleted.value}, LSP loaded: ${lspPackagesLoaded.value}`
    );
  }
};

const cleanupEventListeners = () => {
  debug('ProjectSwitchModal: Cleaning up all event listeners.');
  unlistenFunctions.forEach((unlisten) => {
    try {
      unlisten();
    } catch (err) {
      error(
        `ProjectSwitchModal: Error during unlisten: ${err instanceof Error ? err.message : String(err)}`
      );
    }
  });
  unlistenFunctions = [];
};

onMounted(async () => {
  debug('ProjectSwitchModal: Component mounted, setting up event listeners');
  await setupEventListeners();
});

onUnmounted(() => {
  cleanupEventListeners();
});
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: rgba(0, 0, 0, 0.8);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 9999;
}

.modal-container {
  background-color: #2a2a2a;
  border-radius: 12px;
  padding: 40px;
  max-width: 500px;
  width: 90%;
  text-align: center;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5);
  border: 1px solid #3c3c3c;
}

.modal-container.compact {
  padding: 30px;
  max-width: 450px;
}

.header-content {
  margin-bottom: 30px;
}

.logo-container {
  margin-bottom: 16px;
}

.logo {
  width: 64px;
  height: 64px;
  border-radius: 12px;
}

.app-title {
  font-size: 24px;
  font-weight: 600;
  color: #ffffff;
  margin: 0 0 8px 0;
}

.app-subtitle {
  font-size: 14px;
  color: #a0a0a0;
  margin: 0 0 16px 0;
}

.current-step {
  display: flex;
  flex-direction: column;
  align-items: center;
  margin-bottom: 30px;
}

.step-icon {
  margin-bottom: 16px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.step-icon.loading {
  color: #4fc3f7;
}

.step-icon.success {
  color: #4caf50;
}

.step-icon.error {
  color: #f44336;
}

.step-message {
  font-size: 16px;
  color: #ffffff;
  text-align: center;
  line-height: 1.5;
}

.progress-section {
  margin-bottom: 20px;
}

.progress-bar {
  width: 100%;
  height: 6px;
  background-color: #3c3c3c;
  border-radius: 3px;
  overflow: hidden;
  position: relative;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #4fc3f7, #2196f3);
  border-radius: 3px;
  transition: width 0.3s ease;
}

.error-message {
  background-color: #3c1f1f;
  border: 1px solid #f44336;
  border-radius: 8px;
  padding: 16px;
  margin-top: 20px;
}

.error-message.compact {
  padding: 12px;
}

.error-message p {
  color: #ff6b6b;
  margin: 0 0 12px 0;
  font-size: 14px;
}

.retry-button {
  background-color: #f44336;
  color: white;
  border: none;
  padding: 8px 16px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
  transition: background-color 0.2s;
}

.retry-button:hover {
  background-color: #d32f2f;
}

.error-icon {
  color: #f44336;
}

.success-icon {
  color: #4caf50;
}
</style>
