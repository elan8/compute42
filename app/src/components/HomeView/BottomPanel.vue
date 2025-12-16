<template>
  <div class="bottom-panel">
    <n-tabs v-model:value="activeTab" type="line" animated>
      <n-tab-pane name="terminal" :tab="'Terminal'">
        <div class="pane-body"><TerminalView /></div>
      </n-tab-pane>
      <n-tab-pane name="diagnostics" :tab="diagnosticsTabTitle">
        <div class="pane-body">
          <DiagnosticsPanel :activeFilePath="activeFilePath" @count="onDiagCount" />
        </div>
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { NTabs, NTabPane } from 'naive-ui';
import TerminalView from './TerminalView.vue';
import DiagnosticsPanel from './DiagnosticsPanel.vue';

const activeTab = ref<'terminal' | 'diagnostics'>('terminal');
const activeFilePath = ref<string | null>(null);
const diagCount = ref<number>(0);

const diagnosticsTabTitle = computed(() =>
  diagCount.value > 0 ? `Diagnostics (${diagCount.value})` : 'Diagnostics'
);

function onDiagCount(n: number) {
  diagCount.value = n;
}

// Listen for active file change events from EditorView
window.addEventListener('active-file-changed', (e: any) => {
  if (e?.detail?.filePath) {
    activeFilePath.value = e.detail.filePath;
  }
});
</script>

<style scoped>
.bottom-panel {
  height: 100%;
  display: flex;
  flex-direction: column;
}
/* Keep Naive UI defaults; constrain the pane ourselves */
.pane-body {
  position: relative;
  height: 100%;
  width: 100%;
  flex: 1 1 auto;
  min-height: 0;
}
/* Ensure tab pane content can size to its container */
.bottom-panel :deep(.n-tabs) {
  height: 100%;
  display: flex;
  flex-direction: column;
}
.bottom-panel :deep(.n-tabs .n-tabs-nav) {
  flex: 0 0 auto;
}
.bottom-panel :deep(.n-tabs .n-tabs-content) {
  flex: 1 1 auto;
  display: flex;
  min-height: 0;
}
.bottom-panel :deep(.n-tabs .n-tabs-pane-wrapper) {
  flex: 1 1 auto;
  display: flex;
  min-height: 0;
}
.bottom-panel :deep(.n-tabs .n-tab-pane) {
  flex: 1 1 auto;
  display: flex;
  min-height: 0;
}
</style>
