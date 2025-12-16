<template>
  <div ref="editorContainer" style="width: 100%; height: 100%"></div>
</template>

<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch, nextTick, watchEffect } from 'vue';
import * as monaco from 'monaco-editor';
import type { editor, IRange } from 'monaco-editor';
import { debounce } from 'lodash-es';
// Removed legacy types import; using generated/shared types where needed
import { lspService } from '../../services/lspService';
import { syntaxService } from '../../services/syntaxService';
import { error, debug, info, logObject } from '../../utils/logger';
import { JuliaLspFeatures } from './JuliaLspFeatures';
import { useSettingsStore } from '../../store/settingsStore';
import {
  ensureDarkPlusTheme,
  normalizeThemeName,
  switchEditorTheme,
} from '../../utils/monacoThemeUtils';
import { debugMonacoColors } from '../../utils/debugMonacoColors';

// Define props
const props = defineProps({
  value: {
    type: String,
    required: true,
  },
  filePath: {
    type: String,
    required: true,
  },
  currentProjectRoot: {
    type: String,
    default: null,
  },
  language: {
    type: String,
    default: 'plaintext',
  },
  readOnly: {
    type: Boolean,
    default: false,
  },
  theme: {
    type: String,
    default: 'vs-dark',
  },
  pendingNavigation: {
    type: Object as () => { filePath: string; range: IRange } | null,
    default: null,
  },
  disableMinimap: {
    type: Boolean,
    default: false,
  },
});

// Define emits
const emit = defineEmits([
  'update:value',
  'editorMounted',
  'contentChanged',
  'openFileAndNavigate',
  'navigationComplete',
  'findReferences',
  'gotoDefinition',
  'save',
]);

// Refs
const editorContainer = ref<HTMLElement | null>(null);
let editorInstance: editor.IStandaloneCodeEditor | null = null;
let juliaLspFeatures: JuliaLspFeatures | null = null;
let preventTrigger = false;
let currentTheme = 'dark-plus'; // Track current theme to detect changes

// Settings store
const settingsStore = useSettingsStore();

// Syntax diagnostics state
let syntaxDiagnostics: any[] = [];
let syntaxMarkerDecorations: string[] = [];

// Debug function to check JuliaLspFeatures mounting conditions (removed verbose logging)
const debugJuliaLspFeaturesConditions = () => {
  // Logging removed to reduce verbosity
};

// Syntax checking functions
const updateSyntaxMarkers = (diagnostics: any[]) => {
  if (!editorInstance) return;

  // Clear existing syntax markers
  syntaxMarkerDecorations = editorInstance.deltaDecorations(syntaxMarkerDecorations, []);

  if (diagnostics.length === 0) {
    // Clear Monaco markers when there are no diagnostics
    const model = editorInstance.getModel();
    if (model) {
      monaco.editor.setModelMarkers(model, 'syntax', []);
    }
    syntaxDiagnostics = [];
    return;
  }

  // Convert syntax diagnostics to Monaco markers
  const markers = diagnostics.map((diagnostic) => ({
    startLineNumber: diagnostic.range.start.line + 1, // Convert to 1-based
    startColumn: diagnostic.range.start.character + 1, // Convert to 1-based
    endLineNumber: diagnostic.range.end.line + 1, // Convert to 1-based
    endColumn: diagnostic.range.end.character + 1, // Convert to 1-based
    message: diagnostic.message,
    severity:
      diagnostic.severity === 1 ? monaco.MarkerSeverity.Error : monaco.MarkerSeverity.Warning,
    source: diagnostic.source || 'tree-sitter',
  }));

  // Set markers on the model
  const model = editorInstance.getModel();
  if (model) {
    monaco.editor.setModelMarkers(model, 'syntax', markers);
  }

  syntaxDiagnostics = diagnostics;
};

const checkSyntax = async (uri: string, content: string, immediate = false) => {
  if (props.language !== 'julia') return;

  try {
    const diagnostics = immediate
      ? await syntaxService.parseSyntaxImmediate(uri, content)
      : await syntaxService.parseSyntax(uri, content);

    updateSyntaxMarkers(diagnostics);
  } catch (err) {
    error(`MonacoEditorInstance: Syntax checking failed: ${err}`);
  }
};

// User-friendly syntax checking - waits for user to stop typing (1.5s delay)
// This prevents syntax errors from appearing while the user is actively typing
const debouncedSyntaxCheck = debounce(checkSyntax, 1500);

onMounted(async () => {
  // Load settings on mount
  try {
    await settingsStore.loadSettings();
  } catch (err) {
    // Settings load failed, using defaults
  }

  nextTick(async () => {
    if (editorContainer.value) {
      // Register TOML language support
      monaco.languages.register({ id: 'toml' });

      // Define TOML language configuration
      monaco.languages.setLanguageConfiguration('toml', {
        comments: {
          lineComment: '#',
        },
        brackets: [
          ['[', ']'],
          ['{', '}'],
        ],
        autoClosingPairs: [
          { open: '[', close: ']' },
          { open: '{', close: '}' },
          { open: '"', close: '"' },
          { open: "'", close: "'" },
        ],
        surroundingPairs: [
          { open: '[', close: ']' },
          { open: '{', close: '}' },
          { open: '"', close: '"' },
          { open: "'", close: "'" },
        ],
      });

      // Define TOML syntax highlighting
      monaco.languages.setMonarchTokensProvider('toml', {
        defaultToken: '',
        tokenPostfix: '.toml',

        keywords: ['true', 'false'],

        // The main tokenizer for our languages
        tokenizer: {
          root: [
            // Comments
            [/#.*$/, 'comment'],

            // Strings
            [/"/, 'string', '@string'],
            [/'/, 'string', '@string_single'],

            // Numbers
            [/\d+/, 'number'],
            [/\d+\.\d+/, 'number'],

            // Booleans
            [/\b(true|false)\b/, 'keyword'],

            // Dates
            [/\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z?/, 'number'],

            // Table headers (including nested ones like [deps])
            [/\[[^\]]+\]/, 'type'],

            // UUIDs (common in Julia Project.toml files)
            [/[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}/i, 'string'],

            // Version strings
            [/"\d+\.\d+\.\d+"/, 'string'],
            [/'[0-9a-f]{40}'/, 'string'],

            // Key-value pairs (keys)
            [/^[a-zA-Z_][a-zA-Z0-9_-]*\s*=/, 'variable'],
            [/^[a-zA-Z_][a-zA-Z0-9_-]*\s*\{/, 'variable'],

            // Assignment operator
            [/=/, 'operator'],

            // Braces for inline tables
            [/\{/, 'operator'],
            [/\}/, 'operator'],

            // Commas
            [/,/, 'operator'],

            // Whitespace
            [/\s+/, 'white'],
          ],

          string: [
            [/[^"]+/, 'string'],
            [/""/, 'string'],
            [/"/, 'string', '@pop'],
          ],

          string_single: [
            [/[^']+/, 'string'],
            [/''/, 'string'],
            [/'/, 'string', '@pop'],
          ],
        },
      });

      // Ensure dark-plus theme is defined
      ensureDarkPlusTheme();

      // Create model with file URI
      const modelUri = monaco.Uri.file(props.filePath);

      let currentModel: editor.ITextModel | null = null;

      try {
        let existingModel = monaco.editor.getModel(modelUri);
        if (existingModel) {
          if (existingModel.getValue() !== props.value) {
            // Add null/undefined check to prevent "Illegal argument" error
            const valueToSet = props.value ?? '';
            existingModel.setValue(valueToSet);
          }
          currentModel = existingModel;
        } else {
          // Add null/undefined check to prevent "Illegal argument" error
          const valueToSet = props.value ?? '';
          currentModel = monaco.editor.createModel(valueToSet, props.language, modelUri);
        }

        if (!currentModel) {
          error(
            `MonacoEditorInstance: Failed to create or find model for URI: ${modelUri.toString()}. Aborting editor creation.`
          );
          return;
        }

        // Get settings from store (with defaults)
        const editorFontFamily = settingsStore.getEditorFontFamily();
        const editorFontSize = settingsStore.getEditorFontSize();
        const editorWordWrap = settingsStore.getEditorWordWrap();
        const editorTabSize = settingsStore.getEditorTabSize();
        const editorLineNumbers = settingsStore.getEditorLineNumbers();
        const editorMinimap = props.disableMinimap ? false : settingsStore.getEditorMinimap();

        const initialTheme = normalizeThemeName(props.theme);
        currentTheme = initialTheme;
        editorInstance = monaco.editor.create(editorContainer.value, {
          model: currentModel,
          theme: initialTheme,
          readOnly: props.readOnly,
          automaticLayout: true,
          fontFamily: editorFontFamily,
          fontSize: editorFontSize,
          wordWrap: editorWordWrap ? 'on' : 'off',
          tabSize: editorTabSize,
          minimap: { enabled: editorMinimap },
          scrollbar: {
            verticalScrollbarSize: 10,
            horizontalScrollbarSize: 10,
          },
          // Make LSP completions take precedence and trigger on typical characters
          wordBasedSuggestions: false,
          quickSuggestions: {
            other: true,
            comments: false,
            strings: true,
          },
          suggestOnTriggerCharacters: true,
          inlayHints: {
            enabled: 'on',
          },
          hover: {
            enabled: true,
            delay: 300,
            sticky: false,
          },
          glyphMargin: false,
          lineNumbers: editorLineNumbers ? 'on' : 'off',
          folding: true,
        });

        // Note: Hover provider is handled by JuliaLspFeatures.vue

        // Initialize JuliaLspFeatures if language is Julia
        if (props.language === 'julia' && editorInstance) {
          juliaLspFeatures = new JuliaLspFeatures({
            editor: editorInstance,
            language: props.language,
            projectRoot: props.currentProjectRoot,
          });

          // Expose debug function globally for console access
          (window as any).debugMonacoColors = async () => {
            if (editorInstance) {
              return await debugMonacoColors(editorInstance);
            }
            await info('Monaco editor instance not available');
            return null;
          };

          // Automatically extract and log Monaco colors when opening a Julia file
          // Wait a bit for the editor to fully render before extracting colors
          setTimeout(async () => {
            try {
              await debugMonacoColors(editorInstance);
            } catch (err) {
              await logObject('error', 'MonacoEditorInstance: Failed to extract Monaco colors', {
                error: String(err),
              });
            }
          }, 500); // Wait 500ms for editor to render
        }

        // Notify LSP that the document is open (send Monaco file URI to match future requests)
        if (props.language === 'julia') {
          const uriString = modelUri.toString();
          try {
            await lspService.notifyDidOpen(uriString, props.value ?? '', 'julia');
          } catch (err) {
            await logObject('error', '[LSP DEBUG] Failed to notify LSP of document open', {
              uri: uriString,
              error: String(err),
            });
          }

          // Initial syntax check - immediate parsing when opening a file
          // This ensures users see syntax errors as soon as they open a file
          try {
            await checkSyntax(uriString, props.value ?? '', true);
          } catch (err) {
            error(`MonacoEditorInstance: Initial syntax check failed: ${err}`);
          }
        }

        // Listen for content changes
        editorInstance.onDidChangeModelContent(() => {
          const editorModel = editorInstance?.getModel();
          if (!preventTrigger && editorModel?.uri.toString() === modelUri.toString()) {
            const currentContent = editorModel.getValue();
            emit('update:value', currentContent);
            emit('contentChanged');

            if (props.language === 'julia') {
              // Mark that user is typing
              syntaxService.markTyping(modelUri.toString());

              // LSP content change notification (if function exists)
              if (typeof debouncedLspDidChange === 'function') {
                debouncedLspDidChange(modelUri.toString(), currentContent);
              }

              // User-friendly syntax checking - waits for user to stop typing
              debouncedSyntaxCheck(modelUri.toString(), currentContent);
            }
          }
        });

        // Listen for cursor position changes to detect when user stops typing
        editorInstance.onDidChangeCursorPosition(() => {
          if (props.language === 'julia') {
            // Mark that user has stopped typing when they move the cursor
            syntaxService.markStoppedTyping(modelUri.toString());
          }
        });

        // Listen for focus changes to detect when user stops typing
        editorInstance.onDidBlurEditorText(() => {
          if (props.language === 'julia') {
            // Mark that user has stopped typing when they lose focus
            syntaxService.markStoppedTyping(modelUri.toString());
          }
        });

        // Listen for focus changes to detect when user starts typing again
        editorInstance.onDidFocusEditorText(() => {
          if (props.language === 'julia') {
            // Mark that user might start typing again when they regain focus
            // This will be overridden by the next content change
            syntaxService.markStoppedTyping(modelUri.toString());
          }
        });

        // Add save handler for immediate syntax checking
        editorInstance.addAction({
          id: 'save-file',
          label: 'Save File',
          keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
          run: async (editor) => {
            const model = editor.getModel();
            if (!model) return;

            const content = model.getValue();
            const uri = model.uri.toString();

            // Emit save event to parent component to handle actual file saving
            emit('save');

            // For Julia files, also do immediate syntax check and LSP notification
            if (props.language === 'julia') {
              // Immediate syntax check on save
              await checkSyntax(uri, content, true);

              // Notify LSP of save (if function exists)
              if (typeof lspService.notifyDidSave === 'function') {
                try {
                  await lspService.notifyDidSave(uri);
                } catch (err) {
                  error(`MonacoEditorInstance: Failed to notify LSP of save: ${err}`);
                }
              }
            }
          },
        });

        emit('editorMounted', editorInstance);

        // Ensure LSP client is initialized
        // ensureLspClientInitialized not required; LSP lifecycle managed by backend

        // Note: Inlay hints provider is handled by JuliaLspFeatures.vue

        // Add "Go to Definition" action for Julia
        editorInstance.addAction({
          id: 'julia-goto-definition',
          label: 'Go to Definition',
          keybindings: [monaco.KeyCode.F12],
          contextMenuGroupId: 'navigation',
          contextMenuOrder: 1.0,
          run: async (editor) => {
            const position = editor.getPosition();
            const model = editor.getModel();
            if (position && model) {
              // Emit event to parent to handle definition navigation
              emit('gotoDefinition', {
                uri: model.uri.toString(),
                line: position.lineNumber - 1, // Convert to 0-based for LSP
                character: position.column - 1, // Convert to 0-based for LSP
              });
            } else {
              await logObject(
                'warn',
                'MonacoEditorInstance: Go to Definition failed - missing position or model',
                {
                  hasPosition: !!position,
                  hasModel: !!model,
                  position: position,
                  modelUri: model?.uri.toString(),
                }
              );
            }
          },
        });

        // Add "Find All References" action for Julia
        editorInstance.addAction({
          id: 'julia-find-references',
          label: 'Find All References',
          keybindings: [monaco.KeyMod.Shift | monaco.KeyCode.F12],
          contextMenuGroupId: 'navigation',
          contextMenuOrder: 2.0,
          run: async (editor) => {
            const position = editor.getPosition();
            const model = editor.getModel();
            if (position && model) {
              // Emit event to parent to show references panel
              emit('findReferences', {
                uri: model.uri.toString(),
                line: position.lineNumber - 1, // Convert to 0-based for LSP
                character: position.column - 1, // Convert to 0-based for LSP
              });
            }
          },
        });
      } catch (err) {
        error(`MonacoEditorInstance: Error creating editor for ${props.filePath}: ${err}`);
      }
    }
  });
});

// Debounced LSP didChange notification
const debouncedLspDidChange = debounce(async (filePath, content) => {
  if (props.currentProjectRoot) {
    try {
      await lspService.notifyDidChange(filePath, content);
    } catch (err) {
      // LSP notification failed
    }
  }
}, 500);

// LSP client initialization
const initializeLspClient = async (projectRoot: string): Promise<boolean> => {
  if (!projectRoot) {
    return false;
  }

  // LSP is now managed by the backend during project activation
  // No need to initialize here - just check if it's available
  try {
    const isRunning = await lspService.isRunning();
    return isRunning;
  } catch (err) {
    error(`MonacoEditorInstance: Failed to check LSP status: ${err}`);
    return false;
  }
};

// Update editor content if the prop changes from outside
watch(
  () => props.value,
  (newValue) => {
    if (editorInstance && newValue !== editorInstance.getValue()) {
      try {
        const model = editorInstance.getModel();
        if (model) {
          preventTrigger = true;
          // Add null/undefined check to prevent "Illegal argument" error
          const valueToSet = newValue ?? '';
          editorInstance.setValue(valueToSet);
          preventTrigger = false;
        }
      } catch (err) {
        error(`MonacoEditorInstance: Error setting value for ${props.filePath}: ${err}`);
        preventTrigger = false;
      }
    }
  }
);

// Update language if the prop changes
watch(
  () => props.language,
  async (newLanguage) => {
    if (editorInstance && newLanguage !== editorInstance.getModel()?.getLanguageId()) {
      monaco.editor.setModelLanguage(editorInstance.getModel()!, newLanguage);

      // Automatically extract colors when switching to Julia
      if (newLanguage === 'julia') {
        setTimeout(async () => {
          try {
            await debugMonacoColors(editorInstance);
          } catch (err) {
            await logObject(
              'error',
              'MonacoEditorInstance: Failed to extract Monaco colors after language switch',
              { error: String(err) }
            );
          }
        }, 500); // Wait 500ms for editor to render
      }
    }
  }
);

// Update readOnly status if the prop changes
watch(
  () => props.readOnly,
  (newReadOnly) => {
    if (editorInstance) {
      editorInstance.updateOptions({ readOnly: newReadOnly });
    }
  }
);

// Update theme if the prop changes
watch(
  () => props.theme,
  async (newTheme) => {
    if (editorInstance) {
      const themeToUse = normalizeThemeName(newTheme);

      // Only update if theme actually changed
      if (themeToUse !== currentTheme) {
        currentTheme = themeToUse;

        await switchEditorTheme(editorInstance, newTheme, (prevent) => {
          preventTrigger = prevent;
        });
      }
    }
  }
);

// Watch for project root changes to check LSP status
watch(
  () => props.currentProjectRoot,
  async (newProjectRoot, oldProjectRoot) => {
    if (newProjectRoot && newProjectRoot !== oldProjectRoot && props.language === 'julia') {
      try {
        await initializeLspClient(newProjectRoot);
      } catch (err) {
        // LSP status check failed
      }
    }
  }
);

// Watch for JuliaLspFeatures mounting conditions
watch(
  [() => editorInstance, () => props.language],
  ([newEditorInstance, newLanguage]) => {
    // Update JuliaLspFeatures when conditions change
    if (newEditorInstance && newLanguage === 'julia' && !juliaLspFeatures) {
      juliaLspFeatures = new JuliaLspFeatures({
        editor: newEditorInstance,
        language: newLanguage,
        projectRoot: props.currentProjectRoot,
      });
    } else if (juliaLspFeatures) {
      juliaLspFeatures.updateLanguage(newLanguage);
    }
  },
  { immediate: true }
);

// Watch for project root changes
watch(
  () => props.currentProjectRoot,
  (newProjectRoot) => {
    if (juliaLspFeatures) {
      juliaLspFeatures.updateProjectRoot(newProjectRoot);
    }
  }
);

// Cleanup on unmount
onBeforeUnmount(() => {
  if (juliaLspFeatures) {
    juliaLspFeatures.dispose();
    juliaLspFeatures = null;
  }
  if (editorInstance) {
    editorInstance.dispose();
    editorInstance = null;
  }
  debouncedLspDidChange.cancel();
});

// Watch for prop changes to understand when component is being recreated
watch(
  () => props.filePath,
  (newPath, oldPath) => {
    logObject('info', `MonacoEditorInstance: filePath prop changed for ${props.filePath}`, {
      oldPath,
      newPath,
      hasEditorInstance: !!editorInstance,
    });
  }
);

watch(
  () => props.value,
  (newValue, oldValue) => {
    logObject('info', `MonacoEditorInstance: value prop changed for ${props.filePath}`, {
      oldValueLength: oldValue?.length,
      newValueLength: newValue?.length,
      hasEditorInstance: !!editorInstance,
    });
  }
);

watch(
  () => props.language,
  (newLang, oldLang) => {
    logObject('info', `MonacoEditorInstance: language prop changed for ${props.filePath}`, {
      oldLanguage: oldLang,
      newLanguage: newLang,
      hasEditorInstance: !!editorInstance,
    });
  }
);

watch(
  () => props.pendingNavigation,
  (newNav, oldNav) => {
    logObject(
      'info',
      `MonacoEditorInstance: pendingNavigation prop changed for ${props.filePath}`,
      {
        oldNavigation: oldNav,
        newNavigation: newNav,
        hasEditorInstance: !!editorInstance,
      }
    );
  }
);

// Watch for settings changes and update editor
watch(
  () => [
    settingsStore.getEditorFontFamily(),
    settingsStore.getEditorFontSize(),
    settingsStore.getEditorWordWrap(),
    settingsStore.getEditorTabSize(),
    settingsStore.getEditorLineNumbers(),
    settingsStore.getEditorMinimap(),
    settingsStore.getEditorColorScheme(),
  ],
  async () => {
    if (editorInstance) {
      const colorScheme = settingsStore.getEditorColorScheme();
      const themeToUse = normalizeThemeName(colorScheme);

      // Only update if theme actually changed
      if (themeToUse !== currentTheme) {
        currentTheme = themeToUse;

        await switchEditorTheme(editorInstance, colorScheme, (prevent) => {
          preventTrigger = prevent;
        });
        // Note: switchEditorTheme will set preventTrigger back to false
      }

      editorInstance.updateOptions({
        fontFamily: settingsStore.getEditorFontFamily(),
        fontSize: settingsStore.getEditorFontSize(),
        wordWrap: settingsStore.getEditorWordWrap() ? 'on' : 'off',
        tabSize: settingsStore.getEditorTabSize(),
        lineNumbers: settingsStore.getEditorLineNumbers() ? 'on' : 'off',
        minimap: { enabled: props.disableMinimap ? false : settingsStore.getEditorMinimap() },
      });
      debug('MonacoEditorInstance: Editor options updated from settings');
    }
  }
);

// Helper function to normalize paths consistently
const normalizePath = (path: string): string => {
  return path.toLowerCase().replace(/[\/\\]/g, '/');
};

// Watch for pending navigation requests
watchEffect(async () => {
  if (props.pendingNavigation && editorInstance) {
    const model = editorInstance.getModel();
    if (model) {
      const modelPath = normalizePath(model.uri.fsPath);
      const pendingPath = normalizePath(props.pendingNavigation.filePath);

      const pathsMatch = modelPath === pendingPath;

      if (!pathsMatch) {
        await logObject('warn', 'MonacoEditorInstance: Paths do not match, skipping navigation', {
          modelPath: modelPath,
          pendingPath: pendingPath,
          difference: {
            modelLength: modelPath.length,
            pendingLength: pendingPath.length,
            modelStartsWithPending: modelPath.startsWith(pendingPath),
            pendingStartsWithModel: pendingPath.startsWith(modelPath),
          },
        });
        return;
      }

      const rangeToNavigate = { ...props.pendingNavigation.range } as IRange;

      // Validate line numbers
      if (rangeToNavigate.startLineNumber > model.getLineCount()) {
        await logObject(
          'warn',
          'MonacoEditorInstance: Start line number exceeds model line count, adjusting',
          {
            requestedLine: rangeToNavigate.startLineNumber,
            modelLineCount: model.getLineCount(),
          }
        );
        // Do not mutate props; compute a safe start line
        const safeStart = model.getLineCount();
        editorInstance.setPosition({ lineNumber: safeStart, column: 1 });
      }

      if (rangeToNavigate.endLineNumber > model.getLineCount()) {
        await logObject(
          'warn',
          'MonacoEditorInstance: End line number exceeds model line count, adjusting',
          {
            requestedLine: rangeToNavigate.endLineNumber,
            modelLineCount: model.getLineCount(),
          }
        );
        // Do not mutate props; compute a safe end line
        const safeEnd = model.getLineCount();
        editorInstance.revealLineInCenter(safeEnd);
      }

      editorInstance.revealLineInCenter(rangeToNavigate.startLineNumber);

      editorInstance.setPosition({
        lineNumber: rangeToNavigate.startLineNumber,
        column: rangeToNavigate.startColumn || 1,
      });

      editorInstance.setSelection({
        startLineNumber: rangeToNavigate.startLineNumber,
        startColumn: 1,
        endLineNumber: rangeToNavigate.startLineNumber,
        endColumn: model.getLineMaxColumn(rangeToNavigate.startLineNumber),
      });

      editorInstance.focus();

      emit('navigationComplete');
    } else {
      await logObject('warn', 'MonacoEditorInstance: Cannot navigate - no model available', {
        hasEditorInstance: !!editorInstance,
        hasModel: !!model,
        pendingNavigation: props.pendingNavigation,
      });
    }
  } else {
    if (props.pendingNavigation && !editorInstance) {
      await logObject(
        'warn',
        'MonacoEditorInstance: Cannot navigate - no editor instance available',
        {
          hasEditorInstance: !!editorInstance,
          pendingNavigation: props.pendingNavigation,
        }
      );
    }
  }
});

// Cleanup on unmount
onBeforeUnmount(() => {
  if (editorInstance) {
    // Clear syntax markers
    const model = editorInstance.getModel();
    if (model) {
      monaco.editor.setModelMarkers(model, 'syntax', []);
    }
  }

  // Clear syntax service cache
  if (props.language === 'julia' && props.filePath) {
    syntaxService.clearCache(props.filePath).catch((err) => {
      error(`MonacoEditorInstance: Failed to clear syntax cache: ${err}`);
    });
  }
});

// Expose methods
defineExpose({
  focus: () => editorInstance?.focus(),
  getCurrentValue: () => editorInstance?.getValue(),
  // Debug function to extract Monaco colors
  debugColors: () => {
    if (editorInstance) {
      return debugMonacoColors(editorInstance);
    }
    return null;
  },
});

// Helper function to map LSP completion item kinds to Monaco kinds
// Removed unused function
</script>

<style scoped>
/* Removed min-height rule */
</style>

<style>
/* Monaco editor global styles */
</style>
