<template>
  <n-modal
    v-model:show="show"
    preset="card"
    style="width: 600px"
    title="Project.toml Configuration"
    :closable="false"
  >
    <n-form
      ref="formRef"
      :model="formValue"
      :rules="rules"
      label-placement="left"
      label-width="auto"
      require-mark-placement="right-hanging"
    >
      <n-form-item label="Project Name" path="name">
        <n-input v-model:value="formValue.name" placeholder="Enter project name" />
      </n-form-item>

      <n-form-item label="Version" path="version">
        <n-input v-model:value="formValue.version" placeholder="0.1.0" />
      </n-form-item>

      <n-form-item label="Description" path="description">
        <n-input
          v-model:value="formValue.description"
          type="textarea"
          placeholder="Enter project description"
        />
      </n-form-item>

      <n-form-item label="Authors" path="authors">
        <n-input
          v-model:value="formValue.authors"
          type="textarea"
          placeholder="Author Name <email@example.com>&#10;Another Author <another@example.com>"
        />
      </n-form-item>

      <n-form-item label="UUID" path="uuid">
        <div style="display: flex; gap: 8px; align-items: center">
          <n-input
            v-model:value="formValue.uuid"
            placeholder="UUID will be generated automatically"
            readonly
          />
          <n-button size="small" @click="regenerateUuid" :disabled="isBackendBusy">
            Regenerate
          </n-button>
        </div>
      </n-form-item>
    </n-form>

    <template #footer>
      <n-space justify="end">
        <n-button @click="show = false" :disabled="isBackendBusy">Cancel</n-button>
        <n-button type="primary" @click="handleSubmit" :loading="loading" :disabled="isBackendBusy">
          Save Configuration
        </n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<script setup>
import { ref, watch, computed } from 'vue';
import { useMessage } from 'naive-ui';
import { invoke } from '@tauri-apps/api/core';
import { NModal, NForm, NFormItem, NInput, NButton, NDivider, NSpace, NText } from 'naive-ui';
import { useAppStore } from '../../store/appStore';

const message = useMessage();
const appStore = useAppStore();
const show = ref(false);
const loading = ref(false);
const currentProjectPath = ref('');

const emit = defineEmits(['configurationSaved']);

// Disable functionality when backend is busy
const isBackendBusy = computed(() => appStore.backendBusy);

const formRef = ref(null);
const formValue = ref({
  name: '',
  version: '0.1.0',
  description: '',
  authors: '',
  uuid: '',
});

const rules = {
  // Project.toml fields are not mandatory, so we don't require them
  // Users can save an empty or partially filled Project.toml
};

const loadProjectToml = async (projectPath) => {
  try {
    const config = await invoke('read_project_toml', { projectPath });
    if (config) {
      formValue.value = {
        name: config.name || '',
        version: config.version || '0.1.0',
        description: config.description || '',
        authors: config.authors || '',
        uuid: config.uuid || '',
      };
    }
  } catch (error) {
    console.error('Failed to load Project.toml:', error);

    // If the file doesn't exist or is invalid, just initialize with empty/default values
    // This allows users to create a new Project.toml or fix an existing one
    formValue.value = {
      name: '',
      version: '0.1.0',
      description: '',
      authors: '',
      uuid: '',
    };

    // Show a gentle info message instead of an error
    if (error.toString().includes('does not exist')) {
      message.info('No Project.toml found. You can create one by filling out the form below.');
    } else {
      message.info(
        'Project.toml could not be loaded. You can edit it by filling out the form below.'
      );
    }
  }
};

const handleSubmit = async () => {
  try {
    loading.value = true;
    // No form validation needed since Project.toml fields are not mandatory

    const config = {
      name: formValue.value.name,
      version: formValue.value.version,
      description: formValue.value.description,
      authors: formValue.value.authors,
      uuid: formValue.value.uuid,
      project_path: currentProjectPath.value,
    };

    await invoke('write_project_toml', { config });

    message.success('Project.toml configuration saved successfully');
    show.value = false;
    emit('configurationSaved', config);
  } catch (error) {
    message.error(`Failed to save configuration: ${error.toString()}`);
  } finally {
    loading.value = false;
  }
};

const open = async (projectPath) => {
  show.value = true;
  currentProjectPath.value = projectPath;
  await loadProjectToml(projectPath);
};

const regenerateUuid = async () => {
  try {
    loading.value = true;
    const newUuid = await invoke('generate_uuid');
    formValue.value.uuid = newUuid;
    message.success('UUID regenerated successfully');
  } catch (error) {
    message.error(`Failed to regenerate UUID: ${error.toString()}`);
  } finally {
    loading.value = false;
  }
};

defineExpose({
  show,
  open,
});
</script>

<style scoped>
.dependencies-section {
  margin-bottom: 16px;
}

.dependency-row {
  display: flex;
  gap: 8px;
  margin-bottom: 8px;
  align-items: center;
}

.mt-2 {
  margin-top: 8px;
}
</style>
