<template>
  <div class="modal-overlay">
    <div class="modal-container compact">
      <!-- Header with Logo -->
      <div class="header-content">
        <div class="logo-container">
          <img src="/icon.png" alt="Compute42" class="logo" />
        </div>
        <h1 class="app-title">Compute42</h1>
        <p class="app-subtitle">Julia Development Environment</p>
        <p class="version-info">v{{ appVersion }}</p>
        <div class="beta-warning">
          <p>⚠️ Beta Software - Use at your own risk</p>
        </div>
        <div v-if="isInitialStartup" class="initial-startup-info">
          <p>
            ⏱️ First startup will take a long time due to Julia installation and package
            precompilation. Please be patient.
          </p>
        </div>
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
          <!-- <div class="progress-text">{{ progressText || 'Initializing...' }}</div> -->
          <div class="progress-fill" :style="{ width: progressPercentage + '%' }"></div>
        </div>
      </div>

      <!-- Terminal Output Section -->
      <div class="terminal-section">
        <div class="terminal-header">
          <span class="terminal-title">
            <n-icon size="16" class="terminal-icon">
              <TerminalOutline />
            </n-icon>
            Detailed Information
          </span>
        </div>
        <div class="terminal-window">
          <div class="terminal-content" ref="terminalContent">
            <div
              v-for="(line, index) in terminalLines"
              :key="index"
              class="terminal-line"
              :class="line.type"
            >
              <span class="terminal-timestamp">{{ line.timestamp }}</span>
              <span class="terminal-source">{{ line.source }}</span>
              <span class="terminal-text">{{ line.text }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Error Message -->
      <div v-if="errorMessage" class="error-message compact">
        <p>{{ errorMessage }}</p>
        <button @click="retrySetup" class="retry-button">Retry</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed, nextTick } from 'vue';
import { NIcon, NSpin } from 'naive-ui';
import { CloseOutline, CheckmarkOutline, TerminalOutline } from '@vicons/ionicons5';
import { invoke } from '@tauri-apps/api/core';
import { debug, error } from '../../utils/logger';
import { unifiedEventService, EventCategory } from '../../services/unifiedEventService';

const emit = defineEmits<{
  startupComplete: [];
}>();

// Startup state
const mainStatusMessage = ref('Initializing Compute42...');
const isError = ref(false);
const isComplete = ref(false);
const errorMessage = ref('');

// Progress bar state
const progressPercentage = ref(0);
const progressText = ref('Initializing...');

// Helper function to update progress - ensures it only increases
const updateProgress = (newPercentage: number) => {
  if (newPercentage > progressPercentage.value) {
    progressPercentage.value = newPercentage;
  }
};

// App version
const appVersion = ref('0.0.0');

// Track if this is the initial startup (Julia not installed)
const isInitialStartup = ref(false);

// Track startup completion states
const orchestratorReady = ref(false);
const lspReady = ref(false);
const lspBusy = ref(false); // true during install/precompile until fully ready

// Track all unlisten functions for cleanup
let unlistenFunctions: (() => void)[] = [];

// Terminal state
const terminalLines = ref<
  Array<{
    timestamp: string;
    source: string;
    text: string;
    type: 'julia' | 'lsp' | 'info';
  }>
>([]);
const terminalContent = ref<HTMLElement | null>(null);

// statusClass for the step icon
const statusClass = computed(() => {
  if (errorMessage.value) return 'error';
  if (isComplete.value) return 'success';
  return 'loading';
});

const retrySetup = () => {
  // Reset state and restart setup
  errorMessage.value = '';
    mainStatusMessage.value = 'Initializing Compute42...';
  progressPercentage.value = 0;
  progressText.value = 'Initializing...';
  isComplete.value = false;
  isError.value = false;
  orchestratorReady.value = false;
  lspReady.value = false;
  terminalLines.value = [];
  // Trigger setup restart
  window.location.reload();
};

const addTerminalLine = (source: string, text: string, type: 'julia' | 'lsp' | 'info' = 'info') => {
  const timestamp = new Date().toLocaleTimeString();
  terminalLines.value.push({
    timestamp,
    source,
    text,
    type,
  });

  // Keep only last 50 lines to prevent memory issues and keep terminal compact
  if (terminalLines.value.length > 50) {
    terminalLines.value = terminalLines.value.slice(-50);
  }

  // Auto-scroll to bottom
  nextTick(() => {
    scrollToBottom();
  });
};

const scrollToBottom = () => {
  if (terminalContent.value) {
    terminalContent.value.scrollTop = terminalContent.value.scrollHeight;
  }
};

// Check if both orchestrator and LSP are ready, then complete startup
// Note: Backend orchestrator now handles the logic of determining when startup is complete
// based on whether it's a Julia project. We just wait for the startup-ready event.
const checkAndCompleteStartup = () => {
  // Backend now handles whether LSP is needed, so we just wait for orchestrator-ready
  // which will only be emitted when appropriate (after LSP ready for Julia projects, immediately for non-Julia projects)
  if (orchestratorReady.value && !isComplete.value) {
    debug(
      'StartupModal: Orchestrator is ready (backend determined completion), completing startup'
    );

    isComplete.value = true;
    mainStatusMessage.value = 'Compute42 is ready!';
    progressText.value = 'Compute42 is ready!';
    progressPercentage.value = 100;

    // Log what the user sees
    debug(`StartupModal: DISPLAYING MESSAGE: "Compute42 is ready!" (100%)`);

    // Complete startup
    debug('StartupModal: Emitting startupComplete event - modal will close');
    emit('startupComplete');
  }
};

const fetchAppVersion = async () => {
  try {
    const version = await invoke('get_app_version');
    appVersion.value = version as string;
    debug(`StartupModal: App version: ${appVersion.value}`);
  } catch (err) {
    error(
      `StartupModal: Failed to fetch app version: ${err instanceof Error ? err.message : String(err)}`
    );
    // Keep default version if fetch fails
  }
};

const checkJuliaInstallation = async () => {
  try {
    const version = await invoke('get_julia_version');
    // If Julia is not installed, version will be "Not installed"
    isInitialStartup.value = version === 'Not installed';
    debug(
      `StartupModal: Julia installation status - version: ${version}, isInitialStartup: ${isInitialStartup.value}`
    );
  } catch (err) {
    error(
      `StartupModal: Failed to check Julia installation: ${err instanceof Error ? err.message : String(err)}`
    );
    // Default to false if check fails
    isInitialStartup.value = false;
  }
};

const setupEventListeners = async () => {
  try {
    // Set up unified event listeners for orchestrator events
    await unifiedEventService.addEventListener(
      EventCategory.Orchestrator,
      'startup-update',
      async (event) => {
        const payload = event.payload;
        if (payload.message && payload.progress !== undefined) {
          debug('StartupModal: Received orchestrator:startup-update event.');

          // Update status message and progress (progress only increases)
          mainStatusMessage.value = payload.message;
          progressText.value = payload.message;
          updateProgress(payload.progress);

          // Log what the user sees (use actual progress value after update)
          debug(
            `StartupModal: DISPLAYING MESSAGE: "${payload.message}" (${progressPercentage.value}%)`
          );
        }
      }
    );

    await unifiedEventService.addEventListener(
      EventCategory.Orchestrator,
      'startup-ready',
      async (event) => {
        const payload = event.payload;
        if (payload.message) {
          debug('StartupModal: Received orchestrator:startup-ready event.');

          // Backend now handles whether LSP is needed, so when we receive startup-ready,
          // it means the backend has determined startup is complete (either immediately for
          // non-Julia projects, or after LSP ready for Julia projects)
          orchestratorReady.value = true;

          // Complete startup immediately - backend has already done the coordination
          checkAndCompleteStartup();
        }
      }
    );

    // Listen for LSP installation events to provide detailed progress updates
    await unifiedEventService.addEventListener(
      EventCategory.Lsp,
      'installation-progress',
      async (event) => {
        const payload = event.payload;
        if (payload.message && payload.progress !== undefined) {
          debug(
            `StartupModal: Received LSP installation progress - ${payload.message} (${payload.progress}%)`
          );

          // Add context to installation messages
          let message = payload.message;
          if (message.includes('Creating LSP environment')) {
            message = 'Setting up language server...';
          } else if (message.includes('Installing LanguageServer')) {
            message = 'Installing language server...';
          } else if (message.includes('Installing SymbolServer')) {
            message = 'Installing language server...';
          }
          mainStatusMessage.value = message;
          progressText.value = message;
          // Use backend percentages directly
          updateProgress(payload.progress);
          lspBusy.value = true;

          // Always add to terminal output for better visibility
          addTerminalLine('LSP', message, 'lsp');

          // Log what the user sees
          debug(`StartupModal: DISPLAYING MESSAGE: "${message}" (${progressPercentage.value}%)`);
          debug(`StartupModal: Added LSP installation progress to terminal: "${message}"`);
        }
      }
    );

    // Listen for LSP installation started event
    await unifiedEventService.addEventListener(
      EventCategory.Lsp,
      'installation-started',
      async () => {
        debug('StartupModal: Received LSP installation started event');
        const message = 'Beginning Language Server package installation...';
        mainStatusMessage.value = message;
        progressText.value = message;
        updateProgress(85);
        lspBusy.value = true;

        // Add to terminal output
        addTerminalLine('LSP', message, 'lsp');

        debug(`StartupModal: DISPLAYING MESSAGE: "${message}" (${progressPercentage.value}%)`);
        debug(`StartupModal: Added LSP installation started to terminal`);
      }
    );

    await unifiedEventService.addEventListener(
      EventCategory.Lsp,
      'installation-complete',
      async () => {
        debug('StartupModal: Received LSP installation complete event');
        const message = 'Language Server packages installed';
        mainStatusMessage.value = message;
        progressText.value = message;
        updateProgress(92);
        lspBusy.value = true; // may still precompile/load

        // Add to terminal output
        addTerminalLine('LSP', message, 'lsp');

        // Log what the user sees
        debug(`StartupModal: DISPLAYING MESSAGE: "${message}" (${progressPercentage.value}%)`);
      }
    );

    // Listen for LSP installation real-time output
    await unifiedEventService.addEventListener(
      EventCategory.Lsp,
      'installation-output',
      async (event) => {
        debug('StartupModal: Received lsp:installation-output event:', event.payload);
        const payload = event.payload;
        if (payload.content && payload.stream_type) {
          const content = payload.content.trim();
          const streamType = payload.stream_type;

          debug(`StartupModal: Processing LSP installation output [${streamType}]: "${content}"`);

          // Add to terminal output
          if (content && content.length > 0) {
            addTerminalLine('LSP', content, 'lsp');

            // Log the output for debugging
            debug(`StartupModal: Added LSP installation output to terminal: "${content}"`);
          }
        }
      }
    );

    // Listen for LSP server starting event
    await unifiedEventService.addEventListener(EventCategory.Lsp, 'server-starting', async () => {
      debug('StartupModal: Received lsp:server-starting event');
      const message = 'Starting Language Server Protocol server...';
      mainStatusMessage.value = message;
      progressText.value = message;
      updateProgress(96);

      // Add to terminal output
      addTerminalLine('LSP', message, 'lsp');

      debug(`StartupModal: DISPLAYING MESSAGE: "${message}" (${progressPercentage.value}%)`);
      debug(`StartupModal: Added LSP server starting to terminal`);
    });

    // Listen for Julia detailed output events (includes filtered messages for StartupModal)
    await unifiedEventService.addEventListener(
      EventCategory.Julia,
      'output-detailed',
      async (event) => {
        const payload = event.payload;

        if (Array.isArray(payload) && payload.length > 0) {
          const output = payload[0];

          if (output.content) {
            const line = output.content.trim();

            // Add to terminal output (detailed output includes all messages, even filtered ones)
            if (line && line.length > 0) {
              addTerminalLine('Julia', line, 'julia');
            }
          }
        }
      }
    );

    // Listen for Julia installation progress events
    await unifiedEventService.addEventListener(
      EventCategory.Julia,
      'installation-progress',
      async (event) => {
        const payload = event.payload;

        if (payload.message) {
          const message = payload.message;
          const progress = payload.progress || 0;

          // Only log every 10% progress to reduce verbosity in debug logs
          if (progress % 10 == 0) {
            debug(`StartupModal: Julia installation progress: "${message}" (${progress}%)`);
          }

          // Always add to terminal output so users see download/extraction progress
          // Download progress updates occur every 5%, so they will be visible in Detailed Information
          addTerminalLine('Julia', message, 'julia');
        }
      }
    );

    // Listen for Julia installation started events
    await unifiedEventService.addEventListener(
      EventCategory.Julia,
      'installation-started',
      async (event) => {
        const payload = event.payload;

        if (payload.message) {
          const message = payload.message;

          debug(`StartupModal: Julia installation started: "${message}"`);

          // Add to terminal output
          addTerminalLine('Julia', message, 'julia');
        }
      }
    );

    // Listen for orchestrator startup errors
    await unifiedEventService.addEventListener(
      EventCategory.Orchestrator,
      'startup-error',
      async (event) => {
        const payload = event.payload;
        error('StartupModal: Received orchestrator:startup-error event:', payload);

        if (payload.message && payload.error_details) {
          // Set main error message
          errorMessage.value = payload.message;
          isError.value = true;
          mainStatusMessage.value = 'Error: ' + payload.message;

          // Add detailed error to terminal
          addTerminalLine('ERROR', payload.error_details, 'info');

          // Update progress
          if (payload.progress !== undefined) {
            updateProgress(payload.progress);
          }

          error(`StartupModal: ERROR - ${payload.message}: ${payload.error_details}`);
        }
      }
    );

    // Listen for Julia installation errors
    await unifiedEventService.addEventListener(
      EventCategory.Julia,
      'installation-error',
      async (event) => {
        const payload = event.payload;
        error('StartupModal: Received julia:installation-error event:', payload);

        if (payload.message && payload.error) {
          errorMessage.value = payload.message;
          isError.value = true;
          mainStatusMessage.value = 'Error: ' + payload.message;
          addTerminalLine('Julia', payload.error, 'julia');
          error(`StartupModal: Julia installation error - ${payload.message}: ${payload.error}`);
        }
      }
    );

    // Listen for LSP installation errors
    await unifiedEventService.addEventListener(
      EventCategory.Lsp,
      'installation-error',
      async (event) => {
        const payload = event.payload;
        error('StartupModal: Received lsp:installation-error event:', payload);

        if (payload.message && payload.error) {
          errorMessage.value = payload.message;
          isError.value = true;
          mainStatusMessage.value = 'Error: ' + payload.message;
          addTerminalLine('LSP', payload.error, 'lsp');
          error(`StartupModal: LSP installation error - ${payload.message}: ${payload.error}`);
        }
      }
    );

    // Listen for Julia installation completed events
    await unifiedEventService.addEventListener(
      EventCategory.Julia,
      'installation-completed',
      async (event) => {
        debug('StartupModal: Received julia:installation-completed event:', event.payload);
        const payload = event.payload;

        if (payload.installation) {
          const version = payload.installation.version || 'unknown';

          debug(`StartupModal: Julia installation completed: version ${version}`);

          // Add to terminal output
          addTerminalLine('Julia', `Julia ${version} installation completed successfully`, 'julia');

          // Log what the user sees
          debug(`StartupModal: Added Julia installation completed to terminal: version ${version}`);
        }
      }
    );

    // Listen for LSP status events (including precompilation progress)
    await unifiedEventService.addEventListener(EventCategory.Lsp, 'status', async (event) => {
      debug('StartupModal: Received lsp:status event:', event.payload);
      const payload = event.payload;

      if (payload.message) {
        const message = payload.message;
        const status = payload.status;

        debug(`StartupModal: LSP status [${status}]: "${message}"`);

        // Update progress based on status
        if (status === 'precompiling') {
          mainStatusMessage.value = 'Precompiling Julia packages...';
          progressText.value = 'Precompiling Julia packages...';
          updateProgress(80);
          lspBusy.value = true;
        } else if (status === 'server-ready') {
          mainStatusMessage.value = 'Starting up...';
          progressText.value = 'Starting up...';
          updateProgress(85);
          lspBusy.value = true;
        } else if (status === 'symbols-ready') {
          mainStatusMessage.value = 'Starting up...';
          progressText.value = 'Starting up...';
          updateProgress(90);
          lspBusy.value = true;
        }

        // Add to terminal output
        addTerminalLine('LSP', message, 'lsp');

        // Log what the user sees
        debug(
          `StartupModal: DISPLAYING MESSAGE: "${mainStatusMessage.value}" (${progressPercentage.value}%)`
        );
        debug(`StartupModal: Added LSP status to terminal: "${message}"`);
      }
    });

    // Listen for LSP ready events (when precompilation is complete)
    await unifiedEventService.addEventListener(EventCategory.Lsp, 'ready', async (event) => {
      debug('StartupModal: Received lsp:ready event:', event.payload);
      const payload = event.payload;

      if (payload.message) {
        const message = payload.message;

        debug(`StartupModal: LSP is ready: "${message}"`);

        // Update progress to show LSP is fully ready
        mainStatusMessage.value = 'Starting up...';
        progressText.value = 'Starting up...';
        updateProgress(95);

        // Add to terminal output
        addTerminalLine('LSP', message, 'lsp');

        // Log what the user sees
        debug(
          `StartupModal: DISPLAYING MESSAGE: "Language server ready" (${progressPercentage.value}%)`
        );
        debug(`StartupModal: Added LSP ready to terminal: "${message}"`);
      }
    });

    // Listen for LSP detailed output events (includes filtered messages for StartupModal)
    await unifiedEventService.addEventListener(
      EventCategory.Lsp,
      'output-detailed',
      async (event) => {
        debug('StartupModal: Received lsp:output-detailed event:', event.payload);
        const payload = event.payload;

        if (Array.isArray(payload) && payload.length > 0) {
          const output = payload[0];

          if (output.content) {
            const line = output.content.trim();
            debug(`StartupModal: Processing LSP detailed output line: "${line}"`);

            // Add to terminal output (detailed output includes all messages, even filtered ones)
            if (line && line.length > 0) {
              addTerminalLine('LSP', line, 'lsp');

              // Log what the user sees
              debug(`StartupModal: Added LSP detailed output to terminal: "${line}"`);
            }
          }
        }
      }
    );

    // Listen for LSP output events to show LSP stderr output (fallback)
    await unifiedEventService.addEventListener(EventCategory.Lsp, 'output', async (event) => {
      debug('StartupModal: Received lsp:output event:', event.payload);
      const payload = event.payload;

      if (Array.isArray(payload) && payload.length > 0) {
        const output = payload[0];

        if (output.stream_type === 'Stderr' && output.content) {
          const line = output.content.trim();
          debug(`StartupModal: Processing LSP stderr line: "${line}"`);

          // Add to terminal output
          if (line && line.length > 0) {
            addTerminalLine('LSP', line, 'lsp');

            // Log what the user sees
            debug(`StartupModal: Added LSP stderr to terminal: "${line}"`);
          }
        }
      }
    });

    // Listen for LSP status events to track when LSP becomes ready
    await unifiedEventService.addEventListener(EventCategory.Lsp, 'status', async (event) => {
      const payload = event.payload;
      if (payload.status && payload.message) {
        debug(`StartupModal: Received LSP status event - ${payload.status}: ${payload.message}`);

        // Update status message and progress based on LSP status
        let terminalMessage = '';
        switch (payload.status) {
          case 'starting':
            mainStatusMessage.value = 'Starting up...';
            progressText.value = 'Starting up...';
            updateProgress(78);
            terminalMessage = 'Starting language server...';
            debug(
              `StartupModal: DISPLAYING MESSAGE: "Starting up..." (${progressPercentage.value}%)`
            );
            break;

          case 'started':
            mainStatusMessage.value = 'Starting up...';
            progressText.value = 'Starting up...';
            updateProgress(80);
            terminalMessage = 'Language Server Protocol server process started successfully';
            debug(
              `StartupModal: DISPLAYING MESSAGE: "Starting up..." (${progressPercentage.value}%)`
            );
            break;

          case 'initialized':
            mainStatusMessage.value = 'Starting up...';
            progressText.value = 'Starting up...';
            updateProgress(82);
            terminalMessage = 'Language server initialized, loading packages...';
            debug(
              `StartupModal: DISPLAYING MESSAGE: "Starting up..." (${progressPercentage.value}%)`
            );
            break;

          case 'loading-cache':
            mainStatusMessage.value = 'Starting up...';
            progressText.value = 'Starting up...';
            updateProgress(85);
            terminalMessage = 'Loading package cache and symbols...';
            debug(
              `StartupModal: DISPLAYING MESSAGE: "Starting up..." (${progressPercentage.value}%)`
            );
            break;

          case 'ready':
            // This is the premature ready from lsp_service.rs - don't set lspReady yet
            mainStatusMessage.value = 'Starting up...';
            progressText.value = 'Starting up...';
            updateProgress(95);
            terminalMessage = 'Language server ready';
            // Keep busy until we get the true ready signal
            debug(
              `StartupModal: DISPLAYING MESSAGE: "Starting up..." (${progressPercentage.value}%)`
            );
            break;

          case 'failed':
            // Show error but don't fail the entire startup
            mainStatusMessage.value = `Language Server error: ${payload.message}`;
            progressText.value = `Language Server error: ${payload.message}`;
            terminalMessage = `Language Server error: ${payload.message}`;
            debug(`StartupModal: DISPLAYING MESSAGE: "Language Server error: ${payload.message}"`);
            // Still allow startup to complete after a delay
            setTimeout(() => {
              lspReady.value = true;
              checkAndCompleteStartup();
            }, 3000);
            break;

          default:
            // For any other status, show the message as-is
            mainStatusMessage.value = `Language Server: ${payload.message}`;
            progressText.value = `Language Server: ${payload.message}`;
            updateProgress(95);
            terminalMessage = `Language Server: ${payload.message}`;
            debug(
              `StartupModal: DISPLAYING MESSAGE: "Language Server: ${payload.message}" (${progressPercentage.value}%)`
            );
            break;
        }

        // Add terminal line for all status transitions
        if (terminalMessage) {
          addTerminalLine('LSP', terminalMessage, 'lsp');
          debug(`StartupModal: Added LSP status to terminal: "${terminalMessage}"`);
        }
      }
    });

    // Listen for LSP stderr events to show detailed progress information
    await unifiedEventService.addEventListener(EventCategory.Lsp, 'stderr', async (event) => {
      const payload = event.payload;
      if (payload.message) {
        debug(`StartupModal: Received LSP stderr event - ${payload.message}`);

        // Add to terminal output
        addTerminalLine('LSP', payload.message, 'lsp');

        // Show informative stderr messages to provide better user feedback
        let displayMessage = payload.message;

        // Clean up and format the message for better readability
        if (displayMessage.includes('[ Info:')) {
          // Extract the info message content
          const match = displayMessage.match(/\[ Info: (.+)/);
          if (match) {
            displayMessage = match[1];
          }
        }

        // Skip messages with package UUIDs
        if (/[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}/.test(displayMessage)) {
          return;
        }

        // Skip "Package X is cached" messages
        if (displayMessage.includes('is cached')) {
          return;
        }

        // Log the stderr message for debugging (don't update main status)
        debug(`StartupModal: Added LSP stderr to terminal: "${displayMessage}"`);
      }
    });

    // Listen for LSP error events to show detailed error information
    await unifiedEventService.addEventListener(EventCategory.Lsp, 'error', async (event) => {
      const payload = event.payload;
      if (payload.message || payload.stderr_line) {
        const errorMsg = payload.message || payload.stderr_line;
        debug(`StartupModal: Received LSP error event - ${errorMsg}`);

        // Add to terminal output
        addTerminalLine('LSP', errorMsg, 'lsp');

        // Show the error but don't fail the startup
        mainStatusMessage.value = `Language Server: ${errorMsg}`;
        progressText.value = `Language Server: ${errorMsg}`;
        debug(`StartupModal: DISPLAYING MESSAGE: "Language Server: ${errorMsg}"`);
      }
    });

    // Listen for the true LSP ready event (when packages are fully loaded)
    await unifiedEventService.addEventListener(EventCategory.Lsp, 'ready', async (event) => {
      debug('StartupModal: Received LSP ready event - packages fully loaded', event.payload);
      lspReady.value = true;
      mainStatusMessage.value = 'Starting up...';
      progressText.value = 'Starting up...';
      updateProgress(95);
      lspBusy.value = false;

      checkAndCompleteStartup();
    });

    // Note: Legacy event listeners removed - using only unified event system

    debug('StartupModal: All event listeners set up successfully');
  } catch (err) {
    error(
      `StartupModal: Failed to set up event listeners: ${err instanceof Error ? err.message : String(err)}`
    );
    isError.value = true;
    errorMessage.value = `Failed to set up event listeners: ${err instanceof Error ? err.message : String(err)}`;
  }
};

onMounted(async () => {
  debug('StartupModal: Component mounted, setting up event listeners');

  // Fetch app version
  await fetchAppVersion();

  // Check if this is the initial startup (Julia not installed)
  await checkJuliaInstallation();

  // Set up event listeners
  await setupEventListeners();
});

onUnmounted(() => {
  // Clean up all event listeners
  if (unlistenFunctions.length > 0) {
    debug('StartupModal: Cleaning up all event listeners.');
    unlistenFunctions.forEach((unlistenFn) => {
      if (unlistenFn) {
        unlistenFn();
      }
    });
    unlistenFunctions = [];
  }
});
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background-color: rgba(0, 0, 0, 0.7);
  display: flex;
  justify-content: center;
  align-items: center;
  z-index: 1000;
}

.modal-container.compact {
  background: linear-gradient(135deg, #1e1e1e 0%, #2d2d2d 100%);
  color: #e0e0e0;
  padding: 2rem;
  border-radius: 12px;
  max-width: 500px;
  width: 90%;
  min-height: 400px;
  box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5);
  border: 1px solid #333;
  display: flex;
  flex-direction: column;
}

.header-content {
  text-align: center;
  margin-bottom: 1rem;
  flex-shrink: 0;
}

.logo-container {
  margin-bottom: 0.5rem;
}

.logo {
  width: 60px;
  height: 60px;
  object-fit: contain;
}

.app-title {
  font-size: 1.5rem;
  font-weight: 600;
  margin: 0 0 0.25rem 0;
  color: #ffffff;
}

.app-subtitle {
  font-size: 0.9rem;
  color: #b0b0b0;
  margin: 0;
}

.version-info {
  font-size: 0.8rem;
  color: #888;
  margin: 0.25rem 0 0.5rem 0;
  font-weight: 500;
}

.beta-warning {
  background: rgba(255, 193, 7, 0.1);
  border: 1px solid #ffc107;
  border-radius: 6px;
  padding: 0.5rem;
  margin-top: 0.5rem;
}

.beta-warning p {
  margin: 0;
  font-size: 0.8rem;
  color: #ffc107;
  text-align: center;
  font-weight: 500;
}

.initial-startup-info {
  background: rgba(255, 193, 7, 0.1);
  border: 1px solid #ffc107;
  border-radius: 6px;
  padding: 0.75rem;
  margin-top: 0.75rem;
}

.initial-startup-info p {
  margin: 0;
  font-size: 0.85rem;
  color: #ffc107;
  text-align: center;
  font-weight: 500;
  line-height: 1.4;
}

.current-step {
  text-align: center;
  margin-bottom: 1.5rem;
  flex: 1;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
}

.step-icon {
  margin-bottom: 1rem;
  display: flex;
  justify-content: center;
  align-items: center;
}

.loading-spin {
  color: #389826;
}

.success-icon {
  color: #4caf50;
}

.error-icon {
  color: #f44336;
}

.step-message {
  font-size: 1.1rem;
  color: #ffffff;
  text-align: center;
  line-height: 1.4;
}

.progress-section {
  margin-bottom: 1.5rem;
  flex-shrink: 0;
}

.progress-bar {
  width: 100%;
  height: 8px;
  background: #333;
  border-radius: 4px;
  overflow: hidden;
  margin-bottom: 0.5rem;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #389826, #4aa830);
  transition: width 0.3s ease;
  position: relative;
  overflow: hidden;
}

.progress-fill::after {
  content: '';
  position: absolute;
  top: 0;
  left: -100%;
  width: 100%;
  height: 100%;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.3), transparent);
  animation: shimmer 2s infinite;
}

@keyframes shimmer {
  0% {
    left: -100%;
  }
  100% {
    left: 100%;
  }
}

.progress-text {
  font-size: 0.9rem;
  color: #b0b0b0;
  text-align: center;
}

@keyframes spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

.error-message.compact {
  background: rgba(244, 67, 54, 0.1);
  border: 1px solid #f44336;
  border-radius: 8px;
  padding: 0.75rem;
  margin-top: 0.75rem;
  text-align: center;
  flex-shrink: 0;
}

.error-message.compact p {
  margin: 0 0 0.75rem 0;
  color: #f44336;
  font-size: 0.9rem;
}

.retry-button {
  background: #f44336;
  color: white;
  border: none;
  padding: 0.4rem 0.8rem;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.8rem;
}

.retry-button:hover {
  background: #d32f2f;
}

/* Terminal Styles */
.terminal-section {
  margin-top: 1rem;
  border: 1px solid #333;
  border-radius: 8px;
  background: #1a1a1a;
  overflow: hidden;
}

.terminal-header {
  display: flex;
  align-items: center;
  padding: 0.5rem 0.75rem;
  background: #2d2d2d;
  border-bottom: 1px solid #333;
}

.terminal-title {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.8rem;
  font-weight: 500;
  color: #e0e0e0;
}

.terminal-icon {
  color: #4caf50;
}

.terminal-window {
  height: 120px;
  overflow: hidden;
}

.terminal-content {
  height: 120px;
  overflow-y: auto;
  padding: 0.5rem;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 0.75rem;
  line-height: 1.3;
  background: #0d1117;
  color: #e6edf3;
}

.terminal-line {
  display: flex;
  gap: 0.4rem;
  margin-bottom: 0.15rem;
  padding: 0.15rem 0;
  border-radius: 2px;
  transition: background-color 0.1s ease;
}

.terminal-line:hover {
  background: rgba(255, 255, 255, 0.05);
}

.terminal-line.julia {
  border-left: 2px solid #4caf50;
  padding-left: 0.3rem;
}

.terminal-line.lsp {
  border-left: 2px solid #2196f3;
  padding-left: 0.3rem;
}

.terminal-line.info {
  border-left: 2px solid #ff9800;
  padding-left: 0.3rem;
}

.terminal-timestamp {
  color: #888;
  font-size: 0.65rem;
  min-width: 50px;
  flex-shrink: 0;
}

.terminal-source {
  color: #4caf50;
  font-weight: 600;
  min-width: 35px;
  flex-shrink: 0;
  font-size: 0.7rem;
}

.terminal-line.lsp .terminal-source {
  color: #2196f3;
}

.terminal-line.info .terminal-source {
  color: #ff9800;
}

.terminal-text {
  color: #e6edf3;
  flex: 1;
  word-break: break-word;
  font-size: 0.7rem;
}

/* Scrollbar styling for terminal */
.terminal-content::-webkit-scrollbar {
  width: 8px;
}

.terminal-content::-webkit-scrollbar-track {
  background: #1a1a1a;
}

.terminal-content::-webkit-scrollbar-thumb {
  background: #444;
  border-radius: 4px;
}

.terminal-content::-webkit-scrollbar-thumb:hover {
  background: #555;
}
</style>
