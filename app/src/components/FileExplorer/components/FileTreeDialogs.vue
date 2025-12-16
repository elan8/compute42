<template>
  <!-- Create File/Folder Dialog -->
  <n-modal
    :show="createDialogVisible"
    @update:show="$emit('create-dialog-update', $event)"
    preset="card"
    style="width: 400px"
    :title="createDialogTitle"
  >
    <n-form
      ref="createFormRef"
      :model="createFormValue"
      :rules="createFormRules"
      label-placement="left"
      label-width="auto"
      require-mark-placement="right-hanging"
    >
      <n-form-item label="Name" path="name">
        <n-input v-model:value="createFormValue.name" :placeholder="createFormPlaceholder" />
      </n-form-item>
    </n-form>

    <template #footer>
      <n-space justify="end">
        <n-button @click="$emit('create-cancel')">Cancel</n-button>
        <n-button type="primary" @click="$emit('create-submit')" :loading="createLoading">
          Create
        </n-button>
      </n-space>
    </template>
  </n-modal>

  <!-- Rename Dialog -->
  <n-modal
    :show="renameDialogVisible"
    @update:show="$emit('rename-dialog-update', $event)"
    preset="card"
    style="width: 400px"
    title="Rename Item"
  >
    <n-form
      ref="renameFormRef"
      :model="renameFormValue"
      :rules="renameFormRules"
      label-placement="left"
      label-width="auto"
      require-mark-placement="right-hanging"
    >
      <n-form-item label="New Name" path="name">
        <n-input v-model:value="renameFormValue.name" placeholder="Enter new name" />
      </n-form-item>
    </n-form>

    <template #footer>
      <n-space justify="end">
        <n-button @click="$emit('rename-cancel')">Cancel</n-button>
        <n-button type="primary" @click="$emit('rename-submit')" :loading="renameLoading">
          Rename
        </n-button>
      </n-space>
    </template>
  </n-modal>

  <!-- Delete Confirmation Dialog -->
  <n-modal
    :show="deleteDialogVisible"
    @update:show="$emit('delete-dialog-update', $event)"
    preset="card"
    style="width: 400px"
    title="Confirm Delete"
  >
    <div style="margin-bottom: 16px">
      <p>Are you sure you want to delete this item?</p>
      <p style="color: #999; font-size: 12px; margin-top: 8px">
        <strong>Name:</strong> {{ deleteTarget?.name }}
      </p>
      <p style="color: #999; font-size: 12px; margin-top: 4px">
        <strong>Type:</strong> {{ deleteTarget?.is_directory ? 'Folder' : 'File' }}
      </p>
      <p style="color: #999; font-size: 12px; margin-top: 4px">
        <strong>Path:</strong> {{ deleteTarget?.path }}
      </p>
      <p style="color: #ff6b6b; font-size: 12px; margin-top: 8px; font-weight: bold">
        ⚠️ This action cannot be undone!
      </p>
    </div>

    <template #footer>
      <n-space justify="end">
        <n-button @click="$emit('delete-cancel')">Cancel</n-button>
        <n-button type="error" @click="$emit('delete-confirm')" :loading="deleteLoading">
          Delete
        </n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { NModal, NForm, NFormItem, NInput, NButton, NSpace } from 'naive-ui';

// Import types
import type { FileNode } from '../../../types/fileTree';

// ============================
// Props
// ============================

interface Props {
  createDialogVisible: boolean;
  createDialogType: 'julia-file' | 'jupyter-notebook' | 'folder';
  createDialogTitle: string;
  createFormValue: { name: string };
  createLoading: boolean;
  renameDialogVisible: boolean;
  renameFormValue: { name: string };
  renameLoading: boolean;
  deleteDialogVisible: boolean;
  deleteTarget: FileNode | null;
  deleteLoading: boolean;
}

const props = defineProps<Props>();

// ============================
// Emits
// ============================

const emit = defineEmits<{
  'create-submit': [];
  'create-cancel': [];
  'create-dialog-update': [value: boolean];
  'rename-submit': [];
  'rename-cancel': [];
  'rename-dialog-update': [value: boolean];
  'delete-confirm': [];
  'delete-cancel': [];
  'delete-dialog-update': [value: boolean];
}>();

// ============================
// Computed Properties
// ============================

const createFormPlaceholder = computed(() => {
  if (props.createDialogType === 'julia-file') {
    return 'filename.jl';
  } else if (props.createDialogType === 'jupyter-notebook') {
    return 'notebook.ipynb';
  } else {
    return 'folder name';
  }
});

const createFormRules = {
  name: {
    required: true,
    message: 'Please enter a name',
    trigger: 'blur',
  },
};

const renameFormRules = {
  name: {
    required: true,
    message: 'Please enter a name',
    trigger: 'blur',
  },
};
</script>

<style scoped>
/* Dialog styles are handled by Naive UI */
</style>
