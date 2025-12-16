<template>
  <div class="variables-panel">
    <div class="panel-content">
      <div v-if="Object.keys(debugVariables).length === 0" class="no-variables">
        <n-icon size="32" color="#888"><InformationCircleOutline /></n-icon>
        <p v-if="isDebugging">No variables available</p>
        <p v-else>No workspace variables</p>
        <p class="hint" v-if="isDebugging">
          Variables will appear when execution pauses at a breakpoint
        </p>
        <p class="hint" v-else>
          Run a Julia script or Jupyter notebook cell to see workspace variables
        </p>
      </div>

      <div v-else class="variables-list">
        <div
          v-for="(variable, name) in debugVariables"
          :key="name"
          class="variable-item"
          :class="{ expandable: variable.is_expandable }"
          @click="variable.is_expandable ? showVariableModal(name, variable) : null"
        >
          <div class="variable-header">
            <span class="variable-name">{{ name }}</span>
            <span class="variable-type">{{ variable.type || 'Unknown' }}</span>
          </div>
          <div class="variable-value">
            {{ getVariableDisplayValue(variable) }}
            <span v-if="variable.is_expandable" class="expand-icon">🔍</span>
          </div>
        </div>
      </div>
    </div>
  </div>

  <!-- Variable Details Modal -->
  <n-modal
    v-model:show="showModal"
    preset="card"
    :title="`Variable: ${selectedVariable?.name}`"
    style="width: 95%; max-width: 1400px; max-height: 95vh"
  >
    <div v-if="selectedVariable" class="modal-content">
      <!-- Metadata in horizontal layout -->
      <div class="modal-metadata">
        <div class="metadata-item">
          <strong>Type:</strong> <span>{{ selectedVariable.type }}</span>
        </div>
        <div v-if="selectedVariable.dimensions" class="metadata-item">
          <strong>Size:</strong>
          <span>{{ selectedVariable.dimensions }}</span>
        </div>
        <!-- DataFrame-specific metadata -->
        <div
          v-if="selectedVariable.is_dataframe && selectedVariable.parsed_data"
          class="metadata-item"
        >
          <strong>Rows:</strong>
          <span>{{ selectedVariable.parsed_data.length }}</span>
        </div>
        <div
          v-if="selectedVariable.is_dataframe && getTableColumns(selectedVariable).length > 0"
          class="metadata-item"
        >
          <strong>Columns:</strong>
          <span>{{ getTableColumns(selectedVariable).length }}</span>
        </div>
        <div
          v-if="selectedVariable.is_dataframe && selectedVariable.column_names"
          class="metadata-item"
        >
          <strong>Column Names:</strong>
          <span>{{ selectedVariable.column_names.join(', ') }}</span>
        </div>
      </div>

      <!-- Value section with more space -->

      <!-- Loading indicator -->
      <div v-if="selectedVariable.loading" class="loading-indicator">
        <n-spin size="small" />
        <span style="margin-left: 8px">Loading full value...</span>
      </div>

      <template v-else>
        <div v-if="isTruncated(selectedVariable.value)" class="truncation-warning">
          ⚠️ Large variable - showing first 10,000 characters. Full pagination support coming soon!
        </div>

        <!-- Display table for arrays/matrices -->
        <div
          v-if="isArrayData(selectedVariable)"
          class="table-container"
          :class="{ 'hide-headers': !isStructuredData(selectedVariable) }"
        >
          <table class="data-table">
            <thead v-if="isStructuredData(selectedVariable)">
              <tr>
                <th v-for="column in getTableColumns(selectedVariable)" :key="column.key">
                  {{ column.title }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="(row, index) in getTableData(selectedVariable)"
                :key="index"
                :class="{ even: index % 2 === 0 }"
              >
                <td
                  v-for="column in getTableColumns(selectedVariable)"
                  :key="column.key"
                  :title="row[column.key]"
                >
                  {{ row[column.key] }}
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Display raw value for non-array data -->
        <pre v-else class="modal-value">{{ selectedVariable.value || 'N/A' }}</pre>
      </template>
    </div>
  </n-modal>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, nextTick, computed } from 'vue';
import { NIcon, NModal, NSpin } from 'naive-ui';
import { InformationCircleOutline } from '@vicons/ionicons5';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { debug as debugLog, error, info, warn } from '../../utils/logger';
import { useAppStore } from '../../store/appStore';

// Props
const props = defineProps<{
  isDebugging: boolean;
}>();

// Access the app store for persisted workspace variables
const appStore = useAppStore();

// State - local ref for debug mode variables (from debug:execution-stopped events)
const debugVariablesLocal = ref<Record<string, any>>({});
const showModal = ref(false);
const selectedVariable = ref<any>(null);

// Computed property that always returns the correct variables source
// In debug mode: use debugVariablesLocal (from debug events)
// In normal mode: use appStore.workspaceVariables (persisted, won't be cleared by spurious events)
const debugVariables = computed(() => {
  const result = props.isDebugging ? debugVariablesLocal.value : appStore.workspaceVariables;
  const count = Object.keys(result).length;
  // Only log if count changes significantly to avoid spam
  // (computed properties can be called many times)
  return result;
});

// Helper function to format variable values
const getVariableDisplayValue = (variable: any): string => {
  // If we have a summary (from events or full data), use it
  if (variable.summary) {
    return variable.summary;
  }

  // Handle lightweight event data (has is_array directly)
  if (variable.is_array) {
    const count = variable.element_count || 0;
    return `[${count} elements]`;
  }

  if (variable.is_dict) {
    return '{...}';
  }

  if (variable.is_struct) {
    const typeName = variable.type || 'Unknown';
    return `${typeName}{...}`;
  }

  // Handle full variable data (has var_type object)
  if (variable.var_type?.is_array) {
    const count = variable.var_type.element_count || 0;
    return `[${count} elements]`;
  }

  if (variable.var_type?.is_dict) {
    return '{...}';
  }

  if (variable.var_type?.is_struct) {
    return `${variable.var_type.name}{...}`;
  }

  return variable.value || '[Empty]';
};

// Show variable details in modal
const showVariableModal = async (name: string, variable: any) => {
  // Store current variables count before opening modal (for debugging)
  const variablesCountBefore = Object.keys(debugVariables.value).length;
  const storeCountBefore = Object.keys(appStore.workspaceVariables).length;
  const localCountBefore = Object.keys(debugVariablesLocal.value).length;

  // Special debugging for DataFrames
  // Also check type field as fallback if is_dataframe flag is not set
  const isActuallyDataFrame =
    variable.is_dataframe || (variable.type && variable.type.includes('DataFrame'));
  if (isActuallyDataFrame) {
    // Ensure is_dataframe flag is set
    if (!variable.is_dataframe) {
      variable.is_dataframe = true;
    }
  }

  // Set initial data
  selectedVariable.value = { name, ...variable };
  showModal.value = true;

  // Check immediately after setting modal
  await nextTick();
  const variablesCountAfterModal = Object.keys(debugVariables.value).length;
  const storeCountAfterModal = Object.keys(appStore.workspaceVariables).length;

  if (variablesCountBefore > 0 && variablesCountAfterModal === 0) {
    await error('⚠️ VARIABLES WERE CLEARED AFTER MODAL OPEN!');
    await error(
      `Before: ${variablesCountBefore}, After: ${variablesCountAfterModal}, Store still has: ${storeCountAfterModal}`
    );
  }

  // Note: In normal mode, debugVariables is computed from appStore.workspaceVariables,
  // so it will automatically reflect the store. No need to manually restore.

  // If variable needs fetch (large variable), or if it's a DataFrame without parsed_data, fetch the full value
  const needsFetch = variable.needs_fetch && !variable.value;
  const isDataFrameNeedingFetch = variable.is_dataframe && !variable.parsed_data && !variable.value;

  if (needsFetch || isDataFrameNeedingFetch) {
    selectedVariable.value = { ...selectedVariable.value, loading: true };

    try {
      const variablesCountBeforeInvoke = Object.keys(debugVariables.value).length;
      const storeCountBeforeInvoke = Object.keys(appStore.workspaceVariables).length;

      const fullValue = await invoke('get_variable_value', { variableName: name });

      const variablesCountAfterInvoke = Object.keys(debugVariables.value).length;
      const storeCountAfterInvoke = Object.keys(appStore.workspaceVariables).length;

      if (variablesCountBeforeInvoke > 0 && variablesCountAfterInvoke === 0) {
        await error('⚠️ VARIABLES WERE CLEARED AFTER INVOKE!');
        await error(
          `Before invoke: ${variablesCountBeforeInvoke}, After invoke: ${variablesCountAfterInvoke}, Store still has: ${storeCountAfterInvoke}`
        );
      }

      // If this is a DataFrame, process the value to extract parsed_data
      let processedVariable: any = {
        ...selectedVariable.value,
        value: fullValue || 'Unable to fetch value',
        loading: false,
      };

      // For DataFrames, check if the returned value is already parsed JSON (parsed_data)
      if (variable.is_dataframe && fullValue) {
        try {
          // Try to parse as JSON - if it's an array, it's likely parsed_data
          const parsed = JSON.parse(fullValue);
          if (Array.isArray(parsed)) {
            processedVariable.parsed_data = parsed;
            processedVariable.value = fullValue; // Keep the JSON string as value too
          } else if (typeof parsed === 'object' && parsed.parsed_data) {
            // If it's an object with parsed_data field
            processedVariable.parsed_data = parsed.parsed_data;
          }
          // Otherwise it's a DataFrame string representation, will be parsed by the table display logic
        } catch (e) {
          // Not JSON, it's a string representation - that's fine, the table display will handle it
        }
      }

      selectedVariable.value = processedVariable;
    } catch (err) {
      error(`VariablesPanel: Failed to fetch variable value: ${err}`);
      selectedVariable.value = {
        ...selectedVariable.value,
        value: 'Error loading value',
        loading: false,
      };
    }
  }
};

const isTruncated = (value: string) => {
  return value && value.includes('[Truncated - showing first');
};

// Check if variable is a DataFrame or structured data with named columns
const isStructuredData = (variable: any): boolean => {
  if (!variable) return false;

  // Check if it's a DataFrame (check flag or type as fallback)
  if (variable.is_dataframe || (variable.type && variable.type.includes('DataFrame'))) return true;

  // Check if it's a DataFrame or similar structured type
  const type = variable.type || variable.var_type?.name || '';
  return type.includes('Table') || type.includes('Dict');
};

// Check if variable data should be displayed as a table
const isArrayData = (variable: any): boolean => {
  if (!variable) return false;

  // DataFrames should be displayed as tables (check flag or type as fallback)
  if (variable.is_dataframe || (variable.type && variable.type.includes('DataFrame'))) return true;

  // Check if it's an array type
  if (variable.is_array || (variable.var_type && variable.var_type.is_array)) {
    return true;
  }

  // Also check if the value looks like an array
  const value = variable.value || '';
  return value.startsWith('[') && value.includes(',');
};

// Parse array/matrix data and generate table columns
const getTableColumns = (variable: any): Array<{ key: string; title: string; width?: number }> => {
  const value = variable.value || '';
  const isStructured = isStructuredData(variable);

  // Handle DataFrames with parsed data
  if (
    variable.is_dataframe &&
    variable.parsed_data &&
    Array.isArray(variable.parsed_data) &&
    variable.parsed_data.length > 0
  ) {
    const firstRow = variable.parsed_data[0];
    const columns: Array<{ key: string; title: string; width?: number }> = [];

    // DataFrame first row logged via debugLog if needed

    // Add columns based on the parsed data keys
    // Use column_names if available (from backend), otherwise use keys from first row
    const columnNames = variable.column_names || Object.keys(firstRow);
    columnNames.forEach((key) => {
      // Calculate appropriate width based on column name length
      const minWidth = 100;
      const maxWidth = 300;
      const width = Math.max(minWidth, Math.min(maxWidth, key.length * 8 + 40));

      columns.push({
        title: key,
        key: key,
        width: width,
      });
    });

    // DataFrame columns logged via debugLog if needed
    return columns;
  }

  // Handle DataFrames without parsed_data - check if value is already parsed JSON
  if (variable.is_dataframe) {
    // DataFrame without parsed_data, checking if value is JSON

    // Check if the value is already a JSON array (parsed DataFrame data)
    if (typeof variable.value === 'string') {
      try {
        const parsed = JSON.parse(variable.value);
        if (Array.isArray(parsed) && parsed.length > 0) {
          // DataFrame value is JSON array, generating columns from first row
          const firstRow = parsed[0];
          const columns: Array<{ key: string; title: string; width?: number }> = [];

          // Add columns based on the parsed data keys
          // Use column_names if available (from backend), otherwise use keys from first row
          const columnNames = variable.column_names || Object.keys(firstRow);
          columnNames.forEach((key) => {
            // Calculate appropriate width based on column name length
            const minWidth = 100;
            const maxWidth = 300;
            const width = Math.max(minWidth, Math.min(maxWidth, key.length * 8 + 40));

            columns.push({
              title: key,
              key: key,
              width: width,
            });
          });

          // Generated DataFrame columns from value
          return columns;
        }
      } catch (e) {
        // DataFrame value is not JSON, ignoring
      }
    } else if (Array.isArray(variable.value) && variable.value.length > 0) {
      // DataFrame value is already an array, generating columns from first row
      const firstRow = variable.value[0];
      const columns: Array<{ key: string; title: string; width?: number }> = [];

      // Add columns based on the parsed data keys
      Object.keys(firstRow).forEach((key) => {
        columns.push({
          title: key,
          key: key,
          width: 150,
        });
      });

      // Generated DataFrame columns from value array
      return columns;
    }

    return [];
  }

  // Try to parse as a Julia array/matrix
  try {
    // Simple 1D array/vector
    if (value.match(/^\[.*\]$/s) && !value.includes(';')) {
      const columns: Array<{ key: string; title: string; width?: number }> = [];

      // Only add index column for structured data
      if (isStructured) {
        columns.push({ title: 'Index', key: 'index', width: 60 });
      }

      columns.push({
        title: isStructured ? 'Value' : '',
        key: 'value',
      });
      return columns;
    }

    // 2D matrix
    const rows = value.split(';').map((r: string) => r.trim());
    if (rows.length > 1) {
      const firstRow = rows[0]
        .replace('[', '')
        .split(/\s+/)
        .filter((v: string) => v);
      const numCols = firstRow.length;

      const columns: Array<{ key: string; title: string; width?: number }> = [];

      // Only add row number column for structured data
      if (isStructured) {
        columns.push({ title: 'Row', key: 'row', width: 60 });
      }

      // Remove column limit - show all columns with horizontal scrolling
      for (let i = 0; i < numCols; i++) {
        columns.push({
          title: isStructured ? `Col ${i + 1}` : '',
          key: `col${i}`,
          width: 120,
        });
      }

      return columns;
    }
  } catch (e) {
    error(`Error parsing array structure: ${e}`);
  }

  // Fallback
  const columns: Array<{ key: string; title: string; width?: number }> = [];
  if (isStructured) {
    columns.push({ title: 'Index', key: 'index', width: 60 });
  }
  columns.push({ title: isStructured ? 'Value' : '', key: 'value' });
  return columns;
};

// Parse array/matrix data and generate table rows
const getTableData = (variable: any): any[] => {
  // Handle DataFrames with parsed data
  if (variable.is_dataframe && variable.parsed_data && Array.isArray(variable.parsed_data)) {
    // Using DataFrame parsed_data
    return variable.parsed_data;
  }

  // Handle DataFrames without parsed_data - check if value is already parsed JSON
  if (variable.is_dataframe) {
    // DataFrame without parsed_data, checking if value is JSON

    // Check if the value is already a JSON array (parsed DataFrame data)
    if (typeof variable.value === 'string') {
      try {
        const parsed = JSON.parse(variable.value);
        if (Array.isArray(parsed)) {
          // DataFrame value is JSON array, using it directly
          return parsed;
        }
      } catch (e) {
        // DataFrame value is not JSON, ignoring
      }
    } else if (Array.isArray(variable.value)) {
      // DataFrame value is already an array, using it directly
      return variable.value;
    }

    return [];
  }

  const value = variable.value || '';
  const isStructured = isStructuredData(variable);

  try {
    // Simple 1D array: [1, 2, 3, 4, 5]
    if (value.match(/^\[.*\]$/s) && !value.includes(';')) {
      const cleanValue = value.replace(/^\[|\]$/g, '').trim();
      const items = cleanValue.split(',').map((v: string) => v.trim());

      return items.map((item: string, index: number) => {
        const rowData: any = { value: item };
        // Only include index for structured data
        if (isStructured) {
          rowData.index = index + 1;
        }
        return rowData;
      });
    }

    // 2D matrix: [1 2 3; 4 5 6; 7 8 9]
    const rows = value
      .replace(/^\[|\]$/g, '')
      .split(';')
      .map((r: string) => r.trim());

    return rows.map((row: string, rowIndex: number) => {
      const values = row.split(/\s+/).filter((v: string) => v);
      const rowData: any = {};

      // Only include row number for structured data
      if (isStructured) {
        rowData.row = rowIndex + 1;
      }

      values.forEach((val: string, colIndex: number) => {
        rowData[`col${colIndex}`] = val;
      });

      return rowData;
    });
  } catch (e) {
    error(`Error parsing array data: ${e}`);
    const errorData: any = { value: 'Error parsing data' };
    if (isStructured) {
      errorData.index = 1;
    }
    return [errorData];
  }
};

// Refresh variables from backend
const refreshVariables = async () => {
  try {
    if (props.isDebugging) {
      // During debugging: get variables from current debug frame
      try {
        const variables = await invoke<Record<string, any>>('debug_get_variables');
        if (variables && Object.keys(variables).length > 0) {
          debugVariablesLocal.value = variables;
        }
      } catch (err: any) {
        // This is expected when debug session is not paused or doesn't exist
        const errorMessage = err?.message || String(err);
        if (
          !errorMessage.includes('not initialized') &&
          !errorMessage.includes('not paused') &&
          !errorMessage.includes('No variables') &&
          !errorMessage.includes('not available')
        ) {
          await warn(`Failed to refresh debug variables: ${errorMessage}`);
        }
      }
    } else {
      // Normal mode: trigger workspace variables refresh
      // This will trigger an automatic update via the workspace:variables-updated event
      try {
        await invoke('refresh_workspace_variables');
      } catch (err: any) {
        // This is expected if no code has been executed or Julia process is not ready
      }
    }
  } catch (err) {
    await warn(`Failed to refresh variables: ${err}`);
  }
};

// Watch for debugging state changes - refresh variables when mode changes
watch(
  () => props.isDebugging,
  (newValue, oldValue) => {
    if (newValue !== oldValue) {
      // Mode changed - refresh variables for the new mode
      if (newValue) {
        // Switched to debug mode - fetch debug variables
        refreshVariables();
      } else {
        // Switched to normal mode - clear debug variables and fetch workspace variables if needed
        debugVariablesLocal.value = {};
        // Only refresh if store is empty
        if (Object.keys(appStore.workspaceVariables).length === 0) {
          refreshVariables();
        }
      }
    }
  }
);

// Listen for variables updated event
let unlistenVariables: (() => void) | null = null;
let unlistenExecutionStopped: (() => void) | null = null;
let unlistenWorkspaceVariables: (() => void) | null = null;
let unlistenDebugCompleted: (() => void) | null = null;
let unlistenDebugSessionStarted: (() => void) | null = null;
let unlistenBackendBusy: (() => void) | null = null;

onMounted(async () => {
  try {
    // Note: In normal mode, debugVariables computed property automatically uses appStore.workspaceVariables
    // In debug mode, debugVariablesLocal will be populated by debug:execution-stopped events

    // Listen for backend-busy event - DO NOT clear variables here
    // The backend-busy event is emitted for ALL code execution, including API calls like get_variable_value
    // We should only clear variables when actual file execution starts, which will be followed by
    // a workspace:variables-updated event that will replace the variables anyway.
    // Clearing here causes variables to disappear when clicking on array variables to view their contents.
    unlistenBackendBusy = await listen('backend-busy', async () => {
      // backend-busy received - NOT clearing variables (will be updated by workspace:variables-updated when file execution completes)
    });

    // Listen for debug session started - clear variables when starting a new debug session
    unlistenDebugSessionStarted = await listen('debug:session-started', async () => {
      debugVariablesLocal.value = {};
      // Also clear the store to avoid stale data
      appStore.setWorkspaceVariables({});
    });

    // Listen for debug execution stopped event (includes variables)
    unlistenExecutionStopped = await listen('debug:execution-stopped', async (event: any) => {
      // Accept both 'variables' (from debugger) and 'variable_summaries' (legacy)
      const variables = event.payload?.variables || event.payload?.variable_summaries;

      if (variables) {
        // Variables are already filtered by Julia backend
        // In debug mode, update debugVariablesLocal (which debugVariables computed property uses)
        debugVariablesLocal.value = variables;
      } else {
        await warn('No variables in payload');
        // Clear debug variables if no variables in payload (shouldn't happen, but be safe)
        debugVariablesLocal.value = {};
      }
    });

    unlistenVariables = await listen('debug:variables-updated', async (event: any) => {
      if (event.payload) {
        // Variables are already filtered by Julia backend
        // In debug mode, update debugVariablesLocal
        debugVariablesLocal.value = event.payload;
      } else {
        await warn('debug:variables-updated event has no payload');
      }
    });

    // Listen for workspace variables after normal execution
    // Only update if not debugging - debug mode uses debug:execution-stopped events
    unlistenWorkspaceVariables = await listen('workspace:variables-updated', async (event: any) => {
      // Only update if not in debug mode - debug mode should use debug:execution-stopped
      if (!props.isDebugging) {
        if (event.payload) {
          // Update the store (debugVariables computed property will automatically reflect this)
          appStore.setWorkspaceVariables(event.payload);
        } else {
          // Don't clear variables if payload is missing - this might be a spurious event
          // (e.g., triggered by get_variable_value call). Just ignore it.
          // The store will persist the existing variables, and debugVariables computed property will reflect them.
          const storeCount = Object.keys(appStore.workspaceVariables).length;
          await warn('⚠️ workspace:variables-updated event has NO payload!');
          await warn(
            `Ignoring to prevent accidental clearing. Store still has: ${storeCount} variables`
          );
        }
      }
    });

    // Listen for debug session completed (includes final variables)
    unlistenDebugCompleted = await listen('debug:session-completed', async (event: any) => {
      const variables = event.payload?.variables;

      if (variables) {
        // Variables are already filtered by Julia backend
        // In debug mode, update debugVariablesLocal
        debugVariablesLocal.value = variables;
      } else {
        await warn('No variables in session-completed payload');
      }
    });

    // After setting up event listeners, check if variables are missing and refresh if needed
    const hasVariables = props.isDebugging
      ? Object.keys(debugVariablesLocal.value).length > 0
      : Object.keys(appStore.workspaceVariables).length > 0;

    if (!hasVariables) {
      await refreshVariables();
    }
  } catch (err) {
    error(`VariablesPanel: Failed to set up event listeners: ${err}`);
  }
});

onUnmounted(() => {
  if (unlistenVariables) {
    unlistenVariables();
  }
  if (unlistenExecutionStopped) {
    unlistenExecutionStopped();
  }
  if (unlistenWorkspaceVariables) {
    unlistenWorkspaceVariables();
  }
  if (unlistenDebugCompleted) {
    unlistenDebugCompleted();
  }
  if (unlistenDebugSessionStarted) {
    unlistenDebugSessionStarted();
  }
  if (unlistenBackendBusy) {
    unlistenBackendBusy();
  }
});
</script>

<style scoped>
.variables-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: #282828;
}

.panel-content {
  flex: 1;
  overflow-y: auto;
  padding: 8px;
  background-color: #282828;
}

.no-debug-session,
.no-variables {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 32px 16px;
  text-align: center;
  color: #888;
}

.no-debug-session p,
.no-variables p {
  margin: 8px 0 0;
  font-size: 13px;
}

.no-debug-session .hint,
.no-variables .hint {
  font-size: 11px;
  color: #666;
  margin-top: 4px;
}

.variables-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.variable-item {
  background-color: #252526;
  border: 1px solid #3e3e42;
  border-radius: 4px;
  padding: 8px;
  font-size: 12px;
  transition: all 0.2s;
}

.variable-item:hover {
  background-color: #2d2d30;
}

.variable-item.expandable {
  cursor: pointer;
  border-color: #389826;
}

.variable-item.expandable:hover {
  background-color: #2a3428;
  border-color: #4aa830;
  transform: translateY(-1px);
}

.variable-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 4px;
}

.variable-name {
  font-weight: 600;
  color: #4aa830;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
}

.variable-type {
  font-size: 11px;
  color: #888;
  font-style: italic;
}

.variable-value {
  color: #cccccc;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  word-break: break-all;
  padding: 4px 0;
}

/* Scrollbar styling */
.panel-content::-webkit-scrollbar {
  width: 8px;
}

.panel-content::-webkit-scrollbar-track {
  background: #282828;
}

.panel-content::-webkit-scrollbar-thumb {
  background: #424242;
  border-radius: 4px;
}

.panel-content::-webkit-scrollbar-thumb:hover {
  background: #4e4e4e;
}

.expand-icon {
  margin-left: 8px;
  opacity: 0.6;
  font-size: 14px;
}

.variable-item.expandable:hover .expand-icon {
  opacity: 1;
}

/* Modal styles */
.modal-content {
  display: flex;
  flex-direction: column;
  gap: 12px;
  max-height: calc(95vh - 120px);
  overflow: hidden;
}

/* Horizontal metadata layout */
.modal-metadata {
  display: flex;
  flex-wrap: wrap;
  gap: 24px;
  padding: 8px 12px;
  background-color: #252526;
  border-radius: 4px;
  border: 1px solid #3e3e42;
}

.metadata-item {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.metadata-item strong {
  color: #4aa830;
  font-weight: 600;
}

.metadata-item span {
  color: #cccccc;
}

.truncation-warning {
  background-color: #3a3a00;
  border: 1px solid #8b8b00;
  border-radius: 4px;
  padding: 8px 12px;
  margin-bottom: 8px;
  color: #ffeb3b;
  font-size: 12px;
  line-height: 1.5;
}

.modal-value {
  background-color: #1e1e1e;
  border: 1px solid #3e3e42;
  border-radius: 4px;
  padding: 12px;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 12px;
  color: #cccccc;
  max-height: 400px;
  overflow: auto;
  white-space: pre-wrap;
  word-wrap: break-word;
}

.table-container {
  margin-top: 8px;
  border-radius: 4px;
  overflow-x: auto;
  overflow-y: auto;
  max-width: 100%;
  width: 100%;
  max-height: 700px;
}

/* Hide headers for arrays/matrices (not for structured data like DataFrames) */
.table-container.hide-headers thead {
  display: none;
}

/* Style the HTML table */
.data-table {
  width: 100%;
  border-collapse: collapse;
  background-color: #1e1e1e;
  font-size: 12px;
  min-width: max-content;
}

.data-table th {
  background-color: #252526;
  color: #4aa830;
  font-weight: 600;
  white-space: nowrap;
  padding: 8px 12px;
  text-align: left;
  border-bottom: 1px solid #333;
  position: sticky;
  top: 0;
  z-index: 1;
}

.data-table td {
  color: #cccccc;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  white-space: nowrap;
  padding: 6px 12px;
  border-bottom: 1px solid #333;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.data-table tr:hover {
  background-color: #2d2d30;
}

.data-table tr.even {
  background-color: #1a1a1a;
}

.data-table tr.even:hover {
  background-color: #2d2d30;
}

.loading-indicator {
  display: flex;
  align-items: center;
  padding: 20px;
  color: #4aa830;
  font-size: 13px;
}
</style>
