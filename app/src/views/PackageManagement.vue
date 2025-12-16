<template>
  <div class="package-management-view">
    <!-- Header Section -->
    <div class="header-section">
      <div class="header-content">
        <div class="header-left">
          <h1 class="page-title">
            <n-icon class="title-icon">
              <Cube />
            </n-icon>
            Package Management
          </h1>
        </div>
        <div class="header-actions">
          <n-button @click="refreshPackages" :loading="loading" size="medium">
            <template #icon>
              <n-icon><Refresh /></n-icon>
            </template>
            Refresh
          </n-button>
          <n-button @click="updateAllPackages" :loading="loading" size="medium">
            <template #icon>
              <n-icon><Reload /></n-icon>
            </template>
            Update All
          </n-button>
          <n-button @click="removeAllTransitive" :loading="loading" size="medium">
            <template #icon>
              <n-icon><Trash /></n-icon>
            </template>
            Clean Transitive
          </n-button>
          <n-button @click="goBack" size="medium">
            <template #icon>
              <n-icon><ArrowBack /></n-icon>
            </template>
            Back to Editor
          </n-button>
        </div>
      </div>
    </div>

    <!-- Main Content Area -->
    <div class="main-content">
      <!-- Active Environment Section -->
      <div class="environment-section">
        <h3>Active Environment</h3>
        <n-tag type="info" size="small">{{ currentProjectPath || 'No project selected' }}</n-tag>
      </div>

      <!-- Add Package Section -->
      <div class="search-section">
        <h3>Add Package</h3>
        <div class="search-input-container">
          <n-input
            v-model:value="packageName"
            placeholder="Enter package name (e.g., Plots, DataFrames)"
            size="large"
            @press-enter="addPackage"
            :disabled="isInstalling"
            clearable
          >
            <template #suffix>
              <n-button
                type="primary"
                @click="addPackage"
                :loading="addLoading"
                :disabled="!packageName.trim() || isInstalling"
              >
                Add
              </n-button>
            </template>
          </n-input>
        </div>
      </div>

      <!-- Content Area -->
      <div class="content-area">
        <!-- Installation Progress -->
        <div v-if="installingPackage" class="installation-progress">
          <n-card title="Installing Package" size="small">
            <div class="progress-content">
              <n-spin size="medium" />
              <div class="progress-text">
                <n-text strong>Installing {{ installingPackage }}...</n-text>
                <n-text v-if="installationStatus" type="info" size="small">
                  {{ installationStatus }}
                </n-text>
              </div>
            </div>
          </n-card>
        </div>

        <!-- Installed Packages -->
        <div class="section">
          <div class="section-header">
            <h2>Installed Packages</h2>
            <n-text type="info" depth="3">{{ installedPackages.length }} packages</n-text>
          </div>
          <div class="installed-packages">
            <n-spin :show="loading">
              <n-data-table
                v-if="installedPackages.length > 0"
                :columns="tableColumns"
                :data="sortedPackages"
                :bordered="false"
                size="small"
                :pagination="false"
                :max-height="tableMaxHeight"
                :scroll-x="800"
              />
              <n-empty v-else description="No packages installed" />
            </n-spin>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, h } from 'vue';
import { useRouter } from 'vue-router';
import { useAppStore } from '../store/appStore';
import { useMessage } from 'naive-ui';
import { invoke } from '@tauri-apps/api/core';
import {
  NButton,
  NIcon,
  NTag,
  NSpin,
  NEmpty,
  NDataTable,
  NCard,
  NText,
  NSpace,
  NInput,
  NModal,
} from 'naive-ui';
import {
  Cube,
  Refresh,
  Reload,
  Trash,
  ArrowBack,
  Star,
  Add,
  Checkmark,
  InformationCircle,
} from '@vicons/ionicons5';

const router = useRouter();
const message = useMessage();
const appStore = useAppStore();

// State
const loading = ref(false);
const addLoading = ref(false);
const currentProjectPath = ref('');
const packageName = ref('');
const installedPackages = ref([]);
const installingPackage = ref('');
const installationStatus = ref('');
const tableMaxHeight = ref(400);

// Computed properties
const isInstalling = computed(() => installingPackage.value !== '');
const installedPackageNames = computed(
  () => new Set(installedPackages.value.map((pkg) => pkg.name))
);
// Map package names to their is_direct status
const directDependencies = computed(
  () => new Map(installedPackages.value.map((pkg) => [pkg.name, pkg.is_direct]))
);
const sortedPackages = computed(() => {
  return [...installedPackages.value].sort((a, b) => {
    if (a.is_direct === b.is_direct) {
      return a.name.localeCompare(b.name);
    }
    return a.is_direct ? -1 : 1;
  });
});

// Table columns for installed packages
const tableColumns = [
  {
    title: 'Package Name',
    key: 'name',
    minWidth: 150,
    ellipsis: { tooltip: true },
  },
  {
    title: 'Version',
    key: 'version',
    minWidth: 80,
  },
  {
    title: 'UUID',
    key: 'uuid',
    minWidth: 200,
    ellipsis: { tooltip: true },
  },
  {
    title: 'Type',
    key: 'is_direct',
    minWidth: 80,
    sortable: true,
    sorter: (rowA, rowB) => {
      if (rowA.is_direct === rowB.is_direct) {
        return rowA.name.localeCompare(rowB.name);
      }
      return rowA.is_direct ? -1 : 1;
    },
    render: (row) => {
      const text = row.is_direct ? 'Direct' : 'Transitive';
      const color = row.is_direct ? '#18a058' : '#d03050';
      return h('span', { style: { color, fontWeight: 'bold' } }, text);
    },
  },
  {
    title: 'Actions',
    key: 'actions',
    minWidth: 100,
    render: (row) => {
      return h(
        NButton,
        {
          size: 'small',
          onClick: () => removePackage(row.name),
        },
        {
          default: () => 'Remove',
          icon: () => h(Trash),
        }
      );
    },
  },
];

// Methods
const formatNumber = (num) => {
  if (num >= 1000) {
    return (num / 1000).toFixed(1) + 'k';
  }
  return num.toString();
};

const calculateTableMaxHeight = () => {
  // Get window height and subtract header, add package section, and other sections
  const windowHeight = window.innerHeight;
  const headerHeight = 120; // Approximate header height
  const addPackageHeight = 80; // Approximate add package section height
  const environmentHeight = 40; // Approximate environment section height
  const padding = 100; // Padding and margins
  const availableHeight =
    windowHeight - headerHeight - addPackageHeight - environmentHeight - padding;

  // Set a reasonable max height (at least 200px, at most 600px)
  tableMaxHeight.value = Math.max(200, Math.min(600, availableHeight));
};

const handleResize = () => {
  calculateTableMaxHeight();
};

const goBack = () => {
  router.push({ name: 'Home' });
};

const refreshPackages = async () => {
  try {
    loading.value = true;
    const status = await invoke('get_julia_package_status');
    installedPackages.value = status.packages || [];
  } catch (error) {
    console.error('Failed to refresh package status:', error);
    message.error(`Failed to refresh package status: ${error.toString()}`);
    installedPackages.value = [];
  } finally {
    loading.value = false;
  }
};

const addPackage = async () => {
  if (!packageName.value.trim()) {
    message.warning('Please enter a package name to add');
    return;
  }

  const packageToAdd = packageName.value.trim();

  try {
    addLoading.value = true;
    installingPackage.value = packageToAdd;
    installationStatus.value = 'Starting installation...';

    packageName.value = '';

    let statusInterval = setInterval(() => {
      if (installationStatus.value === 'Starting installation...') {
        installationStatus.value = 'Downloading package and dependencies...';
      } else if (installationStatus.value === 'Downloading package and dependencies...') {
        installationStatus.value = 'Resolving dependencies...';
      } else if (installationStatus.value === 'Resolving dependencies...') {
        installationStatus.value = 'Installing package...';
      } else if (installationStatus.value === 'Installing package...') {
        installationStatus.value = 'Finalizing installation...';
      }
    }, 3000);

    await invoke('run_julia_pkg_command', {
      command: `add ${packageToAdd}`,
    });

    clearInterval(statusInterval);
    installationStatus.value = 'Installation completed successfully!';
    message.success(`Successfully added ${packageToAdd}`);
    await refreshPackages();
  } catch (error) {
    let errorMessage = error.toString();

    try {
      const errorJson = JSON.parse(errorMessage);
      if (errorJson.message) {
        errorMessage = errorJson.message;
      }
    } catch (parseError) {
      console.log('Error is not JSON, using as string:', errorMessage);
    }

    if (errorMessage.includes('not found') || errorMessage.includes('not found in registry')) {
      message.error(
        `Package "${packageToAdd}" not found in the Julia registry. Please check the package name and try again.`
      );
    } else if (errorMessage.includes('already exists') || errorMessage.includes('already added')) {
      message.warning(`Package "${packageToAdd}" is already installed in this environment.`);
    } else {
      message.error(`Failed to add "${packageToAdd}": ${errorMessage}`);
    }
  } finally {
    addLoading.value = false;
    installingPackage.value = '';
    installationStatus.value = '';
    if (statusInterval) {
      clearInterval(statusInterval);
    }
  }
};

const removePackage = async (packageName) => {
  try {
    loading.value = true;
    await invoke('run_julia_pkg_command', {
      command: `rm ${packageName}`,
    });
    message.success(`Successfully removed ${packageName}`);
    await refreshPackages();
  } catch (error) {
    message.error(`Failed to remove ${packageName}: ${error.toString()}`);
  } finally {
    loading.value = false;
  }
};

const updateAllPackages = async () => {
  try {
    loading.value = true;
    await invoke('run_julia_pkg_command', {
      command: 'update',
    });
    message.success('All packages updated successfully');
    await refreshPackages();
  } catch (error) {
    message.error(`Failed to update packages: ${error.toString()}`);
  } finally {
    loading.value = false;
  }
};

const removeAllTransitive = async () => {
  try {
    loading.value = true;
    await invoke('clean_transitive_dependencies');
    message.success('Transitive dependencies cleaned successfully');
    await refreshPackages();
  } catch (error) {
    message.error(`Failed to clean transitive dependencies: ${error.toString()}`);
  } finally {
    loading.value = false;
  }
};

// Initialize
onMounted(async () => {
  // Calculate initial table max height
  calculateTableMaxHeight();

  // Add resize listener
  window.addEventListener('resize', handleResize);

  // Get current project path from app store
  currentProjectPath.value = appStore.projectPath || '';

  if (!currentProjectPath.value) {
    message.warning('No project selected. Please open a Julia project first.');
    return;
  }

  // Load project context
  try {
  } catch (error) {
    console.error('Failed to load project context:', error);
  }

  // Load initial data
  await refreshPackages();
});

// Cleanup
onUnmounted(() => {
  window.removeEventListener('resize', handleResize);
});
</script>

<style scoped>
.package-management-view {
  height: 100vh;
  display: flex;
  flex-direction: column;
  background: var(--n-color);
}

.header-section {
  background: var(--n-card-color);
  border-bottom: 1px solid var(--n-border-color);
  padding: 24px;
  flex-shrink: 0;
  box-sizing: border-box;
}

.header-content {
  display: flex;
  justify-content: space-between;
  align-items: center;
  max-width: 1200px;
  margin: 0 auto;
  width: 100%;
}

.header-left {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.page-title {
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 1.8rem;
  font-weight: 600;
  color: var(--n-text-color);
  margin: 0;
}

.title-icon {
  color: var(--n-primary-color);
  font-size: 1.5rem;
}

.project-info {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.header-actions {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.main-content {
  flex: 1;
  display: flex;
  flex-direction: column;
  max-width: 1200px;
  margin: 0 auto;
  width: 100%;
  min-height: 0;
  padding: 24px;
  box-sizing: border-box;
  overflow: hidden;
}

.environment-section {
  margin-bottom: 24px;
  width: 100%;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 12px;
}

.environment-section h3 {
  margin: 0;
  color: var(--n-text-color);
  font-size: 1.3rem;
  font-weight: 600;
}

.search-section {
  margin-bottom: 24px;
  width: 100%;
  flex-shrink: 0;
}

.search-section h3 {
  margin: 0 0 12px 0;
  color: var(--n-text-color);
  font-size: 1.3rem;
  font-weight: 600;
}

.search-input-container {
  width: 100%;
}

.content-area {
  flex: 1;
  min-height: 0;
  width: 100%;
  box-sizing: border-box;
  display: flex;
  flex-direction: column;
}

.section {
  margin-bottom: 32px;
  display: flex;
  flex-direction: column;
  flex: 1;
  min-height: 0;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  flex-shrink: 0;
}

.section-header h2 {
  margin: 0;
  color: var(--n-text-color);
  font-size: 1.3rem;
  font-weight: 600;
}

.recommendations-grid,
.search-results-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
  gap: 16px;
  flex-shrink: 0;
}

.installation-progress {
  margin-bottom: 24px;
  flex-shrink: 0;
}

.progress-content {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px;
}

.progress-text {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.installed-packages {
  background: var(--n-card-color);
  border: 1px solid var(--n-border-color);
  border-radius: 8px;
  overflow: hidden;
  width: 100%;
  flex: 1;
  min-height: 0;
}

.package-option {
  padding: 8px 12px;
  border-bottom: 1px solid var(--n-border-color);
}

.package-option:last-child {
  border-bottom: none;
}

.package-name {
  font-weight: 600;
  color: var(--n-text-color);
  margin-bottom: 4px;
}

.package-description {
  font-size: 12px;
  color: var(--n-text-color-3);
  line-height: 1.4;
  max-width: 100%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.package-stars {
  display: flex;
  align-items: center;
  gap: 4px;
  font-size: 11px;
  color: var(--n-text-color-3);
  margin-top: 4px;
}

.table-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.search-modal-content {
  padding: 0;
}

.search-modal-header {
  margin-bottom: 16px;
  padding-bottom: 12px;
  border-bottom: 1px solid var(--n-border-color);
}

.search-modal-table {
  width: 100%;
}

.package-expand-content {
  padding: 16px;
  background: var(--n-card-color);
  border-radius: 8px;
  margin: 8px 0;
}

.package-details-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}

.package-detail-section {
  margin-bottom: 16px;
}

.package-detail-section h4 {
  margin: 0 0 8px 0;
  font-size: 0.9rem;
  font-weight: 600;
  color: var(--n-text-color);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.package-detail-section p {
  margin: 0;
  line-height: 1.5;
  color: var(--n-text-color-2);
  font-size: 0.9rem;
}

.keywords-list {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.authors-list {
  margin: 0;
  padding-left: 16px;
  color: var(--n-text-color-2);
  font-size: 0.9rem;
}

.authors-list li {
  margin-bottom: 4px;
}

.installation-info {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* Responsive design for expandable content */
@media (max-width: 768px) {
  .package-details-grid {
    grid-template-columns: 1fr;
    gap: 16px;
  }
}

/* Responsive design */
@media (max-width: 768px) {
  .header-content {
    flex-direction: column;
    align-items: stretch;
    gap: 16px;
  }

  .header-actions {
    justify-content: stretch;
  }

  .header-actions .n-button {
    flex: 1;
  }

  .recommendations-grid,
  .search-results-grid {
    grid-template-columns: 1fr;
  }

  .main-content {
    padding: 16px;
  }
}
</style>
