<template>
  <n-space
    vertical
    align="center"
    justify="start"
    style="padding-top: 10px; height: 100%; background-color: #282828"
  >
    <n-tooltip placement="right" trigger="hover">
      <template #trigger>
        <n-button
          quaternary
          :type="selectedView === 'explorer' ? 'primary' : 'default'"
          @click="selectView('explorer')"
          style="width: 48px; height: 48px"
        >
          <n-icon size="24">
            <FolderOpenOutline />
          </n-icon>
        </n-button>
      </template>
      Explorer
    </n-tooltip>

    <n-tooltip placement="right" trigger="hover">
      <template #trigger>
        <n-button
          quaternary
          :type="selectedView === 'packages' ? 'primary' : 'default'"
          @click="selectView('packages')"
          style="width: 48px; height: 48px"
        >
          <n-icon size="24">
            <CubeOutline />
          </n-icon>
        </n-button>
      </template>
      Package Management
    </n-tooltip>

    <n-tooltip placement="right" trigger="hover">
      <template #trigger>
        <n-button
          quaternary
          :type="selectedView === 'settings' ? 'primary' : 'default'"
          @click="selectView('settings')"
          style="width: 48px; height: 48px"
        >
          <n-icon size="24">
            <SettingsOutline />
          </n-icon>
        </n-button>
      </template>
      Settings
    </n-tooltip>

    <n-tooltip placement="right" trigger="hover">
      <template #trigger>
        <n-button
          quaternary
          :type="selectedView === 'about' ? 'primary' : 'default'"
          @click="selectView('about')"
          style="width: 48px; height: 48px"
        >
          <n-icon size="24">
            <InformationCircleOutline />
          </n-icon>
        </n-button>
      </template>
      Help & About
    </n-tooltip>

    <!-- Add more icons later -->
  </n-space>
</template>

<script setup>
import { ref, computed, watch } from 'vue';
import { useRouter, useRoute } from 'vue-router';
import { NButton, NSpace, NTooltip, NIcon } from 'naive-ui';
import {
  FolderOpenOutline,
  InformationCircleOutline,
  CubeOutline,
  SettingsOutline,
} from '@vicons/ionicons5';
import { useAppStore } from '../../store/appStore';
import { debug, info, warn } from '../../utils/logger';
import { primaryColor, primaryColorHover } from '../../theme';

const router = useRouter();
const route = useRoute();
const appStore = useAppStore();
const emit = defineEmits(['navigate']);

// Compute selected view based on current route
const selectedView = computed(() => {
  switch (route.name) {
    case 'Home':
      return 'explorer';
    case 'PackageManagement':
      return 'packages';
    case 'Settings':
      return 'settings';
    case 'About':
      return 'about';
    default:
      return 'explorer';
  }
});

const selectView = (view) => {
  debug(`NavigationRail: Selecting view: ${view}`);
  console.log(`NavigationRail: Selecting view: ${view}`);
  emit('navigate', view);
};

// Expose the selectView method to parent components
defineExpose({
  selectView,
});
</script>

<style scoped>
/* Scoped styles if needed */
.n-button {
  margin-bottom: 8px; /* Spacing between icons */
}
.n-button .n-icon {
  color: v-bind(primaryColor) !important; /* Use theme color */
}

.n-button:hover .n-icon {
  color: v-bind(primaryColorHover) !important; /* Use theme hover color */
}
</style>
