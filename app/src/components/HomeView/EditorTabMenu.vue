<template>
  <div class="editor-tab-menu">
    <n-space
      align="center"
      justify="space-between"
      style="padding: 8px 12px; border-bottom: 1px solid #333"
    >
      <div class="menu-left">
        <n-button
          v-if="isJuliaFile"
          size="small"
          type="primary"
          :loading="isBackendBusy"
          :disabled="isBackendBusy"
          @click="runFile"
          style="margin-right: 8px"
        >
          <template #icon>
            <n-icon><PlayCircleOutline /></n-icon>
          </template>
        </n-button>
        <n-button
          v-if="props.viewerType === 'notebook' && props.onRunAllCells"
          size="small"
          type="primary"
          :loading="isBackendBusy"
          :disabled="isBackendBusy"
          @click="runAllCells"
          style="margin-right: 8px"
        >
          <template #icon>
            <n-icon><PlayCircleOutline /></n-icon>
          </template>
          Run All
        </n-button>
        <n-button size="small" @click="saveFile" :disabled="!isDirty" style="margin-right: 8px">
          <template #icon>
            <n-icon><Save /></n-icon>
          </template>
        </n-button>
      </div>
    </n-space>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { NSpace, NButton, NIcon, NTooltip } from 'naive-ui';
import { PlayCircleOutline, Save } from '@vicons/ionicons5';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { debug, error } from '../../utils/logger';
import { useAppStore } from '../../store/appStore';

// Extend Window interface to include our custom property
declare global {
  interface Window {
    currentExecutingFile?: string | null;
  }
}

interface Props {
  filePath: string;
  fileContent: string;
  language: string;
  isDirty: boolean;
  viewerType?: 'monaco' | 'image' | 'document' | 'binary' | 'csv' | 'notebook';
  onRunFile: () => void;
  onRunAllCells?: () => void;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  save: [];
  runFile: [];
  runStarted: [];
  runCompleted: [];
  runError: [error: string];
  runOutput: [output: string];
  variablesUpdated: [variables: Record<string, any>];
}>();

const appStore = useAppStore();

const isJuliaFile = computed(() => {
  return props.language === 'julia' || props.filePath.endsWith('.jl');
});

// Use centralized backend busy state as single source of truth
const isBackendBusy = computed(() => {
  return appStore.getBackendBusyStatus();
});

onMounted(async () => {
  debug('EditorTabMenu: Component mounted');
});

const runFile = async () => {
  props.onRunFile();
};

const runAllCells = async () => {
  if (props.onRunAllCells) {
    props.onRunAllCells();
  }
};

const saveFile = () => {
  emit('save');
};
</script>

<style scoped>
.editor-tab-menu {
  background-color: #2d2d30;
  border-bottom: 1px solid #3e3e42;
}

.menu-left {
  display: flex;
  align-items: center;
}

.menu-right {
  display: flex;
  align-items: center;
}
</style>
