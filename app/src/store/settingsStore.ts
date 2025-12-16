import { defineStore } from 'pinia';
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { debug, error } from '../utils/logger';

export interface AppSettings {
  editor_font_family: string | null;
  editor_font_size: number | null;
  terminal_font_family: string | null;
  terminal_font_size: number | null;
  editor_word_wrap: boolean | null;
  editor_tab_size: number | null;
  editor_line_numbers: boolean | null;
  editor_minimap: boolean | null;
  editor_color_scheme: string | null;
}

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<AppSettings>({
    editor_font_family: null,
    editor_font_size: null,
    terminal_font_family: null,
    terminal_font_size: null,
    editor_word_wrap: null,
    editor_tab_size: null,
    editor_line_numbers: null,
    editor_minimap: null,
    editor_color_scheme: null,
  });

  const isLoading = ref(false);
  const isSaving = ref(false);

  // Default values
  const defaultEditorFontFamily = '"Fira Code", "Consolas", "Monaco", monospace';
  const defaultEditorFontSize = 14;
  const defaultTerminalFontFamily = '"Consolas", "Monaco", "Courier New", monospace';
  const defaultTerminalFontSize = 13;
  const defaultWordWrap = false;
  const defaultTabSize = 4;
  const defaultLineNumbers = true;
  const defaultMinimap = true;
  const defaultColorScheme = 'vs-dark';

  // Getter functions that return defaults if setting is null
  const getEditorFontFamily = () => {
    return settings.value.editor_font_family ?? defaultEditorFontFamily;
  };

  const getEditorFontSize = () => {
    return settings.value.editor_font_size ?? defaultEditorFontSize;
  };

  const getTerminalFontFamily = () => {
    return settings.value.terminal_font_family ?? defaultTerminalFontFamily;
  };

  const getTerminalFontSize = () => {
    return settings.value.terminal_font_size ?? defaultTerminalFontSize;
  };

  const getEditorWordWrap = () => {
    return settings.value.editor_word_wrap ?? defaultWordWrap;
  };

  const getEditorTabSize = () => {
    return settings.value.editor_tab_size ?? defaultTabSize;
  };

  const getEditorLineNumbers = () => {
    return settings.value.editor_line_numbers ?? defaultLineNumbers;
  };

  const getEditorMinimap = () => {
    return settings.value.editor_minimap ?? defaultMinimap;
  };

  const getEditorColorScheme = () => {
    return settings.value.editor_color_scheme ?? defaultColorScheme;
  };

  async function loadSettings() {
    isLoading.value = true;
    try {
      debug('SettingsStore: Loading settings from backend');
      const result = await invoke('get_app_settings');

      if (result && typeof result === 'object') {
        settings.value = {
          editor_font_family: (result as any).editor_font_family ?? null,
          editor_font_size: (result as any).editor_font_size ?? null,
          terminal_font_family: (result as any).terminal_font_family ?? null,
          terminal_font_size: (result as any).terminal_font_size ?? null,
          editor_word_wrap: (result as any).editor_word_wrap ?? null,
          editor_tab_size: (result as any).editor_tab_size ?? null,
          editor_line_numbers: (result as any).editor_line_numbers ?? null,
          editor_minimap: (result as any).editor_minimap ?? null,
          editor_color_scheme: (result as any).editor_color_scheme ?? null,
        };
        debug('SettingsStore: Settings loaded successfully', settings.value);
      }
    } catch (err) {
      error('SettingsStore: Failed to load settings', err);
      throw err;
    } finally {
      isLoading.value = false;
    }
  }

  async function saveSettings(partial: Partial<AppSettings>) {
    isSaving.value = true;
    try {
      debug('SettingsStore: Saving settings', partial);

      // Build update object with only non-null values
      const update: Record<string, any> = {};
      if (partial.editor_font_family !== undefined) {
        update.editor_font_family = partial.editor_font_family;
      }
      if (partial.editor_font_size !== undefined) {
        update.editor_font_size = partial.editor_font_size;
      }
      if (partial.terminal_font_family !== undefined) {
        update.terminal_font_family = partial.terminal_font_family;
      }
      if (partial.terminal_font_size !== undefined) {
        update.terminal_font_size = partial.terminal_font_size;
      }
      if (partial.editor_word_wrap !== undefined) {
        update.editor_word_wrap = partial.editor_word_wrap;
      }
      if (partial.editor_tab_size !== undefined) {
        update.editor_tab_size = partial.editor_tab_size;
      }
      if (partial.editor_line_numbers !== undefined) {
        update.editor_line_numbers = partial.editor_line_numbers;
      }
      if (partial.editor_minimap !== undefined) {
        update.editor_minimap = partial.editor_minimap;
      }
      if (partial.editor_color_scheme !== undefined) {
        update.editor_color_scheme = partial.editor_color_scheme;
      }

      await invoke('set_app_settings', { settings: update });

      // Update local state
      Object.assign(settings.value, partial);

      debug('SettingsStore: Settings saved successfully');
    } catch (err) {
      error('SettingsStore: Failed to save settings', err);
      throw err;
    } finally {
      isSaving.value = false;
    }
  }

  function updateEditorFont(family: string | null, size: number | null) {
    return saveSettings({
      editor_font_family: family,
      editor_font_size: size,
    });
  }

  function updateTerminalFont(family: string | null, size: number | null) {
    return saveSettings({
      terminal_font_family: family,
      terminal_font_size: size,
    });
  }

  return {
    settings,
    isLoading,
    isSaving,
    loadSettings,
    saveSettings,
    updateEditorFont,
    updateTerminalFont,
    getEditorFontFamily,
    getEditorFontSize,
    getTerminalFontFamily,
    getTerminalFontSize,
    getEditorWordWrap,
    getEditorTabSize,
    getEditorLineNumbers,
    getEditorMinimap,
    getEditorColorScheme,
  };
});
