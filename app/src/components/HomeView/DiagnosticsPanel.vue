<template>
  <div class="diagnostics-panel-container">
    <div class="diagnostics-panel">
      <div class="toolbar">
        <n-select
          size="small"
          :options="severityOptions"
          v-model:value="severityFilter"
          style="width: 140px"
        />
        <n-button size="small" tertiary style="margin-left: auto" @click="refresh"
          >Refresh</n-button
        >
      </div>
      <div class="list" v-if="items.length > 0">
        <div class="row" v-for="(d, idx) in items" :key="idx" @dblclick="jumpTo(d)">
          <span class="sev" :class="sevClass(d)">{{ sevLabel(d) }}</span>
          <span class="file" v-if="d.filePath">{{ getFileName(d.filePath) }}</span>
          <span class="pos">{{ d.range.start.line + 1 }}:{{ d.range.start.character + 1 }}</span>
          <span class="src" v-if="d.source">[{{ d.source }}]</span>
          <span class="msg">{{ d.message }}</span>
        </div>
      </div>
      <div v-else class="empty">
        <n-empty :description="loading ? 'Loading diagnostics…' : 'No diagnostics'" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onBeforeUnmount } from 'vue';
import { NSelect, NButton, NSwitch, NEmpty } from 'naive-ui';
import { lspService, type LSPDiagnostic } from '../../services/lspService';
import * as monaco from 'monaco-editor';
import { debug } from '../../utils/logger';

const props = defineProps<{ activeFilePath: string | null }>();
const emit = defineEmits<{ (e: 'count', value: number): void }>();

// Extended diagnostic type that includes file path
interface DiagnosticWithFile extends LSPDiagnostic {
  filePath: string;
  uri: string;
}

const diagnostics = ref<DiagnosticWithFile[]>([]);
const loading = ref(false);
const severityFilter = ref<'all' | 'error' | 'warning' | 'info' | 'hint'>('all');

const severityOptions = [
  { label: 'All severities', value: 'all' },
  { label: 'Errors', value: 'error' },
  { label: 'Warnings', value: 'warning' },
  { label: 'Info', value: 'info' },
  { label: 'Hints', value: 'hint' },
];

const items = ref<DiagnosticWithFile[]>([]);

// Store Monaco marker change listener
let markerChangeListener: monaco.IDisposable | null = null;

// Convert Monaco markers to diagnostics format
function markersToDiagnostics(
  markers: monaco.editor.IMarker[],
  uri: string,
  filePath: string
): DiagnosticWithFile[] {
  return markers.map((marker) => ({
    range: {
      start: { line: marker.startLineNumber - 1, character: marker.startColumn - 1 },
      end: { line: marker.endLineNumber - 1, character: marker.endColumn - 1 },
    },
    severity:
      marker.severity === monaco.MarkerSeverity.Error
        ? 1
        : marker.severity === monaco.MarkerSeverity.Warning
          ? 2
          : marker.severity === monaco.MarkerSeverity.Info
            ? 3
            : 4,
    message: marker.message,
    source: marker.source,
    code: marker.code ? String(marker.code) : undefined,
    filePath,
    uri,
  }));
}

// Collect diagnostics from all Monaco models
function collectDiagnosticsFromMonaco(): DiagnosticWithFile[] {
  const allDiagnostics: DiagnosticWithFile[] = [];

  // Get all Monaco models (representing open files)
  const models = monaco.editor.getModels();

  for (const model of models) {
    const uri = model.uri.toString();
    let filePath: string;

    try {
      // Try to get file path from URI
      if (model.uri.scheme === 'file') {
        filePath = model.uri.fsPath;
      } else {
        // Fallback: try to parse as file URI
        const uriObj = monaco.Uri.parse(uri);
        filePath = uriObj.fsPath || uriObj.path;
      }
    } catch (e) {
      debug(`DiagnosticsPanel: Failed to parse URI ${uri}, skipping`);
      continue;
    }

    // Get all markers for this model (from all sources: 'syntax', 'lsp', etc.)
    const markers = monaco.editor.getModelMarkers({ resource: model.uri });

    if (markers.length > 0) {
      const diags = markersToDiagnostics(markers, uri, filePath);
      allDiagnostics.push(...diags);
    }
  }

  return allDiagnostics;
}

// Update diagnostics from Monaco markers
function updateDiagnosticsFromMonaco() {
  const allDiagnostics = collectDiagnosticsFromMonaco();
  diagnostics.value = allDiagnostics;
  debug(`DiagnosticsPanel: Updated diagnostics from Monaco, found ${allDiagnostics.length} total`);
}

watch([diagnostics, severityFilter], () => {
  const filtered = diagnostics.value.filter((d) => {
    switch (severityFilter.value) {
      case 'error':
        return d.severity === 1;
      case 'warning':
        return d.severity === 2;
      case 'info':
        return d.severity === 3;
      case 'hint':
        return d.severity === 4;
      default:
        return true;
    }
  });
  items.value = filtered;
  emit('count', filtered.length);
});

async function refresh() {
  try {
    loading.value = true;

    // First, update from Monaco markers (which are already set by the editor)
    updateDiagnosticsFromMonaco();

    // Also fetch diagnostics from LSP for all open files
    // Get all Monaco models to find open files
    const models = monaco.editor.getModels();
    const lspDiagnostics: DiagnosticWithFile[] = [];

    for (const model of models) {
      const uri = model.uri.toString();
      let filePath: string;

      try {
        if (model.uri.scheme === 'file') {
          filePath = model.uri.fsPath;
        } else {
          const uriObj = monaco.Uri.parse(uri);
          filePath = uriObj.fsPath || uriObj.path;
        }
      } catch (e) {
        continue;
      }

      // Only fetch for Julia files
      if (model.getLanguageId() === 'julia') {
        try {
          const diags = await lspService.requestDiagnostics(uri);
          if (diags && diags.length > 0) {
            const diagsWithFile = diags.map((d) => ({
              ...d,
              filePath,
              uri,
            }));
            lspDiagnostics.push(...diagsWithFile);
          }
        } catch (err) {
          debug(`DiagnosticsPanel: Failed to fetch LSP diagnostics for ${uri}: ${err}`);
        }
      }
    }

    // Merge Monaco markers and LSP diagnostics
    // Monaco markers take precedence as they're already displayed in the editor
    const monacoDiags = collectDiagnosticsFromMonaco();
    const combined = [...monacoDiags];

    // Add LSP diagnostics that aren't already covered by Monaco markers
    for (const lspDiag of lspDiagnostics) {
      const exists = monacoDiags.some(
        (m) =>
          m.filePath === lspDiag.filePath &&
          m.range.start.line === lspDiag.range.start.line &&
          m.range.start.character === lspDiag.range.start.character &&
          m.message === lspDiag.message
      );
      if (!exists) {
        combined.push(lspDiag);
      }
    }

    diagnostics.value = combined;
  } finally {
    loading.value = false;
  }
}

function sevLabel(d: DiagnosticWithFile) {
  switch (d.severity) {
    case 1:
      return 'E';
    case 2:
      return 'W';
    case 3:
      return 'I';
    case 4:
      return 'H';
    default:
      return '';
  }
}

function sevClass(d: DiagnosticWithFile) {
  return d.severity === 1 ? 'e' : d.severity === 2 ? 'w' : d.severity === 3 ? 'i' : 'h';
}

function getFileName(filePath: string): string {
  // Extract just the filename from the path
  const parts = filePath.split(/[\/\\]/);
  return parts[parts.length - 1] || filePath;
}

function jumpTo(d: DiagnosticWithFile) {
  // Use the file path from the diagnostic, not just the active file
  const filePath = d.filePath;
  if (!filePath) return;

  const range = {
    startLineNumber: d.range.start.line + 1,
    startColumn: d.range.start.character + 1,
    endLineNumber: d.range.end.line + 1,
    endColumn: d.range.end.character + 1,
  };

  // Emit event to open the file and navigate to the location
  window.dispatchEvent(new CustomEvent('open-file-and-navigate', { detail: { filePath, range } }));
}

// Watch for active file changes to update diagnostics
watch(
  () => props.activeFilePath,
  () => {
    // Small delay to allow Monaco markers to be set
    setTimeout(() => {
      updateDiagnosticsFromMonaco();
    }, 100);
  }
);

onMounted(() => {
  // Listen to Monaco marker changes to automatically update diagnostics
  markerChangeListener = monaco.editor.onDidChangeMarkers((uris) => {
    debug(`DiagnosticsPanel: Monaco markers changed for ${uris.length} files`);
    updateDiagnosticsFromMonaco();
  });

  // Initial refresh
  refresh();
});

onBeforeUnmount(() => {
  if (markerChangeListener) {
    markerChangeListener.dispose();
    markerChangeListener = null;
  }
});
</script>

<style scoped>
.diagnostics-panel-container {
  position: relative;
  height: 100%;
  width: 100%;
  overflow: hidden;
  min-height: 0;
}
.diagnostics-panel {
  position: absolute;
  top: 0;
  right: 0;
  bottom: 0;
  left: 0;
  display: flex;
  flex-direction: column;
  min-height: 0;
}
.toolbar {
  display: flex;
  align-items: center;
  padding: 6px 8px;
  border-bottom: 1px solid #333;
}
.list {
  flex: 1;
  overflow: auto;
  padding: 8px;
  min-height: 0;
}
.row {
  display: flex;
  gap: 8px;
  padding: 4px 0;
  cursor: default;
}
.row:hover {
  background: rgba(255, 255, 255, 0.05);
}
.sev {
  width: 18px;
  text-align: center;
  border-radius: 3px;
  color: #000;
  font-weight: 700;
}
.sev.e {
  background: #e51400;
}
.sev.w {
  background: #ffcc00;
}
.sev.i {
  background: #4fc1ff;
}
.sev.h {
  background: #9cdcfe;
}
.file {
  color: #888;
  font-size: 0.9em;
  min-width: 120px;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.pos {
  color: #bbb;
  width: 64px;
}
.src {
  color: #aaa;
}
.msg {
  color: #ddd;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}
.empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
}
</style>
