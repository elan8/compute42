<template>
  <div
    class="notebook-code-cell"
    :class="{ 'cell-focused': isFocused, 'cell-running': executionState === 'running' }"
  >
    <!-- Minimal action buttons - only show on hover -->
    <div class="cell-actions">
      <n-button
        v-if="executionState === 'running'"
        size="small"
        quaternary
        circle
        @click="cancelExecution"
        class="action-button"
      >
        <template #icon>
          <n-icon><Stop /></n-icon>
        </template>
      </n-button>
      <n-button
        size="small"
        quaternary
        circle
        @click="$emit('delete')"
        class="action-button delete-button"
      >
        <template #icon>
          <n-icon><Trash /></n-icon>
        </template>
      </n-button>
    </div>

    <div class="cell-editor">
      <CodeMirrorNotebookEditor
        :value="sourceString"
        language="julia"
        :readOnly="false"
        @update:value="handleContentChanged"
        @focus="handleFocus"
        @blur="handleBlur"
        :ref="
          (el) => {
            editorRef = el;
          }
        "
      />
    </div>

    <div v-if="outputs.length > 0" class="cell-outputs">
      <NotebookOutput v-for="(output, index) in outputs" :key="index" :output="output" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue';
import { NButton, NIcon } from 'naive-ui';
import { Stop, Trash } from '@vicons/ionicons5';
import CodeMirrorNotebookEditor from './CodeMirrorNotebookEditor.vue';
import NotebookOutput from './NotebookOutput.vue';
import { notebookExecutionService } from '../../services/notebookExecutionService';
import type { CellOutput, CellExecutionState } from '../../types/notebook';

const props = defineProps<{
  cellId: string;
  source: string | string[] | null | undefined;
  executionCount: number | null;
  outputs: CellOutput[];
  notebookPath?: string;
}>();

const emit = defineEmits<{
  (e: 'update:source', value: string): void;
  (e: 'update:outputs', value: CellOutput[]): void;
  (e: 'update:executionCount', value: number | null): void;
  (e: 'execute'): void;
  (e: 'delete'): void;
  (e: 'focus'): void;
  (e: 'blur'): void;
}>();

const editorRef = ref<InstanceType<typeof CodeMirrorNotebookEditor> | null>(null);
const isFocused = ref(false);
const executionState = ref<CellExecutionState>('idle');

// Ensure source is always a string
// Jupyter notebooks can have source as either a string or an array of strings
// When it's an array, each element is a line, so we need to join with newlines
const sourceString = computed(() => {
  if (typeof props.source === 'string') {
    return props.source;
  }
  if (Array.isArray(props.source)) {
    // Check if the source lines already contain newlines (standard Jupyter)
    // or if they are stripped (some other formats/implementations)
    const hasNewlines = props.source.some((s) => s.endsWith('\n'));
    const separator = hasNewlines ? '' : '\n';
    return props.source.join(separator);
  }
  if (props.source == null) {
    return '';
  }
  return String(props.source);
});

function handleContentChanged(newValue: string) {
  emit('update:source', newValue);
}

function handleFocus() {
  isFocused.value = true;
  emit('focus');
}

function handleBlur() {
  isFocused.value = false;
  emit('blur');
}

async function executeCell() {
  executionState.value = 'running';
  emit('execute');

  try {
    const outputs = await notebookExecutionService.executeCell(
      props.cellId,
      sourceString.value,
      props.notebookPath
    );
    const executionCount = notebookExecutionService.getExecutionCount();

    emit('update:outputs', outputs);
    emit('update:executionCount', executionCount);
    executionState.value = outputs.some((o) => o.output_type === 'error') ? 'error' : 'success';
  } catch (error) {
    console.error('Cell execution failed:', error);
    executionState.value = 'error';
    emit('update:outputs', [
      {
        output_type: 'error',
        ename: 'Error',
        evalue: error instanceof Error ? error.message : String(error),
        traceback: [],
      },
    ]);
  }
}

function cancelExecution() {
  notebookExecutionService.cancelExecution(props.cellId);
  executionState.value = 'idle';
}
</script>

<style scoped>
.notebook-code-cell {
  position: relative;
  border: 1px solid transparent;
  border-radius: 8px;
  margin: 4px 0;
  padding: 8px 12px;
  background-color: rgba(30, 30, 30, 0.3);
  transition: all 0.2s ease;
}

.notebook-code-cell:hover {
  background-color: rgba(30, 30, 30, 0.6);
  border-color: rgba(255, 255, 255, 0.1);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2);
}

.notebook-code-cell.cell-focused {
  border-color: rgba(0, 122, 204, 0.5);
  background-color: rgba(30, 30, 30, 0.8);
}

.notebook-code-cell.cell-running {
  border-color: rgba(76, 175, 80, 0.5);
}

/* Action buttons - hidden by default, show on hover */
.cell-actions {
  position: absolute;
  top: 8px;
  right: 8px;
  display: flex;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.2s ease;
  z-index: 10;
}

.notebook-code-cell:hover .cell-actions,
.notebook-code-cell.cell-focused .cell-actions,
.notebook-code-cell.cell-running .cell-actions {
  opacity: 1;
}

.action-button {
  background-color: rgba(40, 40, 40, 0.9) !important;
  backdrop-filter: blur(4px);
}

.action-button:hover {
  background-color: rgba(60, 60, 60, 0.9) !important;
}

.delete-button:hover {
  background-color: rgba(244, 67, 54, 0.2) !important;
  color: #f44336 !important;
}

.cell-editor {
  position: relative;
  margin-top: 4px;
  /* No min-height - CodeMirror will auto-size */
}

.cell-outputs {
  margin-top: 8px;
  padding-top: 8px;
  border-top: 1px solid rgba(255, 255, 255, 0.05);
  background-color: transparent;
}
</style>
