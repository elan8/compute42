<template>
  <div class="settings-container">
    <n-card title="Application Settings" class="settings-card">
      <n-spin :show="isLoading">
        <n-space vertical :size="24">
          <!-- Editor Settings -->
          <n-card title="Editor Settings" size="small">
            <div class="editor-settings-layout">
              <div class="editor-settings-form">
                <n-form :model="localSettings" label-placement="left" label-width="160">
                  <n-form-item label="Font Family">
                    <n-select
                      v-model:value="localSettings.editor_font_family"
                      :options="fontFamilyOptions"
                      placeholder="Select font family"
                      filterable
                      @update:value="handleEditorFontFamilyChange"
                    />
                  </n-form-item>
                  <n-form-item label="Font Size">
                    <n-input-number
                      v-model:value="localSettings.editor_font_size"
                      :min="8"
                      :max="24"
                      :step="1"
                      style="width: 120px"
                      @update:value="debouncedSaveEditorFontSize"
                    />
                  </n-form-item>
                  <n-form-item label="Word Wrap">
                    <n-switch
                      :value="localSettings.editor_word_wrap ?? false"
                      @update:value="handleWordWrapChange"
                    />
                  </n-form-item>
                  <n-form-item label="Tab Size">
                    <n-input-number
                      v-model:value="localSettings.editor_tab_size"
                      :min="2"
                      :max="8"
                      :step="1"
                      style="width: 120px"
                      @update:value="debouncedSaveTabSize"
                    />
                  </n-form-item>
                  <n-form-item label="Line Numbers">
                    <n-switch
                      :value="localSettings.editor_line_numbers ?? true"
                      @update:value="handleLineNumbersChange"
                    />
                  </n-form-item>
                  <n-form-item label="Minimap">
                    <n-switch
                      :value="localSettings.editor_minimap ?? true"
                      @update:value="handleMinimapChange"
                    />
                  </n-form-item>
                  <n-form-item label="Color Scheme">
                    <n-select
                      v-model:value="localSettings.editor_color_scheme"
                      :options="colorSchemeOptions"
                      placeholder="Select color scheme"
                      @update:value="handleColorSchemeChange"
                    />
                  </n-form-item>
                </n-form>
              </div>
              <div class="editor-preview-container">
                <div class="editor-preview-label">Preview</div>
                <MonacoEditorPreview
                  :value="sampleCode"
                  language="julia"
                  :theme="previewTheme"
                  :font-family="previewFontFamily"
                  :font-size="previewFontSize"
                  :word-wrap="previewWordWrap"
                  :tab-size="previewTabSize"
                  :line-numbers="previewLineNumbers"
                  :minimap="previewMinimap"
                />
              </div>
            </div>
          </n-card>

          <!-- Terminal Settings -->
          <n-card title="Terminal Settings" size="small">
            <div class="terminal-settings-layout">
              <div class="terminal-settings-form">
                <n-form :model="localSettings" label-placement="left" label-width="160">
                  <n-form-item label="Font Family">
                    <n-select
                      v-model:value="localSettings.terminal_font_family"
                      :options="fontFamilyOptions"
                      placeholder="Select font family"
                      filterable
                      @update:value="handleTerminalFontFamilyChange"
                    />
                  </n-form-item>
                  <n-form-item label="Font Size">
                    <n-input-number
                      v-model:value="localSettings.terminal_font_size"
                      :min="8"
                      :max="24"
                      :step="1"
                      style="width: 120px"
                      @update:value="debouncedSaveTerminalFontSize"
                    />
                  </n-form-item>
                </n-form>
              </div>
              <div class="terminal-preview-container">
                <div class="terminal-preview-label">Preview</div>
                <TerminalPreview
                  :font-family="previewTerminalFontFamily"
                  :font-size="previewTerminalFontSize"
                />
              </div>
            </div>
          </n-card>
        </n-space>
      </n-spin>
    </n-card>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, watch, computed } from 'vue';
import { NCard, NForm, NFormItem, NSelect, NInputNumber, NSwitch, NSpace, NSpin } from 'naive-ui';
import { useSettingsStore } from '../store/settingsStore';
import { useMessage } from 'naive-ui';
import { debounce } from 'lodash-es';
import { invoke } from '@tauri-apps/api/core';
import MonacoEditorPreview from '../components/SettingsView/MonacoEditorPreview.vue';
import TerminalPreview from '../components/SettingsView/TerminalPreview.vue';

interface FontOption {
  label: string;
  value: string;
}

interface ColorSchemeOption {
  label: string;
  value: string;
}

const message = useMessage();
const settingsStore = useSettingsStore();

const isLoading = ref(false);

// Local reactive settings that mirror the store
const localSettings = ref({
  editor_font_family: null as string | null,
  editor_font_size: null as number | null,
  terminal_font_family: null as string | null,
  terminal_font_size: null as number | null,
  editor_word_wrap: null as boolean | null,
  editor_tab_size: null as number | null,
  editor_line_numbers: null as boolean | null,
  editor_minimap: null as boolean | null,
  editor_color_scheme: null as string | null,
});

// Font family options - loaded from system
const fontFamilyOptions = ref<FontOption[]>([]);

// Color scheme options
const colorSchemeOptions = ref<ColorSchemeOption[]>([
  { label: 'Dark (VS Code Dark)', value: 'vs-dark' },
  { label: 'Dark Plus (Custom)', value: 'dark-plus' },
  { label: 'Light', value: 'vs' },
  { label: 'High Contrast Dark', value: 'hc-black' },
]);

// Sample code for preview
const sampleCode = ref(`# Example Julia code
function fibonacci(n::Int)::Int
    if n <= 1
        return n
    end
    return fibonacci(n - 1) + fibonacci(n - 2)
end

# Calculate the first 10 Fibonacci numbers
results = [fibonacci(i) for i in 1:10]
println("Fibonacci sequence: ", results)

# Example with types and documentation
\"\"\"
    compute_sum(numbers::Vector{Int}) -> Int
    
Compute the sum of all numbers in the vector.
\"\"\"
function compute_sum(numbers::Vector{Int})::Int
    return sum(numbers)
end`);

// Computed preview values that react to local settings
const previewTheme = computed(() => {
  return localSettings.value.editor_color_scheme || settingsStore.getEditorColorScheme();
});

const previewFontFamily = computed(() => {
  return localSettings.value.editor_font_family || settingsStore.getEditorFontFamily();
});

const previewFontSize = computed(() => {
  return localSettings.value.editor_font_size || settingsStore.getEditorFontSize();
});

const previewWordWrap = computed(() => {
  return localSettings.value.editor_word_wrap ?? settingsStore.getEditorWordWrap();
});

const previewTabSize = computed(() => {
  return localSettings.value.editor_tab_size || settingsStore.getEditorTabSize();
});

const previewLineNumbers = computed(() => {
  return localSettings.value.editor_line_numbers ?? settingsStore.getEditorLineNumbers();
});

const previewMinimap = computed(() => {
  return localSettings.value.editor_minimap ?? settingsStore.getEditorMinimap();
});

const previewTerminalFontFamily = computed(() => {
  return localSettings.value.terminal_font_family || settingsStore.getTerminalFontFamily();
});

const previewTerminalFontSize = computed(() => {
  return localSettings.value.terminal_font_size || settingsStore.getTerminalFontSize();
});

// Watch store changes and update local settings
watch(
  () => settingsStore.settings,
  (newSettings) => {
    localSettings.value = { ...newSettings };
  },
  { deep: true }
);

// Load settings and fonts on mount
onMounted(async () => {
  isLoading.value = true;
  try {
    // Load available fonts from system
    try {
      const fonts = await invoke<Array<{ label: string; value: string }>>('get_available_fonts');
      fontFamilyOptions.value = fonts;
      console.log(`Loaded ${fonts.length} available monospace fonts from system`);
    } catch (err) {
      console.error('Failed to load fonts from system:', err);
      message.warning('Could not load system fonts, using fallback list');
      // Fallback to a basic list if font enumeration fails
      fontFamilyOptions.value = [
        { label: 'Consolas', value: '"Consolas", monospace' },
        { label: 'Courier New', value: '"Courier New", monospace' },
        { label: 'Monaco', value: '"Monaco", monospace' },
        { label: 'monospace', value: 'monospace' },
      ];
    }

    // Load user settings
    await settingsStore.loadSettings();
    localSettings.value = { ...settingsStore.settings };
  } catch (err) {
    message.error('Failed to load settings');
    console.error('Failed to load settings:', err);
  } finally {
    isLoading.value = false;
  }
});

// Handler functions for immediate saves (toggles)
async function handleEditorFontFamilyChange(value: string | null) {
  try {
    await settingsStore.saveSettings({ editor_font_family: value });
  } catch (err) {
    message.error('Failed to save editor font family');
    console.error('Failed to save editor font family:', err);
  }
}

async function handleTerminalFontFamilyChange(value: string | null) {
  try {
    await settingsStore.saveSettings({ terminal_font_family: value });
  } catch (err) {
    message.error('Failed to save terminal font family');
    console.error('Failed to save terminal font family:', err);
  }
}

async function handleWordWrapChange(value: boolean) {
  try {
    await settingsStore.saveSettings({ editor_word_wrap: value });
  } catch (err) {
    message.error('Failed to save word wrap setting');
    console.error('Failed to save word wrap:', err);
  }
}

async function handleLineNumbersChange(value: boolean) {
  try {
    await settingsStore.saveSettings({ editor_line_numbers: value });
  } catch (err) {
    message.error('Failed to save line numbers setting');
    console.error('Failed to save line numbers:', err);
  }
}

async function handleMinimapChange(value: boolean) {
  try {
    await settingsStore.saveSettings({ editor_minimap: value });
  } catch (err) {
    message.error('Failed to save minimap setting');
    console.error('Failed to save minimap:', err);
  }
}

async function handleColorSchemeChange(value: string | null) {
  try {
    await settingsStore.saveSettings({ editor_color_scheme: value });
    message.success('Color scheme updated');
  } catch (err) {
    message.error('Failed to save color scheme');
    console.error('Failed to save color scheme:', err);
  }
}

// Debounced handlers for number inputs
const debouncedSaveEditorFontSize = debounce(async (value: number | null) => {
  try {
    await settingsStore.saveSettings({ editor_font_size: value });
  } catch (err) {
    message.error('Failed to save editor font size');
    console.error('Failed to save editor font size:', err);
  }
}, 500);

const debouncedSaveTerminalFontSize = debounce(async (value: number | null) => {
  try {
    await settingsStore.saveSettings({ terminal_font_size: value });
  } catch (err) {
    message.error('Failed to save terminal font size');
    console.error('Failed to save terminal font size:', err);
  }
}, 500);

const debouncedSaveTabSize = debounce(async (value: number | null) => {
  try {
    await settingsStore.saveSettings({ editor_tab_size: value });
  } catch (err) {
    message.error('Failed to save tab size');
    console.error('Failed to save tab size:', err);
  }
}, 500);
</script>

<style scoped>
.settings-container {
  height: 100vh;
  max-height: 100vh;
  display: flex;
  flex-direction: column;
  padding: 1rem;
  background: var(--n-color);
  overflow-y: auto;
  box-sizing: border-box;
}

.settings-card {
  width: 100%;
  max-width: 100%;
  margin: 0;
}

.editor-settings-layout {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
  align-items: start;
}

.editor-settings-form {
  min-width: 0;
}

.editor-preview-container {
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.editor-preview-label {
  font-size: 14px;
  font-weight: 500;
  color: var(--n-text-color);
  margin-bottom: 8px;
}

.terminal-settings-layout {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
  align-items: start;
}

.terminal-settings-form {
  min-width: 0;
}

.terminal-preview-container {
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.terminal-preview-label {
  font-size: 14px;
  font-weight: 500;
  color: var(--n-text-color);
  margin-bottom: 8px;
}

/* Responsive Design */
@media (max-width: 1024px) {
  .editor-settings-layout,
  .terminal-settings-layout {
    grid-template-columns: 1fr;
    gap: 16px;
  }

  .editor-preview-container,
  .terminal-preview-container {
    order: -1; /* Show preview first on mobile */
  }
}

@media (max-width: 768px) {
  .settings-container {
    padding: 0.5rem;
  }

  .editor-settings-layout,
  .terminal-settings-layout {
    gap: 12px;
  }
}

@media (max-width: 480px) {
  .settings-container {
    padding: 0.25rem;
  }
}
</style>
