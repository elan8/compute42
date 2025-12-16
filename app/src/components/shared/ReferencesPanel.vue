<template>
  <div class="references-panel">
    <div class="references-header">
      <h3>References</h3>
      <button class="close-button" @click="handleClose" title="Close panel">
        <span>×</span>
      </button>
    </div>

    <div v-if="loading" class="references-loading">
      <n-spin size="small" />
      <span>Loading references...</span>
    </div>

    <div v-else-if="references.length === 0" class="references-empty">
      <p>No references found</p>
    </div>

    <div v-else class="references-list">
      <div
        v-for="(reference, index) in references"
        :key="index"
        class="reference-item"
        :class="{ 'reference-item--selected': selectedReferenceIndex === index }"
        @click="handleReferenceClick(reference, index)"
      >
        <div class="reference-preview">
          <div v-if="reference.previewLoading" class="preview-loading">
            <n-spin size="small" />
            <span>Loading preview...</span>
          </div>
          <div v-else-if="reference.previewError" class="preview-error">
            <span>Failed to load preview</span>
          </div>
          <div v-else-if="reference.preview" class="preview-content">
            {{ reference.preview }}
          </div>
          <div v-else class="preview-placeholder">...</div>
        </div>
        <div class="reference-header">
          <span class="reference-file">{{ getFileName(reference.uri) }}</span>
          <span class="reference-location"
            >L{{ reference.range.start.line + 1 }}:{{ reference.range.start.character + 1 }}</span
          >
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, onMounted, watch } from 'vue';
import { NSpin } from 'naive-ui';
import { invoke } from '@tauri-apps/api/core';
import { debug, info } from '../../utils/logger';
import { lspService } from '../../services/lspService';

const props = defineProps({
  uri: {
    type: String,
    required: true,
  },
  line: {
    type: Number,
    required: true,
  },
  character: {
    type: Number,
    required: true,
  },
});

const emit = defineEmits(['navigate', 'close']);

const references = ref([]);
const loading = ref(true);
const selectedReferenceIndex = ref(-1);

// Watch for prop changes and reset state
watch(
  () => [props.uri, props.line, props.character],
  () => {
    console.log('ReferencesPanel: Props changed, resetting state');
    references.value = [];
    loading.value = true;
    selectedReferenceIndex.value = -1;
    loadReferences();
  },
  { immediate: false }
);

// Function to load references
const loadReferences = async () => {
  console.log('ReferencesPanel: Component mounted with props:', props);
  try {
    debug('ReferencesPanel: Requesting references');
    console.log('ReferencesPanel: About to call lspService.requestReferences with:', {
      uri: props.uri,
      line: props.line,
      character: props.character,
    });

    const results = await lspService.requestReferences(props.uri, props.line, props.character);
    console.log('ReferencesPanel: LSP service returned:', results);

    // Add preview loading state to each reference
    const referencesWithPreview = results.map((ref) => ({
      ...ref,
      preview: null,
      previewLoading: true,
      previewError: false,
    }));

    references.value = referencesWithPreview;
    debug(`ReferencesPanel: Received ${results.length} references`);
    console.log('ReferencesPanel: References loaded:', references.value);

    // Load previews for all references
    await loadAllPreviews();
  } catch (error) {
    debug('ReferencesPanel: Failed to get references');
    console.error('ReferencesPanel: Error loading references:', error);
  } finally {
    loading.value = false;
  }
};

onMounted(async () => {
  await loadReferences();
});

async function loadAllPreviews() {
  debug('ReferencesPanel: Starting to load all previews');
  const previewPromises = references.value.map(async (reference, index) => {
    try {
      debug(`ReferencesPanel: Loading preview for reference ${index}`);
      await loadReferencePreview(reference, index);
    } catch (error) {
      console.error(`Failed to load preview for reference ${index}:`, error);
      debug(`ReferencesPanel: Failed to load preview for reference ${index}: ${error}`);
      references.value[index].previewError = true;
      references.value[index].previewLoading = false;
    }
  });

  await Promise.all(previewPromises);
  debug('ReferencesPanel: Finished loading all previews');
}

async function loadReferencePreview(reference, index) {
  try {
    debug(`ReferencesPanel: Loading preview for reference ${index}: ${reference.uri}`);

    // Convert URI to file path
    const filePath = uriToFilePath(reference.uri);
    debug(`ReferencesPanel: Converted URI to file path: ${reference.uri} -> ${filePath}`);

    if (!filePath) {
      debug('ReferencesPanel: Invalid URI - filePath is null');
      throw new Error('Invalid URI');
    }

    // Read file content
    debug(`ReferencesPanel: Reading file content for: ${filePath}`);
    let content;
    try {
      content = await invoke('read_file_content', { path: filePath });
      debug(`ReferencesPanel: File content length: ${content.length}`);
    } catch (fileError) {
      debug(`ReferencesPanel: Error reading file: ${fileError}`);
      throw fileError;
    }

    // Extract the line content
    const lines = content.split('\n');
    const lineNumber = reference.range.start.line;
    debug(`ReferencesPanel: Extracting line ${lineNumber} from ${lines.length} total lines`);

    if (lineNumber >= 0 && lineNumber < lines.length) {
      let lineContent = lines[lineNumber];
      debug(`ReferencesPanel: Line content: ${lineContent}`);

      // Highlight the reference range if it's on a single line
      if (reference.range.start.line === reference.range.end.line) {
        const startChar = reference.range.start.character;
        const endChar = Math.min(reference.range.end.character, lineContent.length);
        debug(
          `ReferencesPanel: Highlighting range: ${startChar}-${endChar} on line length ${lineContent.length}`
        );

        if (startChar < lineContent.length && endChar > startChar) {
          // Truncate if the line is too long
          const maxLength = 100;
          if (lineContent.length > maxLength) {
            const beforeHighlight = lineContent.substring(0, startChar);
            const highlighted = lineContent.substring(startChar, endChar);
            const afterHighlight = lineContent.substring(endChar);

            // Truncate intelligently
            let truncatedBefore = beforeHighlight;
            let truncatedAfter = afterHighlight;

            if (beforeHighlight.length > 30) {
              truncatedBefore = '...' + beforeHighlight.substring(beforeHighlight.length - 30);
            }
            if (afterHighlight.length > 30) {
              truncatedAfter = afterHighlight.substring(0, 30) + '...';
            }

            lineContent = truncatedBefore + `[${highlighted}]` + truncatedAfter;
          } else {
            lineContent =
              lineContent.substring(0, startChar) +
              `[${lineContent.substring(startChar, endChar)}]` +
              lineContent.substring(endChar);
          }
        }
      }

      debug(`ReferencesPanel: Final preview content: ${lineContent}`);
      references.value[index].preview = lineContent;
    } else {
      debug(`ReferencesPanel: Line ${lineNumber} not found in file with ${lines.length} lines`);
      references.value[index].preview = 'Line not found';
    }
  } catch (error) {
    debug(`ReferencesPanel: Error loading preview: ${error}`);
    debug(`ReferencesPanel: Reference details: ${JSON.stringify(reference)}`);
    references.value[index].preview = 'Failed to load preview';
  } finally {
    references.value[index].previewLoading = false;
  }
}

function uriToFilePath(uri) {
  console.log(`Converting URI to file path:`, uri);
  try {
    // Handle file:// URIs
    if (uri.startsWith('file://')) {
      const url = new URL(uri);
      let pathname = decodeURIComponent(url.pathname);

      // Fix Windows paths - remove leading slash if it exists
      if (pathname.startsWith('/') && pathname.length > 2 && pathname[2] === ':') {
        pathname = pathname.substring(1);
      }

      console.log(`File URI converted:`, uri, '->', pathname);
      return pathname;
    }

    // Handle relative paths or other URI formats
    if (uri.startsWith('/')) {
      console.log(`Absolute path:`, uri);
      return uri;
    }

    // For Windows paths, handle file:///c:/path format
    if (uri.startsWith('file:///')) {
      const path = uri.substring(7); // Remove 'file:///'
      console.log(`Windows file URI converted:`, uri, '->', path);
      return path;
    }

    console.log(`Returning URI as-is:`, uri);
    return uri;
  } catch (error) {
    console.error('Error converting URI to file path:', error);
    console.error('URI was:', uri);
    return null;
  }
}

function getFileName(uri) {
  try {
    const url = new URL(uri);
    const path = decodeURIComponent(url.pathname);
    return path.split('/').pop() || 'Unknown';
  } catch {
    return uri.split('/').pop() || 'Unknown';
  }
}

function handleReferenceClick(reference, index) {
  selectedReferenceIndex.value = index;
  emit('navigate', reference);
}

function handleClose() {
  emit('close');
}
</script>

<style scoped>
.references-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: #1e1e1e;
  border-left: 1px solid #444;
}

.references-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid #444;
  background-color: #2d2d2d;
}

.references-header h3 {
  margin: 0;
  font-size: 14px;
  font-weight: 600;
  color: #e6e6e6;
}

.close-button {
  background: none;
  border: none;
  color: #ccc;
  font-size: 18px;
  cursor: pointer;
  padding: 0;
  width: 24px;
  height: 24px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  transition: background-color 0.2s;
}

.close-button:hover {
  background-color: #555;
  color: #fff;
}

.references-loading {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 16px;
  color: #888;
}

.references-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #888;
}

.references-list {
  flex: 1;
  overflow-y: auto;
}

.reference-item {
  padding: 12px 16px;
  border-bottom: 1px solid #333;
  cursor: pointer;
  transition: background-color 0.2s;
}

.reference-item:hover {
  background-color: #2a2a2a;
}

.reference-item--selected {
  background-color: #3a3a3a;
}

.reference-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-top: 8px;
}

.reference-file {
  font-weight: 600;
  color: #e6e6e6;
  font-size: 13px;
}

.reference-location {
  color: #888;
  font-size: 12px;
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
}

.reference-preview {
  color: #ccc;
  font-size: 12px;
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
  line-height: 1.4;
  white-space: pre-wrap;
  word-break: break-word;
}

.preview-loading {
  display: flex;
  align-items: center;
  gap: 6px;
  color: #888;
  font-size: 11px;
}

.preview-error {
  color: #ff6b6b;
  font-size: 11px;
  font-style: italic;
}

.preview-content {
  background-color: #2a2a2a;
  padding: 8px;
  border-radius: 4px;
  border-left: 3px solid #4ade80;
}

.preview-placeholder {
  color: #666;
  font-style: italic;
}
</style>
