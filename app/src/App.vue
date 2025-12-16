<script setup lang="ts">
// This starter template is using Vue 3 <script setup> SFCs
// Check out https://vuejs.org/api/sfc-script-setup.html#script-setup
import { ref, onMounted, onUnmounted, watch } from 'vue';

import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core'; // Import invoke
import StartupModal from './components/shared/StartupModal.vue';
import ProjectSwitchModal from './components/shared/ProjectSwitchModal.vue';
import NotificationToast from './components/shared/NotificationToast.vue';
import WelcomeModal from './components/shared/WelcomeModal.vue';
import ErrorScreen from './components/shared/ErrorScreen.vue';
import { NConfigProvider, NGlobalStyle, NMessageProvider, NSpin, darkTheme } from 'naive-ui';
import { themeOverrides } from './theme';
// MainLayout is now handled by the router

import { debug, logError } from './utils/logger'; // Import our logger
import { useAppStore } from './store/appStore'; // Import app store
import { applicationService } from './services/applicationService'; // Import application service
import { unifiedEventService, EventCategory } from './services/unifiedEventService'; // Import unified event service

// Import new types - removed unused imports

// Initialize stores
const appStore = useAppStore();

// No longer need useMessage or most other imports/logic here

// Correct approach: Initialize as ref/shallowRef, assign in onMounted
// const messageApi = shallowRef<any | null>(null); // Using any for now to bypass linter, can refine later

// Matches the Rust struct SetupStatusPayload
// Removed unused SetupStatusPayload interface

// Note: Removed unused FinalSetupStatusResponse type alias

const startupReady = ref(false);
const authenticationComplete = ref(false);
const continueStartupSent = ref(false);
const isProjectSwitching = ref(false);
const showWelcomeModal = ref(false); // Start as false, will be set by AccountManager events
const welcomeModalMode = ref<
  'checking' | 'login' | 'signup' | 'emailVerification' | 'eulaAgreement'
>('checking');
const welcomeModalUserEmail = ref('');
const welcomeModalAuthError = ref('');
let eventUnlistenFn: UnlistenFn | null = null;

// Watch for startupReady changes to debug modal visibility
watch(startupReady, (newValue, oldValue) => {
  if (newValue === true) {
    debug('App.vue: MainLayout will be shown because startupReady is now true');
  }
});

// Watch for authenticationComplete changes to debug authentication flow
watch(authenticationComplete, (newValue, oldValue) => {
  if (newValue === true) {
    debug('App.vue: Authentication complete, StartupModal should now be visible');
  }
});

const finalSetupErrorMessage = ref<string | null>(null);
const projectPath = ref<string | null>(null); // Assuming this gets set somehow, e.g., after setup or project selection
const showErrorScreen = ref(false);

// Reactive state for Julia Daemon Status
// const juliaDaemonStatus = shallowRef<any | null>(null); // Removed daemon status tracking
// const showJuliaLogsModal = ref(false);

// Notification state
const notification = ref<{
  show: boolean;
  title: string;
  message: string;
  type: 'info' | 'success' | 'warning' | 'error';
}>({
  show: false,
  title: '',
  message: '',
  type: 'info',
});

// To store the unlisten function for the global status event
// let juliaDaemonStatusUnlistenFn: UnlistenFn | null = null; // Removed daemon listener

// Store unlisten functions for backend busy state events
let unlistenBackendBusyFn: UnlistenFn | null = null;
let unlistenBackendDoneFn: UnlistenFn | null = null;

// Removed unused Julia daemon monitoring helpers

// Removed unused Julia daemon monitoring helpers

// Track if we're in the initial startup phase (including project activation)
// This is used to suppress project switching modal during initial startup
const isInitialStartupPhase = ref(true);

// (NEW) Handle startup completion from modal - now includes project activation
const handleStartupComplete = async () => {
  startupReady.value = true;
  authenticationComplete.value = true; // Ensure authentication is marked as complete
  appStore.setJuliaDaemonReady(true); // Set Julia daemon as ready in store

  // Mark initial startup phase as complete after a short delay
  // This allows any delayed project-change-status events to be suppressed
  setTimeout(() => {
    isInitialStartupPhase.value = false;
    debug(
      'App.vue: Initial startup phase complete - project switching modal will now be shown for manual switches'
    );
  }, 2000); // 2 second delay to catch any delayed events

  // Note: frontend_ready is now handled by the backend-ready handshake mechanism
};

// Note: handleWelcomeComplete is no longer needed - authentication-complete event handler manages modal transitions

// Handle welcome modal mode change
const handleWelcomeModalModeChange = (mode: 'login' | 'signup') => {
  welcomeModalMode.value = mode;
  welcomeModalAuthError.value = ''; // Clear any previous errors when switching modes
};

// Handle project activation separately - don't show startup modal again
const handleProjectActivation = () => {
  debug(
    'App.vue: Project activation started, but keeping startup modal visible for continuous process'
  );
  // Don't set startupReady to true again - keep the existing startup modal visible
  // The StartupModal will handle the project activation internally
};

// Backend setup completion now handled by StartupModal

onMounted(async () => {
  try {
    // Open-source version - skip validity checks
    // Set authentication as complete immediately
    authenticationComplete.value = true;
    showWelcomeModal.value = false;

    // Initialize stores immediately (no authentication needed)
    try {
      const { useTerminalStore } = await import('./store/terminalStore');
      const terminalStore = useTerminalStore();
      await terminalStore.initializeGlobalStream();
    } catch (streamError) {
      await logError('App.vue: Failed to initialize global Julia stream listening', streamError);
    }

    try {
      const { usePlotStore } = await import('./store/plotStore');
      usePlotStore();
    } catch (plotError) {
      await logError('App.vue: Failed to initialize plot store', plotError);
    }

    // Set up global listener for workspace variables
    try {
      await listen('workspace:variables-updated', async (event: any) => {
        if (event.payload) {
          appStore.setWorkspaceVariables(event.payload);
        }
      });
    } catch (varsError) {
      await logError('App.vue: Failed to initialize workspace variables listener', varsError);
    }

    // Continue orchestrator startup immediately (no authentication needed)
    try {
      if (!continueStartupSent.value) {
        continueStartupSent.value = true;
        await invoke('continue_orchestrator_startup');
      }
    } catch (error) {
      await logError('App.vue: Failed to continue orchestrator startup', error);
    }

    // Application is valid, set up event listeners first, then signal frontend ready

    // Skip account event listeners in open-source version (no authentication)

    // Check backend startup phase before sending handshake
    // This handles reconnection scenarios (e.g., after sleep/wake)
    try {
      // Wait 1 second to ensure backend is fully initialized and ready
      await new Promise((resolve) => setTimeout(resolve, 1000));

      // Check current startup phase
      const startupPhase = await invoke<string>('get_orchestrator_startup_phase');

      if (startupPhase === 'Completed') {
        // Backend is already running - restore frontend state without handshake
        startupReady.value = true;
        authenticationComplete.value = true;
        appStore.setJuliaDaemonReady(true);
        isInitialStartupPhase.value = false;
        // Tabs will be restored by EditorView.vue route watcher
      } else if (startupPhase.startsWith('Failed')) {
        // Backend failed - show error state
        // Extract error message if available (format: "Failed(\"message\")")
        const errorMatch = startupPhase.match(/Failed\("([^"]*)"\)/);
        const errorMessage = errorMatch
          ? errorMatch[1]
          : 'Backend startup failed. Please restart the application.';
        finalSetupErrorMessage.value = errorMessage;
        showErrorScreen.value = true;
      } else {
        // Backend is starting or in progress - proceed with normal handshake
        await invoke('frontend_ready_handshake');
      }
    } catch (error) {
      await logError('App.vue: Failed to check startup phase or send handshake', error);
      // On error, try to proceed with handshake as fallback
      try {
        await invoke('frontend_ready_handshake');
      } catch (handshakeError) {
        await logError('App.vue: Failed to send frontend_ready_handshake command', handshakeError);
      }
    }

    // Set up backend initialization event listeners
    await setupBackendInitializationListeners();

    // Set up Tauri event listeners for project activation
    try {
      await listen('project-activation-started', () => {
        window.dispatchEvent(new CustomEvent('project-activation-started'));
      });
    } catch (err) {
      await logError('App.vue: Failed to set up project activation event listener', err);
    }

    // Unified backend busy/done
    await unifiedEventService.addEventListener(
      EventCategory.System,
      'backend-busy',
      async (event) => {
        window.dispatchEvent(new CustomEvent('backend-busy', { detail: event.payload }));
      }
    );
    await unifiedEventService.addEventListener(
      EventCategory.System,
      'backend-done',
      async (event) => {
        window.dispatchEvent(new CustomEvent('backend-done', { detail: event.payload }));
      }
    );

    // Set up file server event listeners using unified event system
    try {
      // Listen for file server started events
      await unifiedEventService.addEventListener(
        EventCategory.File,
        'server-started',
        async (event) => {
          const payload = event.payload;
          if (payload.port) {
            appStore.setFileServerPort(payload.port);
          }
        }
      );

      // Listen for file server error events (non-fatal - startup continues)
      await unifiedEventService.addEventListener(
        EventCategory.File,
        'server-error',
        async (event) => {
          const payload = event.payload;
          if (payload.error || payload.message) {
            const errorMsg = payload.error || payload.message;
            debug(`App.vue: File server error: ${errorMsg}`);
            // Store error in app store for file explorer to display
            appStore.setFileServerError(errorMsg);
          }
        }
      );

      // Listen for plot server started events
      await unifiedEventService.addEventListener(
        EventCategory.Plot,
        'server-started',
        async (event) => {
          const payload = event.payload;
          if (payload.port) {
            // Plot server port is now handled in plotStore
            // This listener is kept for backward compatibility
            debug(`App.vue: Plot server started on port ${payload.port}`);
          }
        }
      );
    } catch (err) {
      await logError('App.vue: Failed to set up file/plot server event listeners', err);
    }

    // Unified selected-directory
    await unifiedEventService.addEventListener(
      EventCategory.Orchestrator,
      'selected-directory',
      async (event) => {
        const payload = event.payload as { path?: string; is_julia_project?: boolean };
        if (payload && typeof payload === 'object' && typeof payload.path === 'string') {
          appStore.setProjectPath(payload.path);
          appStore.setIsJuliaProject(!!payload.is_julia_project);
        }
        appStore.setInitialProjectLoadAttempted(true);
      }
    );

    // Set up unified event listeners for orchestrator events
    try {
      // Listen for orchestrator startup update events
      await unifiedEventService.addEventListener(
        EventCategory.Orchestrator,
        'startup-update',
        async (event) => {
          const payload = event.payload;
          if (payload.message && payload.progress !== undefined) {
            // Startup progress update
          }
        }
      );

      // Listen for orchestrator startup ready event
      await unifiedEventService.addEventListener(
        EventCategory.Orchestrator,
        'startup-ready',
        async (event) => {
          const payload = event.payload;
          if (payload.message) {
            // Startup ready
          }

          // Note: Don't transition to main application here anymore
          // The StartupModal will handle the transition after both orchestrator and LSP are ready
        }
      );

      // Listen for orchestrator initialization status events
      await unifiedEventService.addEventListener(
        EventCategory.Orchestrator,
        'initialization-status',
        async (event) => {
          const payload = event.payload;
          if (payload.message && payload.progress !== undefined) {
            if (payload.is_error && payload.error_details) {
              // Initialization error
            }
          }
        }
      );

      // Listen for orchestrator project change complete events
      await unifiedEventService.addEventListener(
        EventCategory.Orchestrator,
        'project-change-complete',
        async (event) => {
          const payload = event.payload;
          if (payload.project_path) {
            // Hide project switching modal after a delay
            setTimeout(() => {
              isProjectSwitching.value = false;
            }, 1000);
          }
        }
      );
    } catch (err) {
      await logError('App.vue: Failed to set up unified orchestrator event listeners', err);
    }

    // Set up unified event listeners for LSP events
    try {
      // Listen for LSP status events
      await unifiedEventService.addEventListener(EventCategory.Lsp, 'status', async (event) => {
        const payload = event.payload;
        if (payload.status && payload.message) {
          // Update LSP status in the store
          appStore.setLspStatus({
            status: payload.status as
              | 'unknown'
              | 'starting'
              | 'started'
              | 'initialized'
              | 'loading-cache'
              | 'ready'
              | 'failed'
              | 'stopped',
            message: payload.message,
            error: payload.error,
            projectPath: payload.project_path,
          });
        }
      });

      // Listen for LSP ready events
      await unifiedEventService.addEventListener(EventCategory.Lsp, 'ready', async (event) => {
        const payload = event.payload;
        if (payload.status) {
          // Update LSP status in the store to ensure it shows as ready
          appStore.setLspStatus({
            status: 'ready',
            message: payload.message || 'Language Server is ready',
            projectPath: payload.project_path || undefined,
          });
        }
      });

      // Listen for LSP installation events
      await unifiedEventService.addEventListener(
        EventCategory.Lsp,
        'installation-started',
        async (_event) => {
          // LSP installation started
        }
      );

      await unifiedEventService.addEventListener(
        EventCategory.Lsp,
        'installation-progress',
        async (event) => {
          const payload = event.payload;
          if (payload.message && payload.progress !== undefined) {
            // LSP installation progress
          }
        }
      );

      await unifiedEventService.addEventListener(
        EventCategory.Lsp,
        'installation-complete',
        async (_event) => {
          // LSP installation complete
        }
      );
    } catch (err) {
      await logError('App.vue: Failed to set up unified LSP event listeners', err);
    }

    // Set up additional LSP event listeners for comprehensive status tracking
    try {
      // Use unified lsp:ready instead of legacy
      // LSP installation started
      // Use unified lsp:installation-started
      // LSP server starting
      // Use unified lsp:server-starting
      // LSP packages loaded
      // Use unified lsp:initialized or installation-complete
      // LSP installation complete
      // Use unified lsp:installation-complete
    } catch (err) {
      await logError('App.vue: Failed to set up additional LSP event listeners', err);
    }

    // Set up unified event listeners for project switching events
    try {
      // Unified orchestrator:project-change-status
      await unifiedEventService.addEventListener(
        EventCategory.Orchestrator,
        'project-change-status',
        async (event) => {
          const payload = event.payload as { message?: string; progress_percentage?: number };
          if (payload?.message && payload?.progress_percentage !== undefined) {
            // Don't show the modal during initial startup - only show for manual project switches
            // Use isInitialStartupPhase flag to track if we're still in the initial startup phase
            // This prevents the modal from showing even after startupReady becomes true
            if (isInitialStartupPhase.value) {
              debug(
                `App.vue: Skipping project switching modal during initial startup phase - ${payload.message} (${payload.progress_percentage}%)`
              );
              return;
            }

            // Only set to true if not already switching or if progress is low (indicating start)
            if (!isProjectSwitching.value || payload.progress_percentage <= 10) {
              isProjectSwitching.value = true;

              // Add a fallback timeout to hide the modal if project-change-complete doesn't work
              setTimeout(() => {
                if (isProjectSwitching.value) {
                  isProjectSwitching.value = false;
                }
              }, 10000); // 10 second timeout
            }
          }
        }
      );
    } catch (err) {
      await logError('App.vue: Failed to set up project switching event listeners', err);
    }

    // Set up system error event listener
    try {
      await unifiedEventService.addEventListener(EventCategory.System, 'error', async (event) => {
        const payload = event.payload;
        if (payload.error) {
          debug('App.vue: Received system error event:', payload.error);
          showErrorScreen.value = true;
        }
      });
    } catch (err) {
      await logError('App.vue: Failed to set up system error event listener', err);
    }
  } catch (err) {
    await logError('App.vue: Error during onMounted initialization', err);
    finalSetupErrorMessage.value = `Initialization error: ${err instanceof Error ? err.message : String(err)}`;
    startupReady.value = false; // Hide startup screen to show error
  }
});

// Set up event listeners for backend initialization
const setupBackendInitializationListeners = async () => {
  // Legacy project-change-status event listener removed - now handled by unified event system

  // Legacy project-change-complete event listener removed to prevent unnecessary project path updates
  // Project path is now managed by the unified event system

  // Listen for new project activation to show startup modal
  window.addEventListener('project-activation-started', (() => {
    // debug('App.vue: Received project-activation-started event, handling project activation');
    handleProjectActivation();
    // Note: Removed circular event dispatch to prevent infinite recursion
  }) as EventListener);

  // Listen for backend busy state events using Tauri's event system
  unlistenBackendBusyFn = await listen('backend-busy', (event) => {
    const payload = event.payload;
    appStore.setBackendBusy(true);
  });

  unlistenBackendDoneFn = await listen('backend-done', (event) => {
    const payload = event.payload;
    appStore.setBackendBusy(false);
  });

  // Set up periodic sync of backend busy status (every 10 seconds to avoid log spam)
  // DISABLED: This interferes with event-based busy state management
  // const busyStatusInterval = setInterval(async () => {
  //   try {
  //     await appStore.syncBackendBusyStatus();
  //   } catch (error) {
  //     debug(`App.vue: Failed to sync backend busy status: ${error}`);
  //   }
  // }, 10000);

  // Sync busy status when window becomes visible (user switches back to app)
  // DISABLED: This might interfere with event-based busy state management
  // const handleVisibilityChange = async () => {
  //   if (!document.hidden) {
  //     debug('App.vue: Window became visible, syncing backend busy status');
  //     try {
  //       await appStore.forceSyncBackendBusyStatus();
  //     } catch (error) {
  //       debug(`App.vue: Failed to force sync backend busy status: ${error}`);
  //     }
  //   }
  // };

  // DISABLED: Visibility change sync is disabled
  // document.addEventListener('visibilitychange', handleVisibilityChange);

  // Store interval ID and handler for cleanup
  // DISABLED: Periodic sync is disabled
  // (window as any).busyStatusInterval = busyStatusInterval;
  // (window as any).handleVisibilityChange = handleVisibilityChange;

  // Backend setup completion now handled by StartupModal
  // window.addEventListener('setup-complete', (() => {
  //   debug('App.vue: Received setup-complete event from backend');
  //   handleBackendSetupComplete();
  // }) as EventListener);
};

onUnmounted(async () => {
  if (eventUnlistenFn) {
    eventUnlistenFn();
    eventUnlistenFn = null;
  }
  // Removed daemon unlisten cleanup

  // Clean up backend busy state event listeners
  if (unlistenBackendBusyFn) {
    unlistenBackendBusyFn();
    unlistenBackendBusyFn = null;
  }

  if (unlistenBackendDoneFn) {
    unlistenBackendDoneFn();
    unlistenBackendDoneFn = null;
  }

  // Clean up busy status sync interval
  // DISABLED: Periodic sync is disabled
  // if ((window as any).busyStatusInterval) {
  //   clearInterval((window as any).busyStatusInterval);
  //   (window as any).busyStatusInterval = null;
  // }

  // Clean up visibility change listener
  // DISABLED: Visibility change sync is disabled
  // if ((window as any).handleVisibilityChange) {
  //   document.removeEventListener('visibilitychange', (window as any).handleVisibilityChange);
  //   (window as any).handleVisibilityChange = null;
  // }

  // Clean up unified event listeners
  try {
    await unifiedEventService.removeAllListeners();
  } catch (error) {
    await logError('App.vue: Failed to cleanup unified event listeners', error);
  }

  // Clean up application service
  try {
    await applicationService.cleanup();
  } catch (error) {
    await logError('App.vue: Failed to cleanup application service', error);
  }
});

// Expose test function globally for manual testing
(window as any).testJuliaRepl = async (code = '2 + 2') => {
  try {
    const result = await invoke('execute_julia_code', {
      code,
      command_type: 'test',
    });
    return result;
  } catch (error) {
    await logError('Julia REPL test failed', error);
    throw error;
  }
};
</script>

<template>
  <n-config-provider :theme="darkTheme" :theme-overrides="themeOverrides">
    <n-message-provider>
      <n-global-style />
      <div>
        <!-- Error Screen - shown when system error occurs -->
        <ErrorScreen v-if="showErrorScreen" />

        <template v-if="!showErrorScreen">
          <!-- Welcome Modal for new users - shown first -->
          <WelcomeModal
            v-if="showWelcomeModal"
            v-model:show="showWelcomeModal"
            :initial-mode="welcomeModalMode"
            :user-email="welcomeModalUserEmail"
            :auth-error="welcomeModalAuthError"
            @mode-change="handleWelcomeModalModeChange"
          />

          <!-- Startup Modal - shown after authentication is complete but startup is not ready -->
          <StartupModal
            v-else-if="authenticationComplete && !startupReady"
            @startup-complete="handleStartupComplete"
          />

          <ProjectSwitchModal
            v-if="isProjectSwitching"
            @project-switch-complete="() => (isProjectSwitching = false)"
          />

          <!-- Backend initialization progress -->
          <div v-else-if="finalSetupErrorMessage" class="setup-error-container">
            <h1>Setup Error</h1>
            <p>{{ finalSetupErrorMessage }}</p>
            <p>
              Please check console logs for more details and ensure Julia and the Julia Language
              Server are correctly installed and configured.
            </p>
          </div>

          <!-- Router View - shown when authentication and startup are complete and no setup is needed -->
          <router-view
            v-else-if="
              authenticationComplete && startupReady && !finalSetupErrorMessage && !showWelcomeModal
            "
          />

          <!-- Fallback Loading Spinner - shown when nothing else is showing -->
          <div v-else class="loading-container">
            <n-spin size="large" class="loading-spin" />
            <p class="loading-text">Loading</p>
          </div>
        </template>

        <!-- Notification Toast -->
        <NotificationToast
          v-if="notification.show"
          :title="notification.title"
          :message="notification.message"
          :type="notification.type"
          :show="notification.show"
          @close="notification.show = false"
        />
      </div>
    </n-message-provider>
  </n-config-provider>
</template>

<style>
/* Global styles if needed, or move them to MainLayout or keep here */
html,
body,
#app {
  height: 100%;
  margin: 0;
  overflow: hidden; /* Prevent body scrollbars */
  background-color: #1e1e1e; /* Match editor background or a neutral one */
}

.splitpanes__pane {
  background-color: #2a2a2a !important; /* Your app's dark background color for panes */
}

/* Styles for splitpanes can remain here if App.vue is the one importing its CSS */
/* Or move to MainLayout.vue if it imports splitpanes.css directly */
.splitpanes.default-theme .splitpanes__splitter {
  background-color: #222222; /* Darker grey for splitter */
  border-left: 1px solid #3c3c3c; /* Grey borders */
  border-right: 1px solid #3c3c3c;
  width: 3px; /* Keep existing width or adjust */
}

.splitpanes.default-theme .splitpanes__splitter:hover {
  border-color: #555555; /* Lighter grey on hover */
}

/* If you use horizontal splitters, style them too */
.splitpanes.default-theme.splitpanes--horizontal > .splitpanes__splitter {
  background-color: #222222;
  border-top: 1px solid #3c3c3c;
  border-bottom: 1px solid #3c3c3c;
  /* Resetting these as horizontal splitters don't usually have left/right borders */
  border-left: none;
  border-right: none;
  width: auto; /* Let height control the clickable area for horizontal */
  height: 5px !important; /* Adjust height if necessary for horizontal splitters */
}

.app-container {
  height: 100%;
  display: flex;
  flex-direction: column;
  box-sizing: border-box; /* Ensures padding is included in the 100% height */
}

.placeholder-content {
  flex-grow: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  text-align: center;
}

/* Example of how you might hide the main app scrollbar during init screen if necessary */
/* body.initializing {
  overflow: hidden;
} */

/* Remove the default Tauri drag region if you don't need it or handle it specifically */
/*
[data-tauri-drag-region] {
  display: none; 
}
*/

.setup-error-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100vh;
  padding: 20px;
  text-align: center;
  background-color: #2a2a2a;
}

.setup-error-container h1 {
  color: #e57373; /* Error red */
  margin-bottom: 16px;
}

.setup-error-container p {
  font-size: 1.1em;
  line-height: 1.6;
  max-width: 600px;
}

/* Loading spinner styles */
.loading-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100vh;
  background-color: #1e1e1e;
}

.loading-spin {
  color: #389826;
  margin-bottom: 16px;
}

.loading-text {
  color: #cccccc;
  font-size: 16px;
  font-weight: 500;
  margin: 0;
}

/* No CSS overrides needed - theme should handle everything automatically */
</style>
