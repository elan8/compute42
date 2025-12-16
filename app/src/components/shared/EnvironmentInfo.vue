<template>
  <div class="environment-info">
    <div class="env-card">
      <div class="env-header">
        <span class="env-title">Julia Environment</span>
      </div>

      <!-- All Environment Buttons -->
      <div class="environment-buttons">
        <div class="button-row">
          <!-- Julia Info Button -->
          <n-tooltip trigger="hover" placement="top">
            <template #trigger>
              <n-button size="tiny" @click="openSettings">
                <n-icon><InformationCircleOutline /></n-icon>
              </n-button>
            </template>
            Julia Info
          </n-tooltip>
        </div>
      </div>
    </div>

    <!-- Julia Info Modal -->
    <JuliaInfoModal v-model:show="showSettingsModal" />
  </div>
</template>

<script setup>
import { ref } from 'vue';
import { NIcon, NButton, NTooltip } from 'naive-ui';
import { InformationCircleOutline } from '@vicons/ionicons5';
import { useAppStore } from '../../store/appStore';
import JuliaInfoModal from './JuliaInfoModal.vue';

const appStore = useAppStore();
const showSettingsModal = ref(false);

const openSettings = () => {
  showSettingsModal.value = true;
};
</script>

<style scoped>
.environment-info {
  margin-bottom: 1px;
  border-top: 1px solid #444;
  padding-top: 1px;
  flex-shrink: 0;
}
.env-card {
  background: transparent;
  border: none;
}
.env-header {
  display: flex;
  align-items: center;
  gap: 3px;
  padding: 1px 8px;
  background: transparent;
  border-bottom: none;
}

.env-title {
  font-size: 12px;
  font-weight: 500;
  flex: 1;
  color: #ccc;
}

.env-status-indicators {
  display: flex;
  align-items: center;
  gap: 4px;
}

/* Styles for Environment Buttons */
.environment-buttons {
  padding: 4px 8px 8px 8px;
  background-color: transparent;
  border-top: 1px solid #333;
  margin-top: 4px;
}

.button-row {
  display: flex;
  justify-content: space-around;
  gap: 4px;
}

.button-row .n-button {
  flex: 1;
}
</style>
