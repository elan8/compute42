import { describe, it, expect, beforeEach, vi } from 'vitest';
import { setActivePinia, createPinia } from 'pinia';
import { useSettingsStore } from './settingsStore';
import { invoke } from '@tauri-apps/api/core';

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('settingsStore', () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
  });

  describe('initialization', () => {
    it('should initialize with default null values', () => {
      const store = useSettingsStore();

      expect(store.settings.editor_font_family).toBeNull();
      expect(store.settings.editor_font_size).toBeNull();
      expect(store.isLoading).toBe(false);
      expect(store.isSaving).toBe(false);
    });
  });

  describe('getter functions', () => {
    it('should return default values when settings are null', () => {
      const store = useSettingsStore();

      expect(store.getEditorFontFamily()).toBe('"Fira Code", "Consolas", "Monaco", monospace');
      expect(store.getEditorFontSize()).toBe(14);
      expect(store.getTerminalFontFamily()).toBe('"Consolas", "Monaco", "Courier New", monospace');
      expect(store.getTerminalFontSize()).toBe(13);
      expect(store.getEditorWordWrap()).toBe(false);
      expect(store.getEditorTabSize()).toBe(4);
      expect(store.getEditorLineNumbers()).toBe(true);
      expect(store.getEditorMinimap()).toBe(true);
      expect(store.getEditorColorScheme()).toBe('vs-dark');
    });

    it('should return custom values when settings are set', () => {
      const store = useSettingsStore();
      store.settings.editor_font_family = 'Arial';
      store.settings.editor_font_size = 16;

      expect(store.getEditorFontFamily()).toBe('Arial');
      expect(store.getEditorFontSize()).toBe(16);
    });
  });

  describe('loadSettings', () => {
    it('should load settings from backend', async () => {
      const mockSettings = {
        editor_font_family: 'Arial',
        editor_font_size: 16,
        terminal_font_family: 'Courier',
        terminal_font_size: 12,
        editor_word_wrap: true,
        editor_tab_size: 2,
        editor_line_numbers: false,
        editor_minimap: false,
        editor_color_scheme: 'vs-light',
      };

      (invoke as any).mockResolvedValue(mockSettings);

      const store = useSettingsStore();
      await store.loadSettings();

      expect(store.settings.editor_font_family).toBe('Arial');
      expect(store.settings.editor_font_size).toBe(16);
      expect(store.isLoading).toBe(false);
    });

    it('should handle errors', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const store = useSettingsStore();
      await expect(store.loadSettings()).rejects.toThrow();
    });
  });

  describe('saveSettings', () => {
    it('should save settings to backend', async () => {
      (invoke as any).mockResolvedValue(undefined);

      const store = useSettingsStore();
      await store.saveSettings({
        editor_font_family: 'Arial',
        editor_font_size: 16,
      });

      expect(invoke).toHaveBeenCalledWith('set_app_settings', {
        settings: {
          editor_font_family: 'Arial',
          editor_font_size: 16,
        },
      });
      expect(store.settings.editor_font_family).toBe('Arial');
      expect(store.settings.editor_font_size).toBe(16);
      expect(store.isSaving).toBe(false);
    });

    it('should handle errors', async () => {
      (invoke as any).mockRejectedValue(new Error('Failed'));

      const store = useSettingsStore();
      await expect(
        store.saveSettings({
          editor_font_family: 'Arial',
        })
      ).rejects.toThrow();
    });
  });

  describe('updateEditorFont', () => {
    it('should update editor font settings', async () => {
      (invoke as any).mockResolvedValue(undefined);

      const store = useSettingsStore();
      await store.updateEditorFont('Arial', 16);

      expect(store.settings.editor_font_family).toBe('Arial');
      expect(store.settings.editor_font_size).toBe(16);
    });
  });

  describe('updateTerminalFont', () => {
    it('should update terminal font settings', async () => {
      (invoke as any).mockResolvedValue(undefined);

      const store = useSettingsStore();
      await store.updateTerminalFont('Courier', 12);

      expect(store.settings.terminal_font_family).toBe('Courier');
      expect(store.settings.terminal_font_size).toBe(12);
    });
  });
});


