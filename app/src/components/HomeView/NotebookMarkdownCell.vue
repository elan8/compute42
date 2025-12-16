<template>
  <div class="notebook-markdown-cell" :class="{ 'cell-focused': isFocused, 'edit-mode': editMode }">
    <div class="cell-content-wrapper">
      <!-- Edit Mode -->
      <div v-if="editMode" class="cell-editor">
        <CodeMirrorNotebookEditor
          :value="sourceString"
          language="markdown"
          :readOnly="false"
          placeholder="Enter markdown..."
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

      <!-- Preview Mode -->
      <div v-else class="cell-preview" @click="toggleEditMode">
        <div class="markdown-content" v-html="renderedMarkdown"></div>
      </div>

      <!-- Minimal action buttons - only show on hover, in right column -->
      <div class="cell-actions">
        <n-button size="small" quaternary circle @click.stop="toggleEditMode" class="action-button">
          <template #icon>
            <n-icon v-if="editMode"><EyeOutline /></n-icon>
            <n-icon v-else><CreateOutline /></n-icon>
          </template>
        </n-button>
        <n-button
          size="small"
          quaternary
          circle
          @click.stop="$emit('delete')"
          class="action-button delete-button"
        >
          <template #icon>
            <n-icon><Trash /></n-icon>
          </template>
        </n-button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { NButton, NButtonGroup, NIcon, NText } from 'naive-ui';
import { CreateOutline, EyeOutline, Trash } from '@vicons/ionicons5';
import CodeMirrorNotebookEditor from './CodeMirrorNotebookEditor.vue';
import { marked } from 'marked';
import { nextTick } from 'vue';

const props = defineProps<{
  cellId: string;
  source: string | string[] | null | undefined;
}>();

const emit = defineEmits<{
  (e: 'update:source', value: string): void;
  (e: 'delete'): void;
  (e: 'focus'): void;
  (e: 'blur'): void;
}>();

const editorRef = ref<InstanceType<typeof CodeMirrorNotebookEditor> | null>(null);
const isFocused = ref(false);
const editMode = ref(false);

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
    // Also check for carriage returns just in case
    const hasReturns = !hasNewlines && props.source.some((s) => s.endsWith('\r'));
    const separator = hasNewlines || hasReturns ? '' : '\n';
    return props.source.join(separator);
  }
  if (props.source == null) {
    return '';
  }
  return String(props.source);
});

onMounted(() => {
  // Start in preview mode if cell has content, otherwise edit mode
  editMode.value = sourceString.value.trim().length === 0;
});

// Configure marked for safe rendering
marked.setOptions({
  breaks: true,
  gfm: true,
});

const renderedMarkdown = computed(() => {
  const source = sourceString.value;
  if (!source.trim()) {
    return '<p class="empty-markdown">Empty markdown cell</p>';
  }
  try {
    return marked.parse(source);
  } catch (error) {
    console.error('Markdown rendering error:', error);
    return `<p style="color: #f44336;">Error rendering markdown: ${error}</p>`;
  }
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

function focusEditor() {
  if (editorRef.value && 'focus' in editorRef.value) {
    editorRef.value.focus();
  }
}

function toggleEditMode() {
  editMode.value = !editMode.value;
  if (editMode.value) {
    nextTick(() => {
      focusEditor();
    });
  }
}
</script>

<style scoped>
.notebook-markdown-cell {
  position: relative;
  border: 1px solid transparent;
  border-radius: 8px;
  margin: 4px 0;
  padding: 2px 0;
  background-color: transparent;
  transition: all 0.2s ease;
}

.cell-content-wrapper {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 0 12px;
}

.notebook-markdown-cell:hover {
  background-color: rgba(30, 30, 30, 0.3);
  border-color: rgba(255, 255, 255, 0.05);
}

.notebook-markdown-cell.cell-focused,
.notebook-markdown-cell.edit-mode {
  border-color: rgba(0, 122, 204, 0.3);
  background-color: rgba(30, 30, 30, 0.5);
}

/* Action buttons - hidden by default, show on hover, in right column */
.cell-actions {
  display: flex;
  flex-direction: row;
  gap: 4px;
  opacity: 0;
  transition: opacity 0.2s ease;
  flex-shrink: 0;
  padding-top: 2px;
}

.notebook-markdown-cell:hover .cell-actions,
.notebook-markdown-cell.cell-focused .cell-actions,
.notebook-markdown-cell.edit-mode .cell-actions {
  opacity: 1;
}

.cell-editor,
.cell-preview {
  flex: 1;
  min-width: 0;
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
  /* No min-height - CodeMirror will auto-size */
}

.cell-preview {
  padding: 0;
  min-height: 24px;
  cursor: text;
}

.markdown-content {
  color: #e0e0e0;
  line-height: 1.7;
  font-size: 15px;
  font-family:
    -apple-system, BlinkMacSystemFont, 'Segoe UI', 'Roboto', 'Helvetica', 'Arial', sans-serif;
}

.markdown-content :deep(h1),
.markdown-content :deep(h2),
.markdown-content :deep(h3),
.markdown-content :deep(h4),
.markdown-content :deep(h5),
.markdown-content :deep(h6) {
  color: #ffffff;
  margin-top: 20px;
  margin-bottom: 12px;
  font-weight: 600;
  letter-spacing: -0.02em;
}

.markdown-content :deep(h1) {
  font-size: 26px;
  border-bottom: 2px solid rgba(255, 255, 255, 0.1);
  padding-bottom: 10px;
  margin-top: 24px;
}

.markdown-content :deep(h2) {
  font-size: 22px;
  margin-top: 20px;
}

.markdown-content :deep(h3) {
  font-size: 18px;
}

.markdown-content :deep(p) {
  margin: 10px 0;
  line-height: 1.7;
}

.markdown-content :deep(code) {
  background-color: rgba(255, 255, 255, 0.08);
  padding: 2px 6px;
  border-radius: 4px;
  font-family: 'JetBrains Mono', 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 0.88em;
  color: #e0e0e0;
  border: 1px solid rgba(255, 255, 255, 0.05);
}

.markdown-content :deep(pre) {
  background-color: rgba(0, 0, 0, 0.3);
  padding: 14px 16px;
  border-radius: 6px;
  overflow-x: auto;
  border: 1px solid rgba(255, 255, 255, 0.08);
  margin: 12px 0;
}

.markdown-content :deep(pre code) {
  background-color: transparent;
  padding: 0;
}

.markdown-content :deep(ul),
.markdown-content :deep(ol) {
  margin: 8px 0;
  padding-left: 24px;
}

.markdown-content :deep(li) {
  margin: 4px 0;
}

.markdown-content :deep(blockquote) {
  border-left: 3px solid #007acc;
  padding-left: 12px;
  margin: 8px 0;
  color: #858585;
}

.markdown-content :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 12px 0;
}

.markdown-content :deep(th),
.markdown-content :deep(td) {
  border: 1px solid #444;
  padding: 8px;
  text-align: left;
}

.markdown-content :deep(th) {
  background-color: #2d2d2d;
  font-weight: 600;
}

.markdown-content :deep(a) {
  color: #4a9eff;
  text-decoration: none;
}

.markdown-content :deep(a:hover) {
  text-decoration: underline;
}

.empty-markdown {
  color: #666;
  font-style: italic;
}
</style>
