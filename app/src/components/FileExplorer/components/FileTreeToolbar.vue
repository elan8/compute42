<template>
  <n-space justify="space-between" align="center" class="file-tree-toolbar">
    <n-text class="project-path" :title="projectPath">
      {{ projectPathShort || 'No Folder Selected' }}
    </n-text>

    <n-space>
      <n-tooltip trigger="hover">
        <template #trigger>
          <n-button size="tiny" circle @click="$emit('select-folder')">
            <n-icon><FolderOpenOutline /></n-icon>
          </n-button>
        </template>
        Select Root Folder
      </n-tooltip>

      <n-tooltip trigger="hover">
        <template #trigger>
          <n-button size="tiny" circle @click="$emit('new-project')">
            <n-icon><AddOutline /></n-icon>
          </n-button>
        </template>
        New Julia Project
      </n-tooltip>

      <n-tooltip trigger="hover">
        <template #trigger>
          <n-button size="tiny" circle @click="$emit('project-config')" :disabled="!projectPath">
            <n-icon><SettingsOutline /></n-icon>
          </n-button>
        </template>
        Project.toml Configuration
      </n-tooltip>

      <n-tooltip trigger="hover">
        <template #trigger>
          <n-button
            size="tiny"
            circle
            @click="$emit('instantiate-dependencies')"
            :disabled="!projectPath || isInstalling"
            :loading="isInstalling"
          >
            <n-icon><CloudDownloadOutline /></n-icon>
          </n-button>
        </template>
        Install Project Dependencies
      </n-tooltip>
    </n-space>
  </n-space>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { NSpace, NButton, NTooltip, NText, NIcon } from 'naive-ui';
import {
  FolderOpenOutline,
  AddOutline,
  SettingsOutline,
  CloudDownloadOutline,
} from '@vicons/ionicons5';

// ============================
// Props
// ============================

interface Props {
  projectPath?: string | null;
  isInstalling?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  projectPath: null,
  isInstalling: false,
});

// ============================
// Emits
// ============================

const emit = defineEmits<{
  'select-folder': [];
  'new-project': [];
  'project-config': [];
  'instantiate-dependencies': [];
}>();

// ============================
// Computed Properties
// ============================

const projectPathShort = computed(() => {
  if (!props.projectPath) return null;

  // Extract the last part of the path
  const parts = props.projectPath.split(/\\|\//);
  return parts[parts.length - 1] || props.projectPath;
});
</script>

<style scoped>
.file-tree-toolbar {
  padding: 1px 8px;
  border-bottom: 1px solid #444;
  flex-shrink: 0;
}

.project-path {
  color: #ccc;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  flex-grow: 1;
  margin-right: 10px;
  padding: 1px 0;
}
</style>
