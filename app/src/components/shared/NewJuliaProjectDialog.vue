<template>
  <n-modal v-model:show="show" preset="card" style="width: 500px" title="Create New Julia Project">
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

      <n-form-item label="Target Directory" path="targetDir">
        <n-input-group>
          <n-input
            v-model:value="formValue.targetDir"
            placeholder="Select target directory"
            readonly
          />
          <n-button @click="selectDirectory">Browse</n-button>
        </n-input-group>
      </n-form-item>

      <n-form-item label="Description" path="description">
        <n-input
          v-model:value="formValue.description"
          placeholder="Brief description of your project"
        />
      </n-form-item>

      <n-form-item label="Authors" path="authors">
        <n-input
          v-model:value="formValue.authors"
          placeholder="Your name <your.email@example.com>"
        />
      </n-form-item>
    </n-form>

    <template #footer>
      <n-space justify="end">
        <n-button @click="show = false">Cancel</n-button>
        <n-button type="primary" @click="handleSubmit" :loading="loading">
          Create Project
        </n-button>
      </n-space>
    </template>
  </n-modal>
</template>

<script setup>
import { ref } from 'vue';
import { useMessage } from 'naive-ui';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { NModal, NForm, NFormItem, NInput, NInputGroup, NButton, NSpace } from 'naive-ui';

const message = useMessage();
const show = ref(false);
const loading = ref(false);

const emit = defineEmits(['projectRootChanged']);

const formRef = ref(null);
const formValue = ref({
  name: '',
  version: '0.1.0',
  targetDir: '',
  description: '',
  authors: '',
});

const rules = {
  name: {
    required: true,
    message: 'Please enter project name',
    trigger: 'blur',
  },
  version: {
    required: true,
    message: 'Please enter version',
    trigger: 'blur',
  },
  targetDir: {
    required: true,
    message: 'Please select target directory',
    trigger: 'blur',
  },
};

const selectDirectory = async () => {
  const selected = await open({
    directory: true,
    multiple: false,
  });
  if (selected) {
    formValue.value.targetDir = selected;
  }
};

const handleSubmit = async () => {
  try {
    loading.value = true;
    await formRef.value?.validate();

    // Call the new backend command to create a Julia project
    const result = await invoke('create_new_julia_project', {
      projectPath: formValue.value.targetDir,
      projectName: formValue.value.name,
      authors: formValue.value.authors.trim() || null,
    });

    message.success('Julia project created successfully');
    show.value = false;

    // Emit the new project root path
    const newProjectPath =
      formValue.value.targetDir.replace(/\\/g, '/') + '/' + formValue.value.name;
    emit('projectRootChanged', newProjectPath);
  } catch (error) {
    message.error(error.toString());
  } finally {
    loading.value = false;
  }
};

defineExpose({
  show,
});
</script>

<style scoped>
/* No additional styles needed for this simplified dialog */
</style>
