<template>
  <div class="file-tree-breadcrumbs" v-if="breadcrumbs.length > 0">
    <n-breadcrumb>
      <n-breadcrumb-item
        v-for="(crumb, index) in breadcrumbs"
        :key="crumb.path"
        :clickable="index < breadcrumbs.length - 1"
        @click="handleNavigate(crumb.path)"
      >
        <n-icon v-if="index === 0">
          <HomeOutline />
        </n-icon>
        {{ crumb.name }}
      </n-breadcrumb-item>
    </n-breadcrumb>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { NBreadcrumb, NBreadcrumbItem, NIcon } from 'naive-ui';
import { HomeOutline } from '@vicons/ionicons5';

// ============================
// Props
// ============================

interface Props {
  currentPath?: string | null;
}

const props = withDefaults(defineProps<Props>(), {
  currentPath: null,
});

// ============================
// Emits
// ============================

const emit = defineEmits<{
  navigate: [path: string];
}>();

// ============================
// Computed Properties
// ============================

const breadcrumbs = computed(() => {
  if (!props.currentPath) return [];

  const parts = props.currentPath.split(/\\|\//).filter((part) => part.length > 0);
  const breadcrumbs = [];

  let currentPath = '';
  for (let i = 0; i < parts.length; i++) {
    currentPath += (currentPath ? '/' : '') + parts[i];
    breadcrumbs.push({
      name: parts[i],
      path: currentPath,
    });
  }

  return breadcrumbs;
});

// ============================
// Event Handlers
// ============================

const handleNavigate = (path: string) => {
  emit('navigate', path);
};
</script>

<style scoped>
.file-tree-breadcrumbs {
  padding: 4px 8px;
  border-bottom: 1px solid #444;
  flex-shrink: 0;
  background-color: #2a2a2a;
}

:deep(.n-breadcrumb-item) {
  color: #ccc;
}

:deep(.n-breadcrumb-item--clickable) {
  cursor: pointer;
}

:deep(.n-breadcrumb-item--clickable:hover) {
  color: #fff;
}
</style>
