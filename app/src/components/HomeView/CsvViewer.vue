<template>
  <div class="csv-viewer" style="height: 100%; display: flex; flex-direction: column">
    <!-- Header with file info -->
    <div class="csv-header" style="padding: 12px; border-bottom: 1px solid var(--n-border-color)">
      <n-space align="center" justify="space-between">
        <div>
          <n-text strong>{{ fileName }}</n-text>
          <n-text depth="3" style="margin-left: 8px">
            {{
              csvData
                ? `${csvData.total_rows} rows, ${csvData.headers?.length || 0} columns`
                : 'Loading...'
            }}
          </n-text>
        </div>
        <n-space>
          <n-button size="small" @click="refreshData" :loading="loading">
            <template #icon>
              <n-icon><Refresh /></n-icon>
            </template>
            Refresh
          </n-button>
        </n-space>
      </n-space>
    </div>

    <!-- Loading state -->
    <div
      v-if="loading"
      style="flex: 1; display: flex; align-items: center; justify-content: center"
    >
      <n-spin size="large">
        <template #description> Loading CSV data... </template>
      </n-spin>
    </div>

    <!-- Error state -->
    <div
      v-else-if="error"
      style="flex: 1; display: flex; align-items: center; justify-content: center"
    >
      <n-empty description="Failed to load CSV data">
        <template #extra>
          <n-text depth="3">{{ error }}</n-text>
          <br />
          <n-button size="small" @click="refreshData">Retry</n-button>
        </template>
      </n-empty>
    </div>

    <!-- CSV Data Table -->
    <div v-else-if="csvData && tableData.length > 0" style="width: 100%">
      <n-data-table
        :columns="tableColumns"
        :data="tableData"
        :header-height="48"
        size="small"
        :max-height="tableHeight"
        :min-row-height="32"
        :scroll-x="scrollX"
        virtual-scroll
        virtual-scroll-x
        virtual-scroll-header
      />
    </div>

    <!-- Empty state -->
    <div v-else style="flex: 1; display: flex; align-items: center; justify-content: center">
      <n-empty description="No data to display">
        <template #extra>
          <n-text depth="3">The CSV file appears to be empty or could not be parsed.</n-text>
        </template>
      </n-empty>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch, nextTick } from 'vue';
import { NDataTable, NButton, NSpace, NText, NSpin, NEmpty, NIcon } from 'naive-ui';
import { Refresh } from '@vicons/ionicons5';
import { debug, error as logError } from '../../utils/logger';
import { useAppStore } from '../../store/appStore';
import { invoke } from '@tauri-apps/api/core';

interface CsvData {
  headers: string[];
  rows: any[][];
  total_rows: number;
  column_widths: number[];
}

interface CsvResponse {
  success: boolean;
  data?: CsvData;
  error?: string;
  file_path: string;
}

interface Props {
  filePath: string;
  fileName: string;
  projectPath?: string;
}

const props = defineProps<Props>();
const appStore = useAppStore();

const loading = ref(false);
const csvData = ref<CsvData | null>(null);
const error = ref<string | null>(null);
const tableHeight = ref(400);
const scrollX = ref(0);

// Computed table columns
const tableColumns = computed(() => {
  if (!csvData.value?.headers) {
    scrollX.value = 0; // Default scrollX when no data
    return [];
  }

  // Use column widths from backend if available, otherwise fallback to fixed width
  const columnWidths = csvData.value.column_widths || [];

  // Calculate scrollX based on actual column widths
  const totalWidth =
    columnWidths.length > 0
      ? columnWidths.reduce((sum, width) => sum + width, 0)
      : csvData.value.headers.length * 120; // Fallback to fixed width
  scrollX.value = totalWidth; // Ensure minimum width of 1000px

  return csvData.value.headers.map((header, index) => ({
    title: header,
    key: `col_${index}`,
    width: columnWidths[index] || 120, // Use calculated width or fallback to 120
    ellipsis: {
      tooltip: true,
    },
    render: (row: any) => {
      const value = row[`col_${index}`];
      if (typeof value === 'number') {
        return value.toLocaleString();
      }
      return value || '';
    },
  }));
});

// Computed table data
const tableData = computed(() => {
  if (!csvData.value?.rows) return [];

  return csvData.value.rows.map((row, rowIndex) => {
    const rowData: any = { key: rowIndex };
    row.forEach((cell, colIndex) => {
      rowData[`col_${colIndex}`] = cell;
    });
    return rowData;
  });
});

// Load CSV data from the backend
const loadCsvData = async () => {
  loading.value = true;
  error.value = null;

  try {
    debug(`Loading CSV data for: ${props.filePath}`);

    // Get file server URL directly from backend (same approach as ImageViewer)
    const serverUrl = await invoke<string | null>('get_file_server_url');
    if (!serverUrl) {
      throw new Error('File server not available');
    }

    // Construct the URL for the CSV parsing endpoint
    const relativePath = props.projectPath
      ? props.filePath.replace(props.projectPath, '').replace(/^[\\\/]/, '')
      : props.filePath;

    const csvUrl = `${serverUrl}/csv/${encodeURIComponent(relativePath)}`;

    debug(`CSV viewer: Attempting to fetch from URL: ${csvUrl}`);
    debug(`CSV viewer: File server URL: ${serverUrl}`);
    debug(`CSV viewer: Relative path: ${relativePath}`);
    debug(`CSV viewer: Original file path: ${props.filePath}`);
    debug(`CSV viewer: Project path: ${props.projectPath || 'undefined'}`);

    const response = await fetch(csvUrl);
    debug(`CSV viewer: Response status: ${response.status} ${response.statusText}`);

    const result: CsvResponse = await response.json();

    if (result.success && result.data) {
      csvData.value = result.data;
      debug(`Successfully loaded CSV with ${result.data.total_rows} rows`);
    } else {
      error.value = result.error || 'Failed to parse CSV file';
      logError(`CSV parsing failed: ${error.value}`);
    }
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Failed to load CSV data';
    logError(`CSV loading error: ${error.value}`);
    debug(`CSV viewer: Full error details: ${err}`);
  } finally {
    loading.value = false;
  }
};

// Refresh data
const refreshData = () => {
  loadCsvData();
};

// Update table height when component mounts
const updateTableHeight = () => {
  nextTick(() => {
    const container = document.querySelector('.csv-viewer');
    if (container) {
      const headerHeight = 60; // Approximate header height
      const padding = 20; // Extra padding
      const availableHeight = container.clientHeight - headerHeight - padding;
      tableHeight.value = Math.max(200, availableHeight);
      // debug(`CSV viewer: Container height: ${container.clientHeight}, Container width: ${container.clientWidth}, Table height: ${tableHeight.value}`);
    }
  });
};

// Watch for file path changes
watch(
  () => props.filePath,
  () => {
    if (props.filePath) {
      loadCsvData();
    }
  }
);

onMounted(() => {
  if (props.filePath) {
    loadCsvData();
  }
  updateTableHeight();

  // Update table height on window resize
  window.addEventListener('resize', updateTableHeight);

  // Set up resize observer for the container
  const container = document.querySelector('.csv-viewer');
  if (container) {
    const resizeObserver = new ResizeObserver(() => {
      updateTableHeight();
    });
    resizeObserver.observe(container);

    // Store the observer for cleanup
    (window as any).csvViewerResizeObserver = resizeObserver;
  }
});

// Cleanup
import { onUnmounted } from 'vue';
onUnmounted(() => {
  window.removeEventListener('resize', updateTableHeight);

  // Clean up resize observer
  if ((window as any).csvViewerResizeObserver) {
    (window as any).csvViewerResizeObserver.disconnect();
  }
});
</script>

<style scoped>
.csv-viewer {
  /* Use theme background color */
  width: 100%;
  min-width: 0; /* Allow flex items to shrink below their content size */
}

.csv-header {
  border-bottom: 1px solid var(--n-border-color);
}

/* Ensure the table container uses full width */
.csv-viewer > div:last-child {
  width: 100%;
  min-width: 0;
}

/* Override any potential table width constraints */
.csv-viewer :deep(.n-data-table-wrapper) {
  width: 100% !important;
  max-width: none !important;
  min-width: 0 !important;
}

.csv-viewer :deep(.n-data-table-base-table) {
  width: 100% !important;
  max-width: none !important;
  min-width: 0 !important;
}
</style>
