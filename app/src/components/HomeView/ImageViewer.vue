<template>
  <div class="image-viewer">
    <div class="image-viewer-header">
      <n-space
        align="center"
        justify="space-between"
        style="padding: 8px 12px; border-bottom: 1px solid #333"
      >
        <div class="viewer-controls">
          <n-button-group>
            <n-button size="small" @click="zoomOut" :disabled="zoomLevel <= 0.25">
              <template #icon>
                <n-icon><Remove /></n-icon>
              </template>
            </n-button>
            <n-button size="small" @click="resetZoom">
              {{ Math.round(zoomLevel * 100) }}%
            </n-button>
            <n-button size="small" @click="zoomIn" :disabled="zoomLevel >= 4">
              <template #icon>
                <n-icon><Add /></n-icon>
              </template>
            </n-button>
          </n-button-group>

          <n-button size="small" @click="toggleFitToWindow" style="margin-left: 8px">
            <template #icon>
              <n-icon><Resize /></n-icon>
            </template>
            {{ fitToWindow ? 'Actual Size' : 'Fit Window' }}
          </n-button>
        </div>

        <div class="image-info">
          <n-text depth="3" size="small">
            {{ imageInfo.width }} × {{ imageInfo.height }} pixels
          </n-text>
        </div>
      </n-space>
    </div>

    <div
      class="image-container"
      ref="imageContainer"
      @wheel="handleWheel"
      @mousedown="startPan"
      @mousemove="pan"
      @mouseup="stopPan"
      @mouseleave="stopPan"
    >
      <n-spin :show="loading" style="width: 100%; height: 100%">
        <div class="image-wrapper" :style="imageWrapperStyle">
          <img
            v-if="!loading && imageUrl"
            :src="imageUrl"
            :alt="fileName"
            @load="onImageLoad"
            @error="onImageError"
            class="image-content"
            :style="imageStyle"
            draggable="false"
          />
        </div>

        <n-empty
          v-if="error"
          description="Failed to load image"
          style="height: 100%; display: flex; justify-content: center; align-items: center"
        >
          <template #extra>
            <n-button size="small" @click="retryLoad">Retry</n-button>
          </template>
        </n-empty>
      </n-spin>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue';
import { NSpace, NButton, NButtonGroup, NIcon, NText, NSpin, NEmpty } from 'naive-ui';
import { Add, Remove, Resize } from '@vicons/ionicons5';
import { invoke } from '@tauri-apps/api/core';
import { debug, error as logError } from '../../utils/logger';

interface Props {
  filePath: string;
  fileName: string;
  projectPath?: string;
}

const props = defineProps<Props>();

interface ImageInfo {
  width: number;
  height: number;
  naturalWidth: number;
  naturalHeight: number;
}

const loading = ref(true);
const error = ref(false);
const imageUrl = ref<string | null>(null);
const zoomLevel = ref(1);
const fitToWindow = ref(true);
const isPanning = ref(false);
const panStart = ref({ x: 0, y: 0 });
const panOffset = ref({ x: 0, y: 0 });
const imageContainer = ref<HTMLElement | null>(null);
const imageInfo = ref<ImageInfo>({
  width: 0,
  height: 0,
  naturalWidth: 0,
  naturalHeight: 0,
});

// Computed styles
const imageWrapperStyle = computed(() => ({
  transform: `translate(${panOffset.value.x}px, ${panOffset.value.y}px)`,
  cursor: isPanning.value ? 'grabbing' : 'grab',
}));

const imageStyle = computed(() => ({
  transform: `scale(${zoomLevel.value})`,
  transformOrigin: 'center center',
}));

// Methods
const loadImage = async () => {
  if (!props.filePath) return;

  loading.value = true;
  error.value = false;

  try {
    // Get file server URL
    const serverUrl = await invoke<string | null>('get_file_server_url');
    if (!serverUrl) {
      throw new Error('File server not running');
    }

    // Construct the image URL
    const relativePath = props.projectPath
      ? props.filePath.replace(props.projectPath, '').replace(/^[\/\\]/, '')
      : props.filePath;

    // Convert backslashes to forward slashes for URL and then encode
    const normalizedPath = relativePath.replace(/\\/g, '/');
    imageUrl.value = `${serverUrl}/files/${encodeURIComponent(normalizedPath)}`;
    await debug(`ImageViewer: Loading image from ${imageUrl.value}`);
  } catch (err) {
    logError(
      `ImageViewer: Failed to load image: ${err instanceof Error ? err.message : String(err)}`
    );
    error.value = true;
  } finally {
    loading.value = false;
  }
};

const onImageLoad = async (event: Event) => {
  const img = event.target as HTMLImageElement;
  imageInfo.value = {
    width: img.naturalWidth,
    height: img.naturalHeight,
    naturalWidth: img.naturalWidth,
    naturalHeight: img.naturalHeight,
  };

  if (fitToWindow.value) {
    fitImageToWindow();
  }

  await debug(`ImageViewer: Image loaded - ${img.naturalWidth}x${img.naturalHeight}`);
};

const onImageError = () => {
  logError('ImageViewer: Failed to load image');
  error.value = true;
  loading.value = false;
};

const retryLoad = () => {
  loadImage();
};

const zoomIn = () => {
  if (zoomLevel.value < 4) {
    zoomLevel.value = Math.min(4, zoomLevel.value * 1.25);
    fitToWindow.value = false;
  }
};

const zoomOut = () => {
  if (zoomLevel.value > 0.25) {
    zoomLevel.value = Math.max(0.25, zoomLevel.value / 1.25);
    fitToWindow.value = false;
  }
};

const resetZoom = () => {
  zoomLevel.value = 1;
  fitToWindow.value = false;
  panOffset.value = { x: 0, y: 0 };
};

const toggleFitToWindow = () => {
  fitToWindow.value = !fitToWindow.value;
  if (fitToWindow.value) {
    fitImageToWindow();
  } else {
    resetZoom();
  }
};

const fitImageToWindow = () => {
  if (!imageContainer.value || !imageInfo.value.naturalWidth) return;

  const containerRect = imageContainer.value.getBoundingClientRect();
  const containerWidth = containerRect.width;
  const containerHeight = containerRect.height;

  const scaleX = containerWidth / imageInfo.value.naturalWidth;
  const scaleY = containerHeight / imageInfo.value.naturalHeight;

  zoomLevel.value = Math.min(scaleX, scaleY, 1); // Don't scale up beyond 100%
  panOffset.value = { x: 0, y: 0 };
};

const handleWheel = (event: WheelEvent) => {
  event.preventDefault();

  const delta = event.deltaY > 0 ? 0.9 : 1.1;
  const newZoom = Math.max(0.25, Math.min(4, zoomLevel.value * delta));

  if (newZoom !== zoomLevel.value) {
    zoomLevel.value = newZoom;
    fitToWindow.value = false;
  }
};

const startPan = (event: MouseEvent) => {
  if (event.button === 0) {
    // Left mouse button
    isPanning.value = true;
    panStart.value = { x: event.clientX - panOffset.value.x, y: event.clientY - panOffset.value.y };
    event.preventDefault();
  }
};

const pan = (event: MouseEvent) => {
  if (isPanning.value) {
    panOffset.value = {
      x: event.clientX - panStart.value.x,
      y: event.clientY - panStart.value.y,
    };
    event.preventDefault();
  }
};

const stopPan = () => {
  isPanning.value = false;
};

// Watch for file path changes
watch(
  () => props.filePath,
  () => {
    loadImage();
  },
  { immediate: true }
);

// Watch for project path changes
watch(
  () => props.projectPath,
  () => {
    loadImage();
  }
);

onMounted(() => {
  loadImage();
});
</script>

<style scoped>
.image-viewer {
  display: flex;
  flex-direction: column;
  height: 100%;
  background-color: #1e1e1e;
}

.image-viewer-header {
  flex-shrink: 0;
  background-color: #2d2d30;
  border-bottom: 1px solid #3e3e42;
}

.viewer-controls {
  display: flex;
  align-items: center;
}

.image-info {
  display: flex;
  align-items: center;
}

.image-container {
  flex: 1;
  overflow: hidden;
  position: relative;
  background-color: #1e1e1e;
  cursor: grab;
}

.image-container:active {
  cursor: grabbing;
}

.image-wrapper {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
}

.image-content {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
  transition: transform 0.1s ease-out;
  user-select: none;
  -webkit-user-drag: none;
}

/* Custom scrollbar for the image container */
.image-container::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

.image-container::-webkit-scrollbar-track {
  background: #2d2d30;
}

.image-container::-webkit-scrollbar-thumb {
  background: #5a5a5a;
  border-radius: 4px;
}

.image-container::-webkit-scrollbar-thumb:hover {
  background: #7a7a7a;
}
</style>
