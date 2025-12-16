<template>
  <div ref="editorContainer" class="monaco-preview-container"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, nextTick } from 'vue';
import * as monaco from 'monaco-editor';
import type { editor } from 'monaco-editor';
import {
  ensureDarkPlusTheme,
  normalizeThemeName,
  switchEditorTheme,
} from '../../utils/monacoThemeUtils';

const props = defineProps({
  value: {
    type: String,
    required: true,
  },
  language: {
    type: String,
    default: 'julia',
  },
  theme: {
    type: String,
    default: 'vs-dark',
  },
  fontFamily: {
    type: String,
    default: null,
  },
  fontSize: {
    type: Number,
    default: null,
  },
  wordWrap: {
    type: Boolean,
    default: false,
  },
  tabSize: {
    type: Number,
    default: 4,
  },
  lineNumbers: {
    type: Boolean,
    default: true,
  },
  minimap: {
    type: Boolean,
    default: true,
  },
});

const editorContainer = ref<HTMLElement | null>(null);
let editorInstance: editor.IStandaloneCodeEditor | null = null;
let currentTheme = 'dark-plus';

onMounted(async () => {
  await nextTick();

  if (editorContainer.value) {
    // Ensure dark-plus theme is defined
    ensureDarkPlusTheme();

    const themeToUse = normalizeThemeName(props.theme);
    currentTheme = themeToUse;

    // Create model
    const modelUri = monaco.Uri.parse(`preview://example.${props.language}`);
    let model = monaco.editor.getModel(modelUri);

    if (!model) {
      model = monaco.editor.createModel(props.value, props.language, modelUri);
    } else {
      model.setValue(props.value);
      monaco.editor.setModelLanguage(model, props.language);
    }

    // Create editor instance
    editorInstance = monaco.editor.create(editorContainer.value, {
      model: model,
      theme: themeToUse,
      readOnly: true,
      automaticLayout: true,
      fontFamily: props.fontFamily || '"Fira Code", "Consolas", "Monaco", monospace',
      fontSize: props.fontSize || 14,
      wordWrap: props.wordWrap ? 'on' : 'off',
      tabSize: props.tabSize || 4,
      lineNumbers: props.lineNumbers ? 'on' : 'off',
      minimap: { enabled: props.minimap },
      scrollBeyondLastLine: false,
      scrollbar: {
        verticalScrollbarSize: 10,
        horizontalScrollbarSize: 10,
      },
    });
  }
});

// Watch for prop changes and update editor
watch(
  () => props.theme,
  async (newTheme) => {
    if (editorInstance) {
      const themeToUse = normalizeThemeName(newTheme);

      if (themeToUse !== currentTheme) {
        currentTheme = themeToUse;

        await switchEditorTheme(editorInstance, newTheme);
      }
    }
  }
);

watch(
  () => [
    props.fontFamily,
    props.fontSize,
    props.wordWrap,
    props.tabSize,
    props.lineNumbers,
    props.minimap,
  ],
  () => {
    if (editorInstance) {
      editorInstance.updateOptions({
        fontFamily: props.fontFamily || '"Fira Code", "Consolas", "Monaco", monospace',
        fontSize: props.fontSize || 14,
        wordWrap: props.wordWrap ? 'on' : 'off',
        tabSize: props.tabSize || 4,
        lineNumbers: props.lineNumbers ? 'on' : 'off',
        minimap: { enabled: props.minimap },
      });
    }
  }
);

watch(
  () => props.value,
  (newValue) => {
    if (editorInstance) {
      const model = editorInstance.getModel();
      if (model && model.getValue() !== newValue) {
        model.setValue(newValue);
      }
    }
  }
);

watch(
  () => props.language,
  (newLanguage) => {
    if (editorInstance) {
      const model = editorInstance.getModel();
      if (model) {
        monaco.editor.setModelLanguage(model, newLanguage);
      }
    }
  }
);

onBeforeUnmount(() => {
  if (editorInstance) {
    editorInstance.dispose();
    editorInstance = null;
  }
});
</script>

<style scoped>
.monaco-preview-container {
  width: 100%;
  height: 400px;
  min-height: 300px;
  border: 1px solid var(--n-border-color);
  border-radius: 4px;
  overflow: hidden;
}
</style>
