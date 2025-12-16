<template>
  <div v-if="showWarning" class="non-julia-warning">
    <n-alert type="warning" :show-icon="false" style="margin-bottom: 8px; border-radius: 4px">
      <template #default>
        <div class="warning-content">
          <div class="warning-icon">
            <n-icon><WarningOutline /></n-icon>
          </div>
          <div class="warning-text">
            <span class="warning-title">Not a Julia project</span>
            <span class="warning-subtitle">Limited features available</span>
          </div>
          <div class="warning-action">
            <n-button size="tiny" type="primary" @click="showDetails"> Learn More </n-button>
          </div>
        </div>
      </template>
    </n-alert>

    <!-- Details Modal -->
    <n-modal
      v-model:show="showDetailsModal"
      preset="card"
      style="width: 500px"
      title="Julia Project Required"
    >
      <template #default>
        <div class="details-content">
          <div class="details-section">
            <h3>What you're missing:</h3>
            <ul class="features-list">
              <li>
                <strong>Julia Language Server:</strong> Syntax highlighting, autocomplete, and error
                detection
              </li>
              <li><strong>Package Management:</strong> Add, remove, and update Julia packages</li>
              <li>
                <strong>Code Analysis:</strong> Function and method analysis, dependency tracking
              </li>
              <li>
                <strong>LSP Features:</strong> Go to definition, find references, symbol search
              </li>
              <li>
                <strong>Project Structure:</strong> Proper Julia project organization and
                environment management
              </li>
            </ul>
          </div>

          <div class="details-section">
            <h3>Available features:</h3>
            <ul class="features-list">
              <li>File browsing and text editing</li>
              <li>Terminal access</li>
              <li>Basic file operations</li>
            </ul>
          </div>

          <div class="details-section">
            <h3>To enable full Julia features:</h3>
            <ul class="features-list">
              <li>Select a folder containing a <code>Project.toml</code> file, or</li>
              <li>Create a new Julia project using the "+" button in the file explorer</li>
            </ul>
          </div>
        </div>
      </template>

      <template #footer>
        <n-space justify="end">
          <n-button @click="showDetailsModal = false">Close</n-button>
          <n-button type="primary" @click="createNewProject"> Create New Project </n-button>
        </n-space>
      </template>
    </n-modal>
  </div>
</template>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { NAlert, NButton, NModal, NSpace, NIcon } from 'naive-ui';
import { WarningOutline } from '@vicons/ionicons5';
import { useAppStore } from '../../store/appStore';

const appStore = useAppStore();

const showDetailsModal = ref(false);

const showWarning = computed(() => {
  return appStore.initialProjectLoadAttempted && appStore.projectPath && !appStore.isJuliaProject;
});

const showDetails = () => {
  showDetailsModal.value = true;
};

const createNewProject = () => {
  showDetailsModal.value = false;
  // Emit event to trigger new project creation
  window.dispatchEvent(new CustomEvent('create-new-julia-project'));
};
</script>

<style scoped>
.non-julia-warning {
  position: relative;
}

.warning-content {
  display: flex;
  align-items: center;
  gap: 8px;
}

.warning-icon {
  color: #f59e0b;
  font-size: 14px;
  flex-shrink: 0;
}

.warning-text {
  display: flex;
  flex-direction: column;
  flex-grow: 1;
  min-width: 0;
}

.warning-title {
  font-size: 12px;
  font-weight: 600;
  color: #f59e0b;
  line-height: 1.2;
}

.warning-subtitle {
  font-size: 11px;
  color: #9ca3af;
  line-height: 1.2;
}

.warning-action {
  flex-shrink: 0;
}

.details-content {
  color: #e5e7eb;
}

.details-section {
  margin-bottom: 20px;
}

.details-section h3 {
  font-size: 14px;
  font-weight: 600;
  margin: 0 0 8px 0;
  color: #f3f4f6;
}

.features-list {
  margin: 0;
  padding-left: 16px;
  font-size: 13px;
  line-height: 1.4;
}

.features-list li {
  margin-bottom: 4px;
  color: #d1d5db;
}

.features-list strong {
  color: #f3f4f6;
}

.features-list code {
  background-color: #374151;
  padding: 2px 4px;
  border-radius: 3px;
  font-size: 12px;
  color: #fbbf24;
}
</style>
