<template>
  <div
    style="
      height: 100%;
      display: flex;
      flex-direction: column;
      background-color: #282828;
      min-height: 0;
    "
  >
    <!-- Header with controls -->
    <div style="padding: 10px; border-bottom: 1px solid #444; flex-shrink: 0">
      <n-space justify="space-between" align="center">
        <n-text style="font-size: 12px; color: #ccc">
          {{ plotCount }} plot{{ plotCount !== 1 ? 's' : '' }}
        </n-text>
        <n-space>
          <n-tooltip trigger="hover">
            <template #trigger>
              <n-button size="tiny" circle @click="clearAllPlots" :disabled="plotCount === 0">
                <n-icon><TrashOutline /></n-icon>
              </n-button>
            </template>
            Clear All Plots
          </n-tooltip>
        </n-space>
      </n-space>
    </div>

    <!-- Plot Grid -->
    <div style="flex-grow: 1; overflow: auto; padding: 10px; min-height: 0">
      <n-empty v-if="plotCount === 0" description="No plots captured yet" style="margin-top: 20px">
        <template #icon>
          <n-icon style="font-size: 48px; color: #666"><TrendingUpOutline /></n-icon>
        </template>
      </n-empty>

      <div v-else class="plot-grid">
        <div
          v-for="plot in plots"
          :key="plot.id"
          class="plot-thumbnail"
          @click="openPlotModal(plot)"
        >
          <div class="plot-thumbnail-content">
            <img
              v-if="plot.mime_type.startsWith('image/')"
              :src="getImageSrcForPlot(plot)"
              :alt="`Plot ${plot.id}`"
              :data-plot-id="plot.id"
              class="plot-image"
              @error="handleImageError"
              @load="() => plotRetryAttempts.delete(plot.id)"
            />
            <div v-else class="plot-placeholder">
              <n-icon><TrendingUpOutline /></n-icon>
              <span>{{ plot.mime_type.split('/')[1]?.toUpperCase() || 'PLOT' }}</span>
            </div>
            <div class="plot-overlay">
              <n-button size="tiny" circle @click.stop="deletePlot(plot.id)">
                <n-icon><CloseOutline /></n-icon>
              </n-button>
            </div>
          </div>
          <div class="plot-info">
            <n-text style="font-size: 11px; color: #ccc">
              {{ formatTimestamp(plot.timestamp) }}
            </n-text>
            <n-text v-if="plot.source_file" style="font-size: 10px; color: #999">
              {{ plot.source_file ? basename(plot.source_file) : 'REPL' }}
            </n-text>
          </div>
        </div>
      </div>
    </div>

    <!-- Plot Modal -->
    <n-modal
      v-model:show="showPlotModal"
      preset="card"
      style="width: 90vw; max-width: 1200px; height: 90vh"
      v-if="currentPlot"
    >
      <template #header>
        <div style="display: flex; justify-content: space-between; align-items: center">
          <n-text>Plot Viewer</n-text>
          <n-space v-if="plots.length > 1">
            <n-button size="small" @click="previousPlot" :disabled="currentPlotIndex <= 0">
              <n-icon><ChevronBackOutline /></n-icon>
            </n-button>
            <n-text style="font-size: 12px; color: #ccc">
              {{ currentPlotIndex + 1 }} / {{ plots.length }}
            </n-text>
            <n-button
              size="small"
              @click="nextPlot"
              :disabled="currentPlotIndex >= plots.length - 1"
            >
              <n-icon><ChevronForwardOutline /></n-icon>
            </n-button>
          </n-space>
        </div>
      </template>

      <div style="height: calc(90vh - 120px); display: flex; flex-direction: column">
        <!-- Plot Display -->
        <div
          style="
            flex: 1;
            display: flex;
            justify-content: center;
            align-items: center;
            min-height: 0;
          "
        >
          <img
            v-if="currentPlot && currentPlot.mime_type.startsWith('image/')"
            :src="getImageSrcForPlot(currentPlot)"
            :alt="`Plot ${currentPlot.id}`"
            :data-plot-id="currentPlot.id"
            style="width: 100%; height: 100%; object-fit: contain"
            @error="handleImageError"
            @load="() => plotRetryAttempts.delete(currentPlot.id)"
          />
          <div v-else-if="currentPlot" class="plot-modal-placeholder">
            <n-icon style="font-size: 64px; color: #666"><TrendingUpOutline /></n-icon>
            <n-text style="margin-top: 16px"
              >{{ currentPlot.mime_type.split('/')[1]?.toUpperCase() || 'PLOT' }} Plot</n-text
            >
            <n-text style="font-size: 12px; color: #999; margin-top: 8px">
              Full display not yet implemented for this plot type
            </n-text>
          </div>
        </div>

        <!-- Plot Details -->
        <div
          v-if="currentPlot"
          style="
            flex-shrink: 0;
            padding: 12px;
            border-top: 1px solid #444;
            background-color: #1e1e1e;
          "
        >
          <n-space vertical size="small">
            <div style="display: flex; justify-content: space-between; align-items: center">
              <n-text style="font-size: 14px; font-weight: bold">Plot Details</n-text>
              <n-button size="small" @click="deleteCurrentPlot">
                <n-icon><TrashOutline /></n-icon>
                Delete
              </n-button>
            </div>

            <n-space vertical size="small">
              <div>
                <n-text style="font-size: 12px; color: #ccc">Created:</n-text>
                <n-text style="font-size: 12px; margin-left: 8px">{{
                  formatTimestamp(currentPlot.timestamp)
                }}</n-text>
              </div>

              <div v-if="currentPlot.source_file">
                <n-text style="font-size: 12px; color: #ccc">Source:</n-text>
                <n-text style="font-size: 12px; margin-left: 8px">
                  {{ currentPlot.source_file || 'REPL Session' }}
                </n-text>
                <n-text
                  v-if="currentPlot.line_number"
                  style="font-size: 12px; margin-left: 8px; color: #999"
                >
                  (Line {{ currentPlot.line_number }})
                </n-text>
              </div>

              <div v-if="currentPlot.code_context">
                <n-text style="font-size: 12px; color: #ccc">Code:</n-text>
                <n-text
                  style="font-size: 11px; margin-left: 8px; color: #999; font-family: monospace"
                >
                  {{ currentPlot.code_context }}
                </n-text>
              </div>
            </n-space>
          </n-space>
        </div>
      </div>
    </n-modal>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, watch } from 'vue';
import { NSpace, NButton, NTooltip, NText, NEmpty, NModal, NIcon } from 'naive-ui';
import {
  TrashOutline,
  TrendingUpOutline,
  CloseOutline,
  ChevronBackOutline,
  ChevronForwardOutline,
} from '@vicons/ionicons5';
import { usePlotStore } from '../../store/plotStore';
import { invoke } from '@tauri-apps/api/core';
import { basename } from '@tauri-apps/api/path';
import { info, warn, error, logObject, debug } from '../../utils/logger';

const plotStore = usePlotStore();
const plots = computed(() => plotStore.plots);
const plotCount = computed(() => plotStore.plotCount);

const showPlotModal = ref(false);
const currentPlot = ref(null);
const currentPlotIndex = ref(0);

// Track failed image srcs to avoid repeated logging
const failedImageSrcs = new Set();

const formatTimestamp = (timestamp) => {
  const date = new Date(timestamp);
  return date.toLocaleTimeString();
};

const getImageSrcForPlot = (plot) => {
  debug(
    `Getting image src for plot: ${plot.id} MIME: ${plot.mime_type} Has imageSrc: ${!!plot.imageSrc} Has image_url: ${!!plot.image_url} Has data: ${!!plot.data}`
  );

  // If imageSrc is already set and valid, use it
  // Validate that it's not empty, not just "/", and starts with http or data:
  if (
    plot.imageSrc &&
    plot.imageSrc.trim() !== '' &&
    plot.imageSrc.trim() !== '/' &&
    (plot.imageSrc.startsWith('http') || plot.imageSrc.startsWith('data:'))
  ) {
    debug(`Using existing imageSrc for plot: ${plot.id}`);
    return plot.imageSrc;
  }

  // If image_url is available, use it directly
  // Validate that it's not empty, not just "/", and starts with http or data:
  if (
    plot.image_url &&
    plot.image_url.trim() !== '' &&
    plot.image_url.trim() !== '/' &&
    (plot.image_url.startsWith('http') || plot.image_url.startsWith('data:'))
  ) {
    debug(`Using image_url for plot: ${plot.id}`);
    return plot.image_url;
  }

  // If we have data and it's an image type, create a data URL
  if (plot.data && plot.mime_type && plot.mime_type.startsWith('image/')) {
    try {
      // Ensure data is valid base64
      const base64Data = plot.data.replace(/^data:.*,/, ''); // Remove existing data URL prefix if any
      const dataUrl = `data:${plot.mime_type};base64,${base64Data}`;
      debug(`Created data URL for plot: ${plot.id} from data`);
      return dataUrl;
    } catch (e) {
      error(`Failed to create data URL for plot ${plot.id}:`, e);
    }
  }

  debug(`No valid image URL or data available for plot: ${plot.id}`);
  // Return null instead of empty string to prevent browser from resolving to tauri.localhost
  // The image element should handle null/undefined src gracefully
  return null;
};

// Track retry attempts for each plot
const plotRetryAttempts = new Map();

const handleImageError = async (event) => {
  const src = event.target.src;
  const plotId = event.target.getAttribute('data-plot-id');

  if (!plotId) {
    // If we can't identify the plot, just log and clear
    if (!failedImageSrcs.has(src)) {
      error(`Image failed to load (unknown plot): ${src}`);
      failedImageSrcs.add(src);
    }
    event.target.src = ''; // Clear the src to show a placeholder
    return;
  }

  const retryCount = plotRetryAttempts.get(plotId) || 0;
  const plot = plots.value.find((p) => p.id === plotId);

  if (retryCount < 2 && plot) {
    // Retry loading the image
    plotRetryAttempts.set(plotId, retryCount + 1);
    debug(`Retrying image load for plot ${plotId} (attempt ${retryCount + 1})`);

    // Try to reload from plot store
    setTimeout(
      async () => {
        const updatedPlot = await plotStore.getPlot(plotId);
        if (updatedPlot) {
          // Try to get image URL or create data URL from plot data
          let newSrc = null;

          // First, try to rebuild URL with current port if available
          const currentPort = plotStore.plotServerPort();
          if (currentPort && updatedPlot.mime_type?.startsWith('image/')) {
            newSrc = `http://127.0.0.1:${currentPort}/plots/${plotId}/image`;
            debug(`Rebuilt URL with current port ${currentPort} for plot ${plotId}`);
          } else if (updatedPlot.image_url) {
            newSrc = updatedPlot.image_url;
          } else if (updatedPlot.data && updatedPlot.mime_type?.startsWith('image/')) {
            try {
              const base64Data = updatedPlot.data.replace(/^data:.*,/, '');
              newSrc = `data:${updatedPlot.mime_type};base64,${base64Data}`;
              debug(`Using data URL fallback for plot ${plotId}`);
            } catch (e) {
              debug(`Failed to create data URL for retry: ${e}`);
            }
          }

          if (newSrc && newSrc !== src) {
            debug(`Retrying with new src for plot ${plotId}: ${newSrc}`);
            event.target.src = newSrc;
          }
        }
      },
      500 * (retryCount + 1)
    ); // Exponential backoff
  } else {
    // Max retries reached - try data URL fallback
    if (plot && plot.data && plot.mime_type?.startsWith('image/')) {
      try {
        const base64Data = plot.data.replace(/^data:.*,/, '');
        const dataUrl = `data:${plot.mime_type};base64,${base64Data}`;
        debug(`Falling back to data URL for plot ${plotId} after HTTP failures`);
        event.target.src = dataUrl;
        return;
      } catch (e) {
        debug(`Failed to create data URL fallback: ${e}`);
      }
    }

    // Max retries reached or no plot found
    if (!failedImageSrcs.has(src)) {
      error(`Image failed to load after ${retryCount} retries: ${src} (plot: ${plotId})`);
      failedImageSrcs.add(src);
    }
    event.target.src = ''; // Clear the src to show a placeholder
  }
};

const clearAllPlots = async () => {
  try {
    await plotStore.clearAllPlots();
  } catch (error) {
    console.error('Failed to clear plots:', error);
  }
};

const deletePlot = async (plotId) => {
  try {
    await plotStore.deletePlot(plotId);
  } catch (error) {
    console.error('Failed to delete plot:', error);
  }
};

const openPlotModal = (plot) => {
  debug(`Opening plot modal for plot: ${plot.id}`);
  if (plot && plots.value.length > 0) {
    currentPlot.value = plot;
    const index = plots.value.findIndex((p) => p.id === plot.id);
    currentPlotIndex.value = index >= 0 ? index : 0;
    showPlotModal.value = true;
  }
};

const previousPlot = () => {
  if (currentPlotIndex.value > 0 && plots.value.length > 0) {
    currentPlotIndex.value--;
    currentPlot.value = plots.value[currentPlotIndex.value];
  }
};

const nextPlot = () => {
  if (currentPlotIndex.value < plotCount.value - 1 && plots.value.length > 0) {
    currentPlotIndex.value++;
    currentPlot.value = plots.value[currentPlotIndex.value];
  }
};

const deleteCurrentPlot = async () => {
  if (currentPlot.value) {
    debug(`Deleting current plot: ${currentPlot.value.id}`);
    await deletePlot(currentPlot.value.id);
    showPlotModal.value = false;
  }
};

// Lifecycle hooks - Plot listening is now initialized globally in the plot store
onMounted(async () => {
  debug('PlotLibrary: Component mounted, plot listening already initialized globally');
  console.log('PlotLibrary: Component mounted, plots count:', plotCount.value);
  console.log('PlotLibrary: Current plots:', plots.value);
});

onUnmounted(() => {
  // Don't stop plot listening here since it's managed globally
});

// Watch for changes in plots and reset modal state if current plot is no longer available
watch(
  plots,
  (newPlots) => {
    debug(`PlotLibrary: Plots changed, count: ${newPlots.length}`);
    newPlots.forEach((plot) => {
      debug(
        `PlotLibrary: Plot ${plot.id} - MIME: ${plot.mime_type}, Has imageSrc: ${!!plot.imageSrc}, Has image_url: ${!!plot.image_url}`
      );
    });

    if (showPlotModal.value && currentPlot.value) {
      const plotStillExists = newPlots.some((p) => p.id === currentPlot.value.id);
      if (!plotStillExists) {
        showPlotModal.value = false;
        currentPlot.value = null;
        currentPlotIndex.value = 0;
      }
    }
  },
  { deep: true }
);
</script>

<style scoped>
.plot-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
  gap: 12px;
}

.plot-thumbnail {
  cursor: pointer;
  border-radius: 6px;
  overflow: hidden;
  background-color: #1e1e1e;
  border: 1px solid #444;
  transition: all 0.2s ease;
}

.plot-thumbnail:hover {
  border-color: #68a0d8;
  transform: translateY(-2px);
  box-shadow: 0 4px 8px rgba(0, 0, 0, 0.3);
}

.plot-thumbnail-content {
  position: relative;
  aspect-ratio: 4/3;
  background-color: #1e1e1e;
}

.plot-image {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.plot-placeholder {
  width: 100%;
  height: 100%;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  color: #666;
  font-size: 12px;
  gap: 4px;
}

.plot-overlay {
  position: absolute;
  top: 4px;
  right: 4px;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.plot-thumbnail:hover .plot-overlay {
  opacity: 1;
}

.plot-info {
  padding: 6px 8px;
  background-color: #1e1e1e;
}

.plot-modal-placeholder {
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  color: #666;
  height: 100%;
}
</style>
