<template>
  <div class="notebook-viewer" style="height: 100%; display: flex; flex-direction: column">
    <!-- Loading state -->
    <div
      v-if="loading"
      style="flex: 1; display: flex; align-items: center; justify-content: center"
    >
      <n-spin size="large">
        <template #description> Loading notebook... </template>
      </n-spin>
    </div>

    <!-- Error state -->
    <div
      v-else-if="error"
      style="flex: 1; display: flex; align-items: center; justify-content: center"
    >
      <n-empty description="Failed to load notebook">
        <template #extra>
          <n-text depth="3">{{ error }}</n-text>
          <br />
          <n-button size="small" @click="loadNotebook">Retry</n-button>
        </template>
      </n-empty>
    </div>

    <!-- Notebook content -->
    <div
      v-else-if="notebook"
      class="notebook-content"
      style="flex: 1; overflow-y: auto; padding: 16px"
    >
      <div
        v-for="(cell, index) in notebook.cells"
        :key="cell.id || index"
        class="cell-wrapper"
        :class="{ 'cell-hidden': isCellHidden(index) }"
        @mouseenter="hoveredCellIndex = index"
        @mouseleave="hoveredCellIndex = null"
      >
        <!-- Section Header (for markdown cells that start with ##) -->
        <div
          v-if="cell.cell_type === 'markdown' && isSectionHeader(cell)"
          class="section-fold-button"
          @click="toggleSection(index)"
        >
          <n-icon size="16" :class="{ 'icon-collapsed': isSectionCollapsed(index) }">
            <Add />
          </n-icon>
        </div>

        <!-- Code Cell -->
        <NotebookCodeCell
          v-if="cell.cell_type === 'code'"
          :cellId="cell.id || `cell-${index}`"
          :source="cell.source"
          :executionCount="cell.execution_count || null"
          :outputs="cell.outputs || []"
          :notebookPath="filePath"
          @update:source="(value) => updateCellSource(index, value)"
          @update:outputs="(value) => updateCellOutputs(index, value)"
          @update:executionCount="(value) => updateCellExecutionCount(index, value)"
          @execute="executeCell(index)"
          @delete="deleteCell(index)"
          @focus="focusedCellIndex = index"
          @blur="focusedCellIndex = null"
        />

        <!-- Markdown Cell -->
        <NotebookMarkdownCell
          v-else-if="cell.cell_type === 'markdown'"
          :cellId="cell.id || `cell-${index}`"
          :source="cell.source"
          @update:source="(value) => updateCellSource(index, value)"
          @delete="deleteCell(index)"
          @focus="focusedCellIndex = index"
          @blur="focusedCellIndex = null"
        />

        <!-- Raw Cell (display only) -->
        <div v-else class="notebook-raw-cell">
          <div class="cell-header">
            <n-text depth="3" style="font-size: 12px">Raw Cell</n-text>
          </div>
          <div class="cell-content">
            <pre>{{ cell.source }}</pre>
          </div>
        </div>

        <!-- Floating add cell buttons at boundary between cells -->
        <div class="cell-boundary-actions" :class="{ 'show-buttons': hoveredCellIndex === index }">
          <n-button size="small" quaternary @click="addCell('code', index)" class="boundary-button">
            <template #icon>
              <n-icon><CodeOutline /></n-icon>
            </template>
            Add Code Cell
          </n-button>
          <n-button
            size="small"
            quaternary
            @click="addCell('markdown', index)"
            class="boundary-button"
          >
            <template #icon>
              <n-icon><DocumentText /></n-icon>
            </template>
            Add Markdown Cell
          </n-button>
        </div>
      </div>

      <!-- Empty state -->
      <div v-if="notebook.cells.length === 0" class="empty-notebook">
        <n-empty description="Empty notebook">
          <template #extra>
            <n-space>
              <n-button @click="addCodeCell">Add Code Cell</n-button>
              <n-button @click="addMarkdownCell">Add Markdown Cell</n-button>
            </n-space>
          </template>
        </n-empty>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick } from 'vue';
import { NButton, NSpace, NText, NSpin, NEmpty, NIcon } from 'naive-ui';
import { Add, CodeOutline, DocumentText } from '@vicons/ionicons5';
import { invoke } from '@tauri-apps/api/core';
import { debug, logError } from '../../utils/logger';
import { debounce } from 'lodash-es';
import NotebookCodeCell from './NotebookCodeCell.vue';
import NotebookMarkdownCell from './NotebookMarkdownCell.vue';
import { notebookExecutionService } from '../../services/notebookExecutionService';
import { embedImageOutputs } from '../../utils/notebookImageEmbedder';
import type { Notebook, NotebookCell, CellType } from '../../types/notebook';

interface Props {
  filePath: string;
  fileName: string;
  projectPath?: string;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  (e: 'dirty', value: boolean): void;
}>();

const loading = ref(false);
const error = ref<string | null>(null);
const notebook = ref<Notebook | null>(null);
const focusedCellIndex = ref<number | null>(null);
const hoveredCellIndex = ref<number | null>(null);
const isDirty = ref(false);
const runningAllCells = ref(false);

// Collapsible sections state
const collapsedSections = ref<Set<number>>(new Set());

// Check if a markdown cell is a section header (starts with ##)
function isSectionHeader(cell: NotebookCell): boolean {
  if (cell.cell_type !== 'markdown') return false;
  const source =
    typeof cell.source === 'string'
      ? cell.source
      : Array.isArray(cell.source)
        ? cell.source.join('')
        : '';
  return source.trim().startsWith('##');
}

// Toggle section collapsed state
function toggleSection(index: number) {
  if (collapsedSections.value.has(index)) {
    collapsedSections.value.delete(index);
  } else {
    collapsedSections.value.add(index);
  }
}

// Check if section is collapsed
function isSectionCollapsed(index: number): boolean {
  return collapsedSections.value.has(index);
}

// Check if a cell should be hidden (belongs to a collapsed section)
function isCellHidden(index: number): boolean {
  if (!notebook.value) return false;

  // Find the most recent section header before this cell
  for (let i = index - 1; i >= 0; i--) {
    const cell = notebook.value.cells[i];
    if (isSectionHeader(cell)) {
      return collapsedSections.value.has(i);
    }
  }

  return false;
}

// Generate unique cell IDs
let cellIdCounter = 0;
function generateCellId(): string {
  return `cell-${Date.now()}-${cellIdCounter++}`;
}

// Load notebook from file
async function loadNotebook() {
  loading.value = true;
  error.value = null;

  try {
    const loadedNotebook = await invoke<Notebook>('read_notebook', {
      path: props.filePath,
    });

    // Add IDs to cells that don't have them (Jupyter format doesn't include IDs, so we add them)
    loadedNotebook.cells = loadedNotebook.cells.map((cell, index) => {
      const cellWithId = cell as any;
      if (!cellWithId.id) {
        cellWithId.id = generateCellId();
      }
      return cellWithId;
    });

    notebook.value = loadedNotebook;
    isDirty.value = false;
    emit('dirty', false);
  } catch (err) {
    const errorMessage = err instanceof Error ? err.message : String(err);
    error.value = errorMessage;
    await logError('Failed to load notebook:', err);
  } finally {
    loading.value = false;
  }
}

// Save notebook to file
const saveNotebook = debounce(async () => {
  if (!notebook.value) return;

  try {
    // Process all cell outputs to embed images (convert URLs to embedded data)
    const processedNotebook: Notebook = {
      ...notebook.value,
      cells: await Promise.all(
        notebook.value.cells.map(async (cell) => {
          if (cell.outputs && cell.outputs.length > 0) {
            const embeddedOutputs = await embedImageOutputs(cell.outputs);
            return {
              ...cell,
              outputs: embeddedOutputs,
            };
          }
          return cell;
        })
      ),
    };

    await invoke('write_notebook', {
      path: props.filePath,
      notebook: processedNotebook,
    });
    isDirty.value = false;
    emit('dirty', false);
  } catch (err) {
    await logError('Failed to save notebook:', err);
  }
}, 500);

// Update cell source
function updateCellSource(index: number, source: string) {
  if (!notebook.value) return;
  notebook.value.cells[index].source = source;
  isDirty.value = true;
  emit('dirty', true);
  saveNotebook();
}

// Update cell outputs
function updateCellOutputs(index: number, outputs: any[]) {
  if (!notebook.value) return;
  notebook.value.cells[index].outputs = outputs;
  isDirty.value = true;
  emit('dirty', true);
  saveNotebook();
}

// Update cell execution count
function updateCellExecutionCount(index: number, executionCount: number | null) {
  if (!notebook.value) return;
  notebook.value.cells[index].execution_count = executionCount;
  isDirty.value = true;
  emit('dirty', true);
  saveNotebook();
}

// Execute cell
async function executeCell(index: number) {
  // Cell execution is handled by NotebookCodeCell component
  // This is just a placeholder for future enhancements (e.g., execute all cells above)
}

// Run all cells sequentially using batch execution (emits busy/done only at start/end)
async function runAllCells() {
  if (!notebook.value || runningAllCells.value) return;

  runningAllCells.value = true;
  const codeCells = notebook.value.cells.filter((cell) => cell.cell_type === 'code');

  // Prepare cells for batch execution
  const cellsToExecute: Array<{ cellId: string; code: string; cellIndex: number }> = [];

  for (let i = 0; i < codeCells.length; i++) {
    const cell = codeCells[i];
    const cellIndex = notebook.value.cells.indexOf(cell);

    if (cellIndex === -1) continue;

    // Get the cell's source code
    const source =
      typeof cell.source === 'string'
        ? cell.source
        : Array.isArray(cell.source)
          ? (() => {
              const hasNewlines = cell.source.some((s) => s.endsWith('\n'));
              return cell.source.join(hasNewlines ? '' : '\n');
            })()
          : '';

    if (!source.trim()) {
      // Skip empty cells
      continue;
    }

    const cellId = cell.id || `cell-${cellIndex}`;
    cellsToExecute.push({ cellId, code: source, cellIndex });
  }

  if (cellsToExecute.length === 0) {
    runningAllCells.value = false;
    return;
  }

  try {
    // Build cellIdsInOrder as a sparse array indexed by full notebook index
    // Backend emits cell_index based on full notebook index (including markdown cells)
    // So we need to map: fullNotebookIndex -> cellId
    const maxIndex = Math.max(...cellsToExecute.map((c) => c.cellIndex), -1);
    const cellIdsInOrder: (string | undefined)[] = new Array(maxIndex + 1);
    for (const { cellId, cellIndex } of cellsToExecute) {
      cellIdsInOrder[cellIndex] = cellId;
    }

    await debug(
      `[NotebookViewer] Executing notebook with ${cellsToExecute.length} code cells. cellIdsInOrder array length: ${cellIdsInOrder.length}, indices: ${cellsToExecute.map((c) => c.cellIndex).join(',')}`
    );

    const outputsMap = await notebookExecutionService.executeNotebookFile(
      props.filePath,
      cellIdsInOrder
    );

    // Update cells with outputs (execution counts not tracked for notebook-run)
    for (const { cellId, cellIndex } of cellsToExecute) {
      const outputs = outputsMap.get(cellId) || [];
      updateCellOutputs(cellIndex, outputs);
      updateCellExecutionCount(cellIndex, null);
    }
  } catch (error) {
    // Log error
    await logError('Failed to execute cells batch:', error);

    // Update cells with error output
    for (const { cellIndex } of cellsToExecute) {
      const errorOutput = [
        {
          output_type: 'error',
          ename: 'Error',
          evalue: error instanceof Error ? error.message : String(error),
          traceback: [],
        },
      ];
      updateCellOutputs(cellIndex, errorOutput);
    }
  } finally {
    runningAllCells.value = false;
  }
}

// Add new cell
function addCell(cellType: CellType, index?: number) {
  if (!notebook.value) return;

  const newCell: NotebookCell = {
    cell_type: cellType,
    source: '',
    metadata: {},
    outputs: [],
    execution_count: null,
    id: generateCellId(),
  } as NotebookCell;

  if (index !== undefined) {
    notebook.value.cells.splice(index + 1, 0, newCell);
  } else {
    notebook.value.cells.push(newCell);
  }

  isDirty.value = true;
  emit('dirty', true);
  saveNotebook();

  // Focus the new cell
  nextTick(() => {
    focusedCellIndex.value = index !== undefined ? index + 1 : notebook.value!.cells.length - 1;
  });
}

function addCodeCell() {
  const insertIndex =
    focusedCellIndex.value !== null ? focusedCellIndex.value : notebook.value?.cells.length || 0;
  addCell('code', insertIndex !== null ? insertIndex - 1 : undefined);
}

function addMarkdownCell() {
  const insertIndex =
    focusedCellIndex.value !== null ? focusedCellIndex.value : notebook.value?.cells.length || 0;
  addCell('markdown', insertIndex !== null ? insertIndex - 1 : undefined);
}

// Delete cell
function deleteCell(index: number) {
  if (!notebook.value) return;
  notebook.value.cells.splice(index, 1);
  isDirty.value = true;
  emit('dirty', true);
  saveNotebook();
}

// Handle keyboard shortcuts
function handleKeyDown(event: KeyboardEvent) {
  // Shift+Enter: Execute cell and move to next
  if (event.shiftKey && event.key === 'Enter') {
    event.preventDefault();
    if (focusedCellIndex.value !== null && notebook.value) {
      const cell = notebook.value.cells[focusedCellIndex.value];
      if (cell.cell_type === 'code') {
        // Trigger execution (handled by NotebookCodeCell)
        // Then move to next cell
        if (focusedCellIndex.value < notebook.value.cells.length - 1) {
          focusedCellIndex.value = focusedCellIndex.value + 1;
        } else {
          addCodeCell();
        }
      }
    }
  }
  // Ctrl+Enter / Cmd+Enter: Execute cell (stay in place)
  else if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
    event.preventDefault();
    // Execution is handled by NotebookCodeCell
  }
  // Alt+Enter: Execute cell and insert new below
  else if (event.altKey && event.key === 'Enter') {
    event.preventDefault();
    if (focusedCellIndex.value !== null && notebook.value) {
      const cell = notebook.value.cells[focusedCellIndex.value];
      if (cell.cell_type === 'code') {
        // Trigger execution (handled by NotebookCodeCell)
        // Then insert new cell below
        addCodeCell();
      }
    }
  }
}

onMounted(() => {
  loadNotebook();
  window.addEventListener('keydown', handleKeyDown);
});

// Cleanup
import { onBeforeUnmount } from 'vue';
onBeforeUnmount(() => {
  window.removeEventListener('keydown', handleKeyDown);
  saveNotebook.cancel();
});

// Expose runAllCells method for parent component
defineExpose({
  runAllCells,
});
</script>

<style scoped>
.notebook-viewer {
  background-color: #1e1e1e;
  color: #d4d4d4;
}

.notebook-content {
  background-color: #1e1e1e;
}

.cell-wrapper {
  position: relative;
  margin-bottom: 4px;
  transition: all 0.3s ease;
}

/* Floating buttons at boundary between cells */
.cell-boundary-actions {
  position: absolute;
  bottom: -20px;
  left: 50%;
  transform: translateX(-50%) translateY(-4px);
  display: flex;
  justify-content: center;
  gap: 8px;
  opacity: 0;
  transition:
    opacity 0.2s ease,
    transform 0.2s ease;
  pointer-events: none;
  z-index: 10;
}

.cell-boundary-actions.show-buttons {
  opacity: 1;
  transform: translateX(-50%) translateY(0);
  pointer-events: auto;
}

.boundary-button {
  background-color: rgba(40, 40, 40, 0.95) !important;
  backdrop-filter: blur(8px);
  border: 1px solid rgba(255, 255, 255, 0.1) !important;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  font-size: 12px;
}

.boundary-button:hover {
  background-color: rgba(60, 60, 60, 0.95) !important;
  border-color: rgba(255, 255, 255, 0.2) !important;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.4);
  transform: translateY(-1px);
}

.cell-wrapper.cell-hidden {
  display: none;
}

.section-fold-button {
  position: absolute;
  left: -24px;
  top: 12px;
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.2s ease;
  z-index: 10;
  border-radius: 4px;
  background-color: rgba(40, 40, 40, 0.8);
}

.cell-wrapper:hover .section-fold-button {
  opacity: 1;
}

.section-fold-button:hover {
  background-color: rgba(60, 60, 60, 0.9);
}

.section-fold-button .n-icon {
  transition: transform 0.2s ease;
}

.section-fold-button .icon-collapsed {
  transform: rotate(-90deg);
}

.notebook-raw-cell {
  border: 1px solid #333;
  border-radius: 4px;
  margin: 8px 0;
  background-color: #1e1e1e;
}

.notebook-raw-cell .cell-header {
  padding: 4px 8px;
  background-color: #252526;
  border-bottom: 1px solid #333;
}

.notebook-raw-cell .cell-content {
  padding: 12px;
}

.notebook-raw-cell pre {
  margin: 0;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 13px;
  color: #d4d4d4;
  white-space: pre-wrap;
  word-wrap: break-word;
}

.empty-notebook {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 400px;
}
</style>
