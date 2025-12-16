<template>
  <GenericModal :show="show" @update:show="onUpdateShow" width="600px" height="auto">
    <template #header>
      <div class="modal-header-content">
        <h2 class="modal-title">Julia Info</h2>
        <p class="modal-subtitle">Julia version and storage information</p>
      </div>
    </template>

    <div class="settings-content">
      <!-- Julia Version Section -->
      <div class="section">
        <h3 class="section-title">Julia Information</h3>
        <div class="version-info">
          <div class="version-item">
            <span class="version-label">Julia Version:</span>
            <n-tag :type="juliaVersion ? 'success' : 'warning'" size="small">
              {{ juliaVersion || 'Loading...' }}
            </n-tag>
          </div>
        </div>
      </div>

      <!-- Storage Paths Section -->
      <div class="section">
        <h3 class="section-title">Storage Paths</h3>
        <div class="paths-grid">
          <div class="path-item" v-for="(pathInfo, key) in storagePaths" :key="key">
            <div class="path-header">
              <span class="path-label">{{ getPathLabel(key) }}</span>
              <n-tag :type="pathInfo.exists ? 'success' : 'warning'" size="small">
                {{ pathInfo.exists ? 'Exists' : 'Missing' }}
              </n-tag>
            </div>
            <div class="path-value" :title="pathInfo.path">
              {{ pathInfo.path }}
            </div>
          </div>
        </div>
      </div>

      <!-- Depot Size Section -->
      <div class="section" v-if="depotSizes">
        <h3 class="section-title">Storage Usage</h3>
        <div class="size-grid">
          <div class="size-item" v-for="(sizeInfo, key) in depotSizes" :key="key">
            <div class="size-header">
              <span class="size-label">{{ getSizeLabel(key) }}</span>
            </div>
            <div class="size-value">
              <span class="size-amount">{{ sizeInfo.size_human }}</span>
              <span class="size-path" :title="sizeInfo.path">{{ sizeInfo.path }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Actions Section -->
      <div class="section">
        <h3 class="section-title">Actions</h3>
        <div class="actions-grid">
          <div class="action-item">
            <div class="action-info">
              <h4 class="action-title">Refresh Information</h4>
              <p class="action-description">Reload storage paths and size information.</p>
            </div>
            <n-button
              size="small"
              @click="refreshInfo"
              :loading="refreshing"
              :disabled="refreshing"
            >
              Refresh
            </n-button>
          </div>
        </div>
      </div>
    </div>
  </GenericModal>
</template>

<script setup>
import { ref, onMounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { NButton, NTag, useMessage } from 'naive-ui';
import GenericModal from './GenericModal.vue';

const props = defineProps({
  show: {
    type: Boolean,
    default: false,
  },
});

const emit = defineEmits(['update:show']);

const message = useMessage();
const storagePaths = ref(null);
const depotSizes = ref(null);
const juliaVersion = ref(null);
const refreshing = ref(false);

const close = () => {
  emit('update:show', false);
};

const onUpdateShow = (val) => {
  emit('update:show', val);
};

const refreshInfo = async () => {
  refreshing.value = true;
  try {
    // Get Julia version
    const version = await invoke('get_julia_version');
    juliaVersion.value = version;

    // Get storage paths
    const paths = await invoke('get_julia_storage_paths');
    storagePaths.value = paths;

    // Get depot sizes
    const sizes = await invoke('get_depot_size_info');
    depotSizes.value = sizes;

    message.success('Information refreshed successfully');
  } catch (error) {
    console.error('Failed to refresh info:', error);
    message.error(`Failed to refresh information: ${error}`);
  } finally {
    refreshing.value = false;
  }
};

const getPathLabel = (key) => {
  const labels = {
    julia_installation: 'Julia Installation',
    juliajunction_depot: 'Compute42 Depot',
    juliajunction_env: 'Compute42 Environment',
    default_depot: 'Default Julia Depot',
    lsp_env: 'LSP Environment',
  };
  return labels[key] || key;
};

const getSizeLabel = (key) => {
  const labels = {
    juliajunction_depot: 'Compute42 Depot',
    default_depot: 'Default Julia Depot',
  };
  return labels[key] || key;
};

onMounted(() => {
  if (props.show) {
    refreshInfo();
  }
});

// Watch for show prop changes to refresh data when modal opens
watch(
  () => props.show,
  (newShow) => {
    if (newShow) {
      refreshInfo();
    }
  }
);
</script>

<style scoped>
.modal-header-content {
  text-align: center;
}

.modal-title {
  font-size: 24px;
  font-weight: 600;
  color: #e0e0e0;
  margin: 0 0 8px 0;
}

.modal-subtitle {
  font-size: 14px;
  color: #999;
  margin: 0;
}

.settings-content {
  max-height: 60vh;
  overflow-y: auto;
  text-align: left;
}

.section {
  margin-bottom: 24px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 12px;
  color: #e0e0e0;
  border-bottom: 1px solid #444;
  padding-bottom: 8px;
}

.version-info {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.version-item {
  display: flex;
  align-items: center;
  gap: 12px;
  background: #2d2d30;
  border: 1px solid #444;
  border-radius: 6px;
  padding: 12px;
}

.version-label {
  font-weight: 500;
  color: #e0e0e0;
  min-width: 120px;
}

.paths-grid {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.path-item {
  background: #2d2d30;
  border: 1px solid #444;
  border-radius: 6px;
  padding: 12px;
}

.path-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.path-label {
  font-weight: 500;
  color: #ccc;
  font-size: 14px;
}

.path-value {
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
  font-size: 12px;
  color: #999;
  word-break: break-all;
  line-height: 1.4;
}

.size-grid {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.size-item {
  background: #2d2d30;
  border: 1px solid #444;
  border-radius: 6px;
  padding: 12px;
}

.size-header {
  margin-bottom: 6px;
}

.size-label {
  font-weight: 500;
  color: #ccc;
  font-size: 14px;
}

.size-value {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.size-amount {
  font-weight: 600;
  color: #4caf50;
  font-size: 14px;
}

.size-path {
  font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
  font-size: 11px;
  color: #999;
  word-break: break-all;
  line-height: 1.3;
}

.actions-grid {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.action-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  background: #2d2d30;
  border: 1px solid #444;
  border-radius: 6px;
  padding: 16px;
}

.action-info {
  flex: 1;
  margin-right: 16px;
}

.action-title {
  font-size: 14px;
  font-weight: 600;
  color: #ccc;
  margin-bottom: 4px;
}

.action-description {
  font-size: 12px;
  color: #999;
  line-height: 1.4;
  margin: 0;
}
</style>
