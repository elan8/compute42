<template>
  <div class="editor-layout">
    <Splitpanes class="default-theme" :horizontal="true" style="height: 100%; width: 100%">
      <Pane :size="editorPaneSize" min-size="30" style="background-color: #1e1e1e">
        <EditorView />
      </Pane>
      <Pane :size="terminalPaneSize" min-size="15" style="background-color: #252526">
        <div class="terminal-container">
          <TerminalMenuBar />
          <div class="terminal-content">
            <BottomPanel />
          </div>
        </div>
      </Pane>
    </Splitpanes>
  </div>
</template>

<script setup>
import { ref, onMounted, computed } from 'vue';

import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { Splitpanes, Pane } from 'splitpanes';
import 'splitpanes/dist/splitpanes.css'; // Ensure styles are available
import EditorView from './EditorView.vue';
import BottomPanel from './BottomPanel.vue';
import TerminalMenuBar from './TerminalMenuBar.vue';
import { useAppStore } from '../../store/appStore';
import { useTerminalStore } from '../../store/terminalStore';
import { storeToRefs } from 'pinia';
import { primaryColor } from '../../theme';

// Default pane sizes, can be made configurable later if needed
const editorPaneSize = ref(70);
const terminalPaneSize = ref(30);
const appStore = useAppStore();
const terminalStore = useTerminalStore();

// Listen for Julia daemon ready events
onMounted(async () => {
  // Listen for Julia daemon status changes (for fallback/backup)
  await listen('julia:daemon-status-changed', async (event) => {
    const payload = event.payload;
    await debug('EditorLayout: Received julia-daemon-status-changed:', payload);
    if (payload && typeof payload === 'object') {
      const wasReady = isJuliaReplEnabled.value;
      appStore.setJuliaDaemonReady(
        payload.operationalStatus === 'idle' &&
          payload.currentOperationMessage &&
          payload.currentOperationMessage.toLowerCase().includes('ready')
      );
      await debug(
        'EditorLayout: Julia daemon ready state changed from',
        wasReady,
        'to',
        isJuliaReplEnabled.value
      );
    }
  });
});
</script>

<style scoped>
.editor-layout {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.terminal-container {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.terminal-content {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: #1e1e1e;
}

/* Styles for splitpanes within EditorLayout could go here if needed */
/* Ensure default-theme styles for splitpanes are globally available or imported */
</style>
