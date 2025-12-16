<template>
  <GenericModal :show="show" @update:show="onUpdateShow" width="800px" height="600px">
    <template #header>
      <div class="modal-header-content">
        <h2 class="modal-title">Language Server Status</h2>
      </div>
    </template>

    <div class="status-content">
      <!-- Current Status Section -->
      <div class="section">
        <h3 class="section-title">Current Status</h3>
        <div class="status-info">
          <div class="status-item">
            <div class="status-header">
              <span class="status-label">Status:</span>
              <n-tag :type="statusTagType" size="small">
                {{ lspStatus.status || 'Unknown' }}
              </n-tag>
            </div>
            <div class="status-message" v-if="lspStatus.message">
              {{ lspStatus.message }}
            </div>
            <div class="status-error" v-if="lspStatus.error">
              <strong>Error:</strong> {{ lspStatus.error }}
            </div>
          </div>
        </div>
      </div>
    </div>
  </GenericModal>
</template>

<script setup>
import { computed } from 'vue';
import { NTag } from 'naive-ui';
import { useAppStore } from '../../store/appStore';
import GenericModal from './GenericModal.vue';

const props = defineProps({
  show: {
    type: Boolean,
    default: false,
  },
});

const emit = defineEmits(['update:show']);

const appStore = useAppStore();

const lspStatus = computed(() => appStore.lspStatus);

const statusTagType = computed(() => {
  const status = lspStatus.value.status;
  switch (status) {
    case 'ready':
      return 'success';
    case 'starting':
    case 'started':
    case 'initialized':
    case 'loading-cache':
      return 'info';
    case 'failed':
      return 'error';
    case 'stopped':
      return 'warning';
    default:
      return 'default';
  }
});

const close = () => {
  emit('update:show', false);
};

const onUpdateShow = (val) => {
  emit('update:show', val);
};
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

.status-content {
  text-align: left;
}

.section {
  margin-bottom: 24px;
}

.section:last-child {
  margin-bottom: 0;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.section-title {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 12px;
  color: #e0e0e0;
  border-bottom: 1px solid #444;
  padding-bottom: 8px;
}

.status-info {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.status-item {
  background: #2d2d30;
  border: 1px solid #444;
  border-radius: 6px;
  padding: 12px;
}

.status-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
}

.status-label {
  font-weight: 500;
  color: #e0e0e0;
  min-width: 60px;
}

.status-message {
  font-size: 14px;
  color: #ccc;
  margin-bottom: 4px;
}

.status-error {
  font-size: 12px;
  color: #ff6b6b;
  background: rgba(255, 107, 107, 0.1);
  padding: 8px;
  border-radius: 4px;
  border-left: 3px solid #ff6b6b;
}
</style>
