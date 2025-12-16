<template>
  <div class="file-tree-search">
    <n-input
      v-model:value="localSearchQuery"
      placeholder="Search files..."
      clearable
      @input="handleSearch"
      @clear="handleClear"
    >
      <template #prefix>
        <n-icon><SearchOutline /></n-icon>
      </template>
    </n-input>
  </div>
</template>

<script setup lang="ts">
import { ref, watch } from 'vue';
import { NInput, NIcon } from 'naive-ui';
import { SearchOutline } from '@vicons/ionicons5';

// ============================
// Props
// ============================

interface Props {
  modelValue?: string;
}

const props = withDefaults(defineProps<Props>(), {
  modelValue: '',
});

// ============================
// Emits
// ============================

const emit = defineEmits<{
  'update:modelValue': [value: string];
  search: [query: string];
}>();

// ============================
// State
// ============================

const localSearchQuery = ref<string>(props.modelValue);

// ============================
// Event Handlers
// ============================

const handleSearch = (value: string) => {
  emit('update:modelValue', value);
  emit('search', value);
};

const handleClear = () => {
  localSearchQuery.value = '';
  emit('update:modelValue', '');
  emit('search', '');
};

// ============================
// Watchers
// ============================

watch(
  () => props.modelValue,
  (newValue) => {
    localSearchQuery.value = newValue;
  }
);
</script>

<style scoped>
.file-tree-search {
  padding: 8px;
  border-bottom: 1px solid #444;
  flex-shrink: 0;
}
</style>
