<template>
  <div class="terminal-menu-bar">
    <div class="menu-bar-content">
      <!-- Clear terminal button -->
      <NTooltip trigger="hover" placement="bottom">
        <template #trigger>
          <NButton
            @click="clearTerminal"
            :disabled="isBusy"
            size="small"
            quaternary
            class="menu-button"
          >
            <template #icon>
              <NIcon>
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path
                    d="M3 6h18M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"
                  />
                </svg>
              </NIcon>
            </template>
          </NButton>
        </template>
        Clear terminal
      </NTooltip>

      <!-- Restart Julia button -->
      <NTooltip trigger="hover" placement="bottom">
        <template #trigger>
          <NButton
            @click="showRestartDialog"
            :disabled="isBusy"
            size="small"
            quaternary
            class="menu-button"
          >
            <template #icon>
              <NIcon>
                <svg
                  width="14"
                  height="14"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                >
                  <path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8" />
                  <path d="M21 3v5h-5" />
                  <path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16" />
                  <path d="M3 21v-5h5" />
                </svg>
              </NIcon>
            </template>
          </NButton>
        </template>
        {{ isBusy ? 'Julia is busy...' : 'Restart Julia' }}
      </NTooltip>

      <!-- Busy indicator next to the buttons -->
      <div class="busy-indicator" v-if="isBusy">
        <div class="spinner"></div>
        <span class="busy-text">Julia is busy...</span>
      </div>
    </div>

    <!-- Confirmation dialog for restart -->
    <NModal
      v-model:show="showRestartConfirmation"
      preset="dialog"
      title="Restart Julia?"
      positive-text="Restart"
      negative-text="Cancel"
      @positive-click="confirmRestart"
      @negative-click="cancelRestart"
    >
      <p>This will restart the Julia process. Any unsaved work in the REPL will be lost.</p>
    </NModal>
  </div>
</template>

<script setup>
import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { NButton, NIcon, NTooltip, NModal } from 'naive-ui';
import { useAppStore } from '../../store/appStore';
import { useTerminalStore } from '../../store/terminalStore';
import { debug, info, error } from '../../utils/logger';

const appStore = useAppStore();
const terminalStore = useTerminalStore();

const showRestartConfirmation = ref(false);

// Computed properties - use centralized busy state as single source of truth
const isBusy = computed(() => {
  return appStore.getBackendBusyStatus();
});

// Methods
const clearTerminal = async () => {
  try {
    debug('TerminalMenuBar: Clearing terminal');
    terminalStore.clearOutputBuffer();
    // Emit event to clear the terminal display
    window.dispatchEvent(new CustomEvent('clear-terminal'));
  } catch (err) {
    error('Failed to clear terminal:', err);
  }
};

const showRestartDialog = () => {
  showRestartConfirmation.value = true;
};

const cancelRestart = () => {
  showRestartConfirmation.value = false;
};

const confirmRestart = async () => {
  try {
    console.log('TerminalMenuBar: Starting Julia restart');
    debug('TerminalMenuBar: Restarting Julia');
    showRestartConfirmation.value = false;

    // Call the restart Julia orchestrator command (more comprehensive)
    // The backend will emit backend-busy/backend-done events to manage button states
    console.log('TerminalMenuBar: Calling restart_julia_orchestrator command');
    await invoke('restart_julia_orchestrator');

    console.log('TerminalMenuBar: Julia restart command completed');
    debug('TerminalMenuBar: Julia restarted successfully');
  } catch (err) {
    console.error('TerminalMenuBar: Failed to restart Julia:', err);
    error('Failed to restart Julia:', err);
  }
};
</script>

<style scoped>
.terminal-menu-bar {
  display: flex;
  justify-content: flex-start;
  align-items: center;
  background-color: #2d2d30;
  border-bottom: 1px solid #3c3c3c;
  padding: 4px 12px;
  height: 28px;
}

.menu-bar-content {
  display: flex;
  align-items: center;
  gap: 8px;
}

.busy-indicator {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #ffa500;
  font-size: 11px;
}

.spinner {
  width: 10px;
  height: 10px;
  border: 1.5px solid #ffa500;
  border-top: 1.5px solid transparent;
  border-radius: 50%;
  animation: spin 1s linear infinite;
}

@keyframes spin {
  0% {
    transform: rotate(0deg);
  }
  100% {
    transform: rotate(360deg);
  }
}

.busy-text {
  font-weight: 500;
}

.menu-button {
  min-width: 24px !important;
  height: 20px !important;
  padding: 0 !important;
}
</style>
