import * as monaco from 'monaco-editor';
import type { editor } from 'monaco-editor';
import { nextTick } from 'vue';
import { getMonacoJuliaThemeRules } from './juliaColorScheme';

/**
 * Ensures the 'dark-plus' theme is defined in Monaco Editor.
 * Safe to call multiple times - will not redefine if already exists.
 * Includes Julia syntax highlighting rules for consistent colors.
 */
export function ensureDarkPlusTheme(): void {
  try {
    monaco.editor.defineTheme('dark-plus', {
      base: 'vs-dark',
      inherit: true,
      rules: getMonacoJuliaThemeRules(),
      colors: {
        'editor.background': '#1E1E1E',
      },
    });
  } catch (e) {
    // Theme might already be defined, ignore the error
  }
}

/**
 * Normalizes a theme name, converting 'vs-dark' to 'dark-plus' if needed.
 * @param theme The theme name to normalize
 * @returns The normalized theme name
 */
export function normalizeThemeName(theme: string): string {
  return theme === 'vs-dark' ? 'dark-plus' : theme;
}

/**
 * Switches the Monaco Editor theme and forces a complete re-tokenization
 * to ensure syntax highlighting colors update correctly.
 *
 * @param editorInstance The Monaco editor instance
 * @param theme The theme name to switch to
 * @param preventTrigger Optional callback to temporarily prevent triggers during theme switch.
 *                       Callback receives a boolean: true to prevent triggers, false to restore.
 * @returns Promise that resolves when the theme switch is complete
 */
export async function switchEditorTheme(
  editorInstance: editor.IStandaloneCodeEditor,
  theme: string,
  preventTrigger?: (prevent: boolean) => void
): Promise<void> {
  const normalizedTheme = normalizeThemeName(theme);

  // Ensure dark-plus theme is defined
  ensureDarkPlusTheme();

  // Set the theme first
  monaco.editor.setTheme(normalizedTheme);

  // Wait for Monaco to apply the theme, then force re-tokenization
  const model = editorInstance.getModel();
  if (!model) {
    return;
  }

  // Use nextTick to ensure theme is fully applied before forcing re-tokenization
  await nextTick();

  // Force tokenization of all lines to update syntax highlighting colors
  // This needs to happen after the theme is applied
  model.forceTokenization(model.getLineCount());

  // Trigger layout update
  editorInstance.layout();

  // Additional force re-render by temporarily changing and restoring content
  await nextTick();

  if (model && editorInstance) {
    const currentValue = model.getValue();

    // Temporarily prevent triggers if callback provided
    if (preventTrigger) {
      // Note: We set it to true during content manipulation and restore to false
      // The caller should manage their own state if needed
      preventTrigger(true);
    }

    model.setValue(currentValue + ' ');

    await nextTick();

    if (model && editorInstance) {
      model.setValue(currentValue);

      // Restore trigger prevention
      if (preventTrigger) {
        preventTrigger(false);
      }

      // Force tokenization again after content restore
      setTimeout(() => {
        if (model) {
          model.forceTokenization(model.getLineCount());
        }
      }, 0);
    }
  }
}
