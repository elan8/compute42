<template>
  <div ref="editorContainer" class="codemirror-notebook-editor"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, shallowRef } from 'vue';
import {
  EditorView,
  keymap,
  ViewPlugin,
  Decoration,
  WidgetType,
  ViewUpdate,
} from '@codemirror/view';
import { EditorState, Compartment } from '@codemirror/state';
import { defaultKeymap, indentWithTab } from '@codemirror/commands';
// Removed oneDark import - we're using a custom dark theme instead
// import { oneDark } from '@codemirror/theme-one-dark';
import {
  syntaxHighlighting,
  defaultHighlightStyle,
  HighlightStyle,
  syntaxTree,
  bracketMatching,
} from '@codemirror/language';
import { tags } from '@lezer/highlight';
import { closeBrackets, closeBracketsKeymap } from '@codemirror/autocomplete';
import { searchKeymap } from '@codemirror/search';
import { julia } from '@plutojl/lang-julia';
import { juliaColors } from '../../utils/juliaColorScheme';
import { info as logInfo } from '@tauri-apps/plugin-log';

const props = defineProps<{
  value: string;
  language?: string;
  readOnly?: boolean;
  placeholder?: string;
}>();

const emit = defineEmits<{
  (e: 'update:value', value: string): void;
  (e: 'focus'): void;
  (e: 'blur'): void;
  (e: 'execute'): void; // Shift+Enter to execute
}>();

const editorContainer = ref<HTMLElement | null>(null);
const editorView = shallowRef<EditorView | null>(null);
const readOnlyCompartment = new Compartment();
const languageCompartment = new Compartment();
const highlightCompartment = new Compartment();

// Custom highlight style using shared color scheme
// @plutojl/lang-julia doesn't assign standard Lezer tags, so we need to map node types
// Based on syntax tree inspection: keywords are tagged as typeName, strings/identifiers have no-tags
// We'll use a ViewPlugin to detect function calls by syntax tree structure
const juliaHighlightStyle = HighlightStyle.define(
  [
    // Comments - GREEN (#6A9955)
    { tag: tags.comment, color: juliaColors.comment, fontStyle: 'italic' },
    { tag: tags.lineComment, color: juliaColors.comment, fontStyle: 'italic' },
    { tag: tags.blockComment, color: juliaColors.comment, fontStyle: 'italic' },

    // Keywords - BLUE (#569CD6)
    // Note: @plutojl/lang-julia tags keywords as typeName, so we map typeName to keyword color
    // But we'll use a ViewPlugin to properly detect keywords vs types
    { tag: tags.keyword, color: juliaColors.keyword },
    { tag: tags.typeName, color: juliaColors.keyword }, // TEMPORARY: keywords are tagged as typeName

    // Strings - ORANGE (#CE9178)
    { tag: tags.string, color: juliaColors.string },
    { tag: tags.special(tags.string), color: juliaColors.string },

    // Numbers/Literals - LIGHT GREEN (#B5CEA8)
    { tag: tags.number, color: juliaColors.number },
    { tag: tags.integer, color: juliaColors.number },
    { tag: tags.float, color: juliaColors.number },
    { tag: tags.literal, color: juliaColors.number },
    { tag: tags.character, color: juliaColors.number },

    // Functions - PURPLE (#C586C0)
    { tag: tags.function, color: juliaColors.function },
    { tag: tags.function(tags.variableName), color: juliaColors.function },
    { tag: tags.function(tags.definition(tags.variableName)), color: juliaColors.function },
    { tag: tags.function(tags.name), color: juliaColors.function },

    // Types - CYAN (#4EC9B0) - will be overridden by ViewPlugin for actual types
    { tag: tags.namespace, color: juliaColors.namespace },

    // Macros - BLUE (like keywords) (#569CD6)
    { tag: tags.macroName, color: juliaColors.macro },

    // Constants - BLUE (#569CD6)
    { tag: tags.constant(tags.variableName), color: juliaColors.constant },
    { tag: tags.constant(tags.name), color: juliaColors.constant },
    { tag: tags.null, color: juliaColors.constant },
    { tag: tags.bool, color: juliaColors.constant },
    { tag: tags.atom, color: juliaColors.constant },

    // Variables - DEFAULT GRAY (#D4D4D4)
    { tag: tags.variableName, color: juliaColors.variable },
    { tag: tags.standard(tags.variableName), color: juliaColors.variable },
    { tag: tags.name, color: juliaColors.variable },

    // Properties and attributes
    { tag: tags.propertyName, color: juliaColors.property },
    { tag: tags.attributeName, color: juliaColors.attribute },

    // Definitions
    { tag: tags.definition(tags.variableName), color: juliaColors.definition },

    // Operators and punctuation - DEFAULT TEXT COLOR (#D4D4D4)
    { tag: tags.operator, color: juliaColors.operator },
    { tag: tags.punctuation, color: juliaColors.punctuation },

    // Brackets and delimiters
    { tag: tags.bracket, color: juliaColors.bracket },
    { tag: tags.squareBracket, color: juliaColors.bracket },
    { tag: tags.paren, color: juliaColors.bracket },
    { tag: tags.brace, color: juliaColors.bracket },

    // Other tokens
    { tag: tags.tagName, color: juliaColors.constant },
    { tag: tags.escape, color: '#D7BA7D' }, // Escape sequences - yellow
  ],
  {
    // Set default color for all unmapped tags
    all: juliaColors.defaultText,
  }
);

// ViewPlugin to detect function calls by syntax tree structure
// Since @plutojl/lang-julia doesn't tag function calls, we detect them by context
const juliaFunctionCallPlugin = ViewPlugin.fromClass(
  class {
    decorations: any;

    constructor(view: EditorView) {
      this.decorations = this.buildDecorations(view);
    }

    update(update: ViewUpdate) {
      if (update.docChanged || update.viewportChanged) {
        this.decorations = this.buildDecorations(update.view);
      }
    }

    buildDecorations(view: EditorView) {
      const decorations: any[] = [];
      const tree = syntaxTree(view.state);
      const JULIA_KEYWORDS = new Set([
        'function',
        'if',
        'else',
        'elseif',
        'for',
        'while',
        'end',
        'begin',
        'let',
        'do',
        'try',
        'catch',
        'finally',
        'return',
        'break',
        'continue',
        'using',
        'import',
        'export',
        'const',
        'struct',
        'mutable',
        'abstract',
        'type',
        'module',
        'macro',
        'where',
        'in',
        'isa',
      ]);

      // Walk the tree to find function calls
      // We need to detect: Symbol/Identifier that is immediately followed by ParenExpression
      const functionCalls = new Set<number>(); // Track positions that are function calls

      tree.iterate({
        enter(node) {
          const nodeName = node.name;

          // Detect ParenExpression - check if its first child is a Symbol/Identifier (function call)
          if (nodeName === 'ParenExpression') {
            // Get the first child node
            let child = node.node.firstChild;
            if (child && (child.name === 'Symbol' || child.name === 'Identifier')) {
              const funcName = view.state.sliceDoc(child.from, child.to).trim();
              // Skip keywords
              if (!JULIA_KEYWORDS.has(funcName)) {
                functionCalls.add(child.from);
                functionCalls.add(child.to);
              }
            }
          }
        },
      });

      // Now apply decorations
      tree.iterate({
        enter(node) {
          const nodeName = node.name;
          const text = view.state.sliceDoc(node.from, node.to).trim();

          // Detect function calls: Symbol/Identifier that we identified as function calls
          if (
            (nodeName === 'Symbol' || nodeName === 'Identifier') &&
            functionCalls.has(node.from)
          ) {
            decorations.push(
              Decoration.mark({
                class: 'cm-function-call',
                attributes: { style: `color: ${juliaColors.function} !important;` },
              }).range(node.from, node.to)
            );
          }

          // Detect string literals (StringLiteral nodes)
          if (nodeName === 'StringLiteral') {
            decorations.push(
              Decoration.mark({
                class: 'cm-string-literal',
                attributes: { style: `color: ${juliaColors.string} !important;` },
              }).range(node.from, node.to)
            );
          }

          // Detect number literals
          if (nodeName === 'IntegerLiteral' || nodeName === 'FloatLiteral') {
            decorations.push(
              Decoration.mark({
                class: 'cm-number-literal',
                attributes: { style: `color: ${juliaColors.number} !important;` },
              }).range(node.from, node.to)
            );
          }
        },
      });

      return Decoration.set(decorations);
    }
  },
  { decorations: (v) => v.decorations }
);

// Custom placeholder extension
function createPlaceholder(text: string) {
  const placeholderWidget = Decoration.widget({
    widget: new (class extends WidgetType {
      toDOM() {
        const span = document.createElement('span');
        span.className = 'cm-placeholder';
        span.style.cssText = 'color: #999; pointer-events: none;';
        span.textContent = text;
        return span;
      }
    })(),
    side: 1,
  });

  return ViewPlugin.fromClass(
    class {
      decorations;
      constructor(view: EditorView) {
        this.decorations =
          view.state.doc.length === 0
            ? Decoration.set([placeholderWidget.range(0)])
            : Decoration.none;
      }
      update(update: ViewUpdate) {
        if (update.docChanged) {
          this.decorations =
            update.view.state.doc.length === 0
              ? Decoration.set([placeholderWidget.range(0)])
              : Decoration.none;
        }
      }
    },
    {
      decorations: (v) => v.decorations,
    }
  );
}

// Custom keymap for notebook shortcuts
const notebookKeymap = keymap.of([
  {
    key: 'Shift-Enter',
    run: () => {
      emit('execute');
      return true;
    },
  },
  {
    key: 'Ctrl-Enter',
    mac: 'Cmd-Enter',
    run: () => {
      emit('execute');
      return true;
    },
  },
  ...closeBracketsKeymap,
  ...searchKeymap,
  ...defaultKeymap,
  indentWithTab,
]);

// Create editor on mount
onMounted(() => {
  if (!editorContainer.value) return;

  const extensions = [
    // Auto-sizing - this is the key for notebook cells!
    EditorView.theme({
      '&': {
        fontSize: '14px',
        fontFamily: 'JetBrains Mono, Consolas, Monaco, Courier New, monospace',
      },
      '.cm-content': {
        minHeight: '60px',
        padding: '8px 0',
      },
      '.cm-line': {
        padding: '0 4px',
      },
      '.cm-scroller': {
        overflow: 'auto',
        fontFamily: 'inherit',
      },
    }),
    // Custom dark theme (without highlight style - we'll add our own)
    // This replaces oneDark so our highlight style can work properly
    EditorView.theme(
      {
        '&': {
          backgroundColor: '#1e1e1e',
          color: '#d4d4d4',
        },
        '.cm-content': {
          caretColor: '#aeafad',
        },
        '.cm-cursor, .cm-dropCursor': {
          borderLeftColor: '#aeafad',
        },
        '.cm-selectionBackground': {
          backgroundColor: '#264f78',
        },
        '.cm-focused .cm-selectionBackground': {
          backgroundColor: '#264f78',
        },
        '.cm-selectionMatch': {
          backgroundColor: '#3a3d41',
        },
        '.cm-gutters': {
          backgroundColor: '#1e1e1e',
          color: '#858585',
          border: 'none',
        },
        '.cm-activeLineGutter': {
          backgroundColor: '#2a2d2e',
        },
        '.cm-activeLine': {
          backgroundColor: '#2a2d2e',
        },
        '.cm-scroller': {
          fontFamily: 'JetBrains Mono, Consolas, Monaco, Courier New, monospace',
        },
      },
      { dark: true }
    ),
    // Language support - apply Julia language if specified
    languageCompartment.of(props.language === 'julia' ? julia() : []),
    // Syntax highlighting - now our custom style will work since oneDark isn't overriding it
    highlightCompartment.of(
      syntaxHighlighting(props.language === 'julia' ? juliaHighlightStyle : defaultHighlightStyle)
    ),
    // Custom ViewPlugin for Julia to detect function calls by syntax tree structure
    // This is needed because @plutojl/lang-julia doesn't assign function tags
    ...(props.language === 'julia' ? [juliaFunctionCallPlugin] : []),
    // Bracket matching
    bracketMatching(),
    // Auto-close brackets
    closeBrackets(),
    // Keymaps
    notebookKeymap,
    // Read-only state
    readOnlyCompartment.of(EditorState.readOnly.of(props.readOnly ?? false)),
    // Update on change
    EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        const newValue = update.state.doc.toString();
        emit('update:value', newValue);
      }

      // Emit focus/blur events
      if (update.focusChanged) {
        if (update.view.hasFocus) {
          emit('focus');
        } else {
          emit('blur');
        }
      }
    }),
  ];

  // Add placeholder if provided
  if (props.placeholder) {
    extensions.push(createPlaceholder(props.placeholder));
  }

  const state = EditorState.create({
    doc: props.value,
    extensions,
  });

  editorView.value = new EditorView({
    state,
    parent: editorContainer.value,
  });
});

// Watch for external value changes
watch(
  () => props.value,
  (newValue) => {
    if (!editorView.value) return;

    const currentValue = editorView.value.state.doc.toString();
    if (currentValue !== newValue) {
      editorView.value.dispatch({
        changes: {
          from: 0,
          to: currentValue.length,
          insert: newValue,
        },
      });
    }
  }
);

// Watch for readOnly changes
watch(
  () => props.readOnly,
  (newReadOnly) => {
    if (!editorView.value) return;

    editorView.value.dispatch({
      effects: readOnlyCompartment.reconfigure(EditorState.readOnly.of(newReadOnly ?? false)),
    });
  }
);

// Watch for language changes
watch(
  () => props.language,
  (newLanguage) => {
    if (!editorView.value) return;

    editorView.value.dispatch({
      effects: [
        languageCompartment.reconfigure(newLanguage === 'julia' ? julia() : []),
        highlightCompartment.reconfigure(
          syntaxHighlighting(newLanguage === 'julia' ? juliaHighlightStyle : defaultHighlightStyle)
        ),
      ],
    });
  }
);

// Cleanup
onBeforeUnmount(() => {
  if (editorView.value) {
    editorView.value.destroy();
    editorView.value = null;
  }
});

// Public methods
const focus = () => {
  editorView.value?.focus();
};

const getCurrentValue = () => {
  return editorView.value?.state.doc.toString() ?? '';
};

defineExpose({
  focus,
  getCurrentValue,
});
</script>

<style scoped>
.codemirror-notebook-editor {
  width: 100%;
  /* No fixed height - editor will auto-size to content */
}

/* Override CodeMirror's default styling for better notebook integration */
.codemirror-notebook-editor :deep(.cm-editor) {
  background: transparent;
  border-radius: 4px;
}

/* CRITICAL: Override CodeMirror's inline color styles for Julia syntax highlighting */
/* CodeMirror uses inline styles like style="color: #D4D4D4" which we need to override */
.codemirror-notebook-editor :deep(.cm-line span[style*='color']) {
  /* We'll use attribute selectors to target specific color values and override them */
}

/* Override specific color values that CodeMirror applies inline */
/* This is a workaround since CodeMirror applies colors via inline styles */
.codemirror-notebook-editor :deep(.cm-line span) {
  /* Force our colors via CSS - this will override inline styles */
}

.codemirror-notebook-editor :deep(.cm-gutters) {
  background: rgba(255, 255, 255, 0.02);
  border-right: 1px solid rgba(255, 255, 255, 0.05);
}

.codemirror-notebook-editor :deep(.cm-activeLineGutter) {
  background: rgba(255, 255, 255, 0.05);
}

.codemirror-notebook-editor :deep(.cm-activeLine) {
  background: rgba(255, 255, 255, 0.02);
}

.codemirror-notebook-editor :deep(.cm-cursor) {
  border-left-color: #4a9eff;
}

.codemirror-notebook-editor :deep(.cm-selectionBackground) {
  background: rgba(74, 158, 255, 0.2) !important;
}

.codemirror-notebook-editor :deep(.cm-focused .cm-selectionBackground) {
  background: rgba(74, 158, 255, 0.3) !important;
}

.codemirror-notebook-editor :deep(.cm-placeholder) {
  color: rgba(255, 255, 255, 0.4);
  font-style: italic;
}
</style>
