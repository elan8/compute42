<template>
  <div style="height: 100%; display: flex; flex-direction: column; background-color: #282828">
    <div style="flex-grow: 1; overflow: auto; display: flex; flex-direction: column">
      <n-collapse
        v-model:expanded-names="expandedNames"
        :default-expanded-names="defaultExpandedNames"
        style="flex-grow: 1; display: flex; flex-direction: column"
      >
        <!-- File Explorer Section (always visible) -->
        <n-collapse-item name="explorer" title="Explorer">
          <template #header>
            <div class="accordion-header">
              <n-icon><FolderOutline /></n-icon>
              <span class="header-text">Explorer</span>
            </div>
          </template>
          <div
            style="
              flex-grow: 1;
              display: flex;
              flex-direction: column;
              min-height: 0;
              height: 100%;
              overflow: hidden;
            "
          >
            <FileExplorer
              @open-file="handleOpenFile"
              @open-package-settings="handleOpenPackageSettings"
              @project-root-changed="handleProjectRootChanged"
            />
          </div>
        </n-collapse-item>

        <!-- Plot Library Section -->
        <n-collapse-item name="plots" title="Plots">
          <template #header>
            <div class="accordion-header">
              <n-icon><TrendingUpOutline /></n-icon>
              <span class="header-text">Plots</span>
              <n-badge v-if="plotCount > 0" :value="plotCount" :max="99" class="header-badge" />
            </div>
          </template>
          <div style="flex-grow: 1; display: flex; flex-direction: column; min-height: 0">
            <PlotLibrary />
          </div>
        </n-collapse-item>

        <!-- Variables Section -->
        <n-collapse-item name="variables" title="Variables">
          <template #header>
            <div class="accordion-header">
              <n-icon><CodeOutline /></n-icon>
              <span class="header-text">Variables</span>
            </div>
          </template>
          <div
            style="
              flex-grow: 1;
              display: flex;
              flex-direction: column;
              min-height: 0;
              height: 100%;
              overflow: hidden;
            "
          >
            <VariablesPanel :is-debugging="isDebugging" />
          </div>
        </n-collapse-item>
      </n-collapse>
    </div>

    <!-- Environment Info - Always visible at bottom -->
    <div style="flex-shrink: 0; border-top: 1px solid #444">
      <EnvironmentInfo />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { NCollapse, NCollapseItem, NBadge, NIcon, NButton } from 'naive-ui';
import { FolderOutline, TrendingUpOutline, CodeOutline } from '@vicons/ionicons5';
import { FileExplorer } from '../FileExplorer';
import PlotLibrary from '../shared/PlotLibrary.vue';
import EnvironmentInfo from '../shared/EnvironmentInfo.vue';
import VariablesPanel from '../HomeView/VariablesPanel.vue';
import { usePlotStore } from '../../store/plotStore';
import { useAppStore } from '../../store/appStore';

const expandedNames = ref(['explorer']); // Start with only explorer expanded
const isDebugging = ref(false); // Always false in open-source (no debug functionality)

// Computed property for default expanded names
const defaultExpandedNames = computed(() => {
  return ['explorer']; // Only explorer expanded by default
});

const plotStore = usePlotStore();
const plotCount = computed(() => plotStore.plotCount);
const appStore = useAppStore();

// Plot listening is now initialized globally in the plot store

// Event handlers for FileExplorer
const emit = defineEmits(['open-file', 'open-package-settings', 'project-root-changed']);

const handleOpenFile = (filePath) => {
  emit('open-file', filePath);
};

const handleOpenPackageSettings = (projectPath) => {
  emit('open-package-settings', projectPath);
};

const handleProjectRootChanged = (newRoot) => {
  emit('project-root-changed', newRoot);
};

onMounted(async () => {
  // Component mounted
});
</script>

<style scoped>
:deep(.n-collapse-item__header) {
  background-color: #1e1e1e !important;
  border-bottom: 1px solid #444 !important;
  margin: 0 !important;
  padding: 0 12px !important;
}

:deep(.n-collapse-item__header:hover) {
  background-color: #252525 !important;
}

:deep(.n-collapse-item__content) {
  background-color: #282828 !important;
  padding: 0 !important;
}

:deep(.n-collapse-item__content-box) {
  padding: 0 !important;
}

/* Only expanded items should have flex-grow on content */
:deep(.n-collapse-item--expanded .n-collapse-item__content) {
  flex-grow: 1 !important;
  display: flex !important;
  flex-direction: column !important;
}

:deep(.n-collapse-item--expanded .n-collapse-item__content-box) {
  flex-grow: 1 !important;
  display: flex !important;
  flex-direction: column !important;
}

/* Make the explorer section take up maximum available space */
:deep(.n-collapse-item[name='explorer'].n-collapse-item--expanded) {
  flex-grow: 1 !important;
  min-height: 0 !important;
}

/* Ensure the explorer takes all available space when expanded */
:deep(
  .n-collapse-item[name='explorer'].n-collapse-item--expanded .n-collapse-item__content-wrapper
) {
  flex-grow: 1 !important;
  min-height: 0 !important;
}

/* Make the plots section take minimal space when collapsed */
:deep(.n-collapse-item[name='plots']) {
  flex-shrink: 0 !important;
}

/* When plots is expanded, it should take minimal space */
:deep(.n-collapse-item[name='plots'].n-collapse-item--expanded) {
  flex-shrink: 0 !important;
  flex-grow: 0 !important;
}

:deep(.n-collapse) {
  background-color: #282828 !important;
  border: none !important;
  gap: 0 !important;
  display: flex !important;
  flex-direction: column !important;
  height: 100% !important;
  min-height: 0 !important;
}

/* Remove any spacing between collapse items */
:deep(.n-collapse-item) {
  margin: 0 !important;
  border: none !important;
  display: flex !important;
  flex-direction: column !important;
}

/* Expanded collapse items should fill available space */
:deep(.n-collapse-item--expanded) {
  flex-grow: 1 !important;
  min-height: 0 !important;
}

/* Ensure proper flex distribution */
:deep(.n-collapse-item) {
  display: flex !important;
  flex-direction: column !important;
}

/* Collapsed collapse items should take minimal space but still be interactive */
:deep(.n-collapse-item:not(.n-collapse-item--expanded)) {
  flex-shrink: 0 !important;
  flex-grow: 0 !important;
  min-height: 32px !important; /* Ensure header is still clickable */
}

/* When explorer is collapsed, plots should take the space */
:deep(.n-collapse-item[name='explorer']:not(.n-collapse-item--expanded)) {
  flex-grow: 0 !important;
  flex-shrink: 0 !important;
}

/* When explorer is collapsed, plots should expand to fill space */
:deep(
  .n-collapse-item[name='explorer']:not(.n-collapse-item--expanded) + .n-collapse-item[name='plots']
) {
  flex-grow: 1 !important;
}

:deep(.n-collapse-item + .n-collapse-item) {
  margin-top: 0 !important;
  border-top: none !important;
}

/* Accordion header styling */
.accordion-header {
  display: flex;
  align-items: center;
  gap: 8px;
  height: 32px; /* Ensure consistent height */
  width: 100%;
}

.header-icon {
  color: #ccc !important; /* Same color as text */
  font-size: 14px;
  flex-shrink: 0;
}

.header-text {
  color: #ccc;
  font-size: 12px;
  font-weight: 500;
  flex: 1;
}

.header-badge {
  flex-shrink: 0;
}

.header-button {
  margin-left: auto;
  flex-shrink: 0;
}

/* Ensure all collapse item headers have the same height */
:deep(.n-collapse-item__header) {
  min-height: 32px !important;
  height: 32px !important;
  padding: 0 12px !important;
}

/* Ensure collapsed content takes no space */
:deep(.n-collapse-item__content-wrapper) {
  margin: 0 !important;
  padding: 0 !important;
}

/* Only expanded items should have flex properties on content wrapper */
:deep(.n-collapse-item--expanded .n-collapse-item__content-wrapper) {
  flex-grow: 1 !important;
  display: flex !important;
  flex-direction: column !important;
}

/* Explorer content wrapper should take maximum space */
:deep(
  .n-collapse-item[name='explorer'].n-collapse-item--expanded .n-collapse-item__content-wrapper
) {
  flex-grow: 1 !important;
  min-height: 0 !important;
}

/* Plots content wrapper should take minimal space when expanded */
:deep(.n-collapse-item[name='plots'].n-collapse-item--expanded .n-collapse-item__content-wrapper) {
  flex-shrink: 0 !important;
  flex-grow: 0 !important;
}

/* Plots content wrapper should take minimal space when collapsed */
:deep(
  .n-collapse-item[name='plots']:not(.n-collapse-item--expanded) .n-collapse-item__content-wrapper
) {
  height: 0 !important;
  overflow: hidden !important;
}

/* Explorer content wrapper should take minimal space when collapsed */
:deep(
  .n-collapse-item[name='explorer']:not(.n-collapse-item--expanded)
    .n-collapse-item__content-wrapper
) {
  height: 0 !important;
  overflow: hidden !important;
}

:deep(.n-collapse-item__content) {
  margin: 0 !important;
  padding: 0 !important;
  flex-grow: 1 !important;
  display: flex !important;
  flex-direction: column !important;
}

:deep(.n-collapse-item__header-content) {
  height: 100% !important;
  display: flex !important;
  align-items: center !important;
}

/* Ensure FileExplorer takes up all available space */
:deep(.n-collapse-item[name='explorer'] .n-collapse-item__content > div) {
  height: 100% !important;
  flex-grow: 1 !important;
  min-height: 0 !important;
  overflow: hidden !important;
}

/* Ensure FileExplorer component itself takes full height */
:deep(.n-collapse-item[name='explorer'] .n-collapse-item__content > div > *) {
  height: 100% !important;
  flex-grow: 1 !important;
  min-height: 0 !important;
}
</style>
