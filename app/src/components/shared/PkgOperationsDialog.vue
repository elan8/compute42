<template>
  <n-modal
    v-model:show="show"
    preset="card"
    style="width: 90vw; height: 90vh; max-width: 1200px; max-height: 90vh"
    title="Package Management"
    :mask-closable="false"
    :closable="!isInstalling"
  >
    <div class="package-manager-container">
      <!-- Header with project information -->
      <div class="header-section">
        <div class="project-info">
          <div class="project-title">
            <n-text strong>Active Project:</n-text>
            <n-text type="info" size="small">{{
              currentProjectPath || 'No project path available'
            }}</n-text>
          </div>
        </div>
        <div class="header-actions">
          <n-button
            @click="refreshPackages"
            :loading="loading"
            size="small"
            :disabled="isInstalling"
          >
            <template #icon>
              <n-icon><Refresh /></n-icon>
            </template>
            Refresh
          </n-button>
          <n-button
            @click="updateAllPackages"
            :loading="loading"
            size="small"
            :disabled="isInstalling"
          >
            <template #icon>
              <n-icon><Reload /></n-icon>
            </template>
            Update All
          </n-button>
          <n-button
            @click="removeAllTransitive"
            :loading="loading"
            size="small"
            :disabled="isInstalling"
          >
            <template #icon>
              <n-icon><Trash /></n-icon>
            </template>
            Remove All Transitive
          </n-button>
        </div>
      </div>

      <!-- Project Context Header -->
      <div v-if="projectContext" class="project-context-section">
        <div class="context-header">
          <h4>{{ projectContext.name || 'Current Project' }}</h4>
          <div class="context-tags">
            <n-tag
              v-for="category in projectContext.categories"
              :key="category"
              type="info"
              size="small"
            >
              {{ category }}
            </n-tag>
            <n-tag type="success" size="small">{{
              projectContext.domain.replace('_', ' ').toUpperCase()
            }}</n-tag>
          </div>
        </div>
        <p v-if="projectContext.description" class="context-description">
          {{ projectContext.description }}
        </p>
      </div>

      <!-- Personalized Recommendations -->
      <div v-if="personalizedRecommendations.length > 0" class="recommendations-section">
        <h4>🎯 Recommended for Your Project</h4>
        <div class="recommendations-grid">
          <PackageCard
            v-for="rec in personalizedRecommendations.slice(0, 6)"
            :key="rec.package.name"
            :package-data="{
              package: rec.package,
              relevance_score: rec.relevance_score,
              reason: rec.reason,
              category: rec.category,
              is_installed: installedPackageNames.has(rec.package.name),
              is_direct: directDependencies.get(rec.package.name) === true,
            }"
            @add-package="handleAddPackage"
            @show-details="handleShowPackageDetails"
          />
        </div>
      </div>

      <!-- Trending Packages -->
      <div class="trending-section">
        <TrendingPackages
          :project-path="currentProjectPath"
          :limit="8"
          :domain-specific="true"
          title="🔥 Trending in Your Domain"
          @add-package="handleAddPackage"
          @package-click="handleShowPackageDetails"
        />
      </div>

      <!-- Enhanced Search Section -->
      <div class="add-section">
        <div class="add-header">
          <n-text strong>Search Packages</n-text>
          <n-tooltip trigger="hover">
            <template #trigger>
              <n-icon size="16" style="cursor: help; color: var(--n-text-color-3)">
                <InformationCircle />
              </n-icon>
            </template>
            Search from 10,000+ Julia packages with intelligent suggestions and rich metadata.
          </n-tooltip>
        </div>
        <n-auto-complete
          v-model:value="packageName"
          :options="packageSuggestions"
          placeholder="Search packages by name, description, or topic..."
          style="width: 100%"
          @keyup.enter="addPackage"
          @select="onPackageSelect"
          @input="onPackageInput"
          :disabled="isInstalling"
          :loading="searchLoading"
          clearable
        >
          <template #option="{ option }">
            <div class="package-option">
              <div class="package-name">{{ option.label }}</div>
              <div class="package-description">{{ option.description }}</div>
              <div v-if="option.stars" class="package-stars">
                <n-icon><Star /></n-icon>
                {{ formatNumber(option.stars) }}
              </div>
            </div>
          </template>
          <template #suffix>
            <n-button
              size="small"
              type="primary"
              @click="addPackage"
              :loading="addLoading"
              :disabled="!packageName.trim() || isInstalling"
            >
              Add
            </n-button>
          </template>
        </n-auto-complete>
      </div>

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

      <!-- Installed Packages Table (hidden during installation) -->
      <div v-if="!isInstalling" class="packages-section">
        <n-text strong>Installed Packages:</n-text>
        <n-spin :show="loading">
          <div class="table-container">
            <n-data-table
              v-if="installedPackages.length > 0"
              :columns="tableColumns"
              :data="sortedPackages"
              :bordered="false"
              size="small"
              :pagination="false"
            />
            <n-empty v-else description="No packages installed" />
          </div>
        </n-spin>
      </div>
    </div>

    <template #footer>
      <!-- Footer content removed - using modal header X button instead -->
    </template>
  </n-modal>
</template>

<script setup>
import { ref, computed, onMounted, onUnmounted, h } from 'vue';
import { useMessage } from 'naive-ui';
import { invoke } from '@tauri-apps/api/core';
import {
  NModal,
  NInput,
  NAutoComplete,
  NButton,
  NSpace,
  NText,
  NSpin,
  NEmpty,
  NDataTable,
  NIcon,
  NTooltip,
  NCard,
  NTag,
} from 'naive-ui';
import { Refresh, Trash, Reload, InformationCircle, Star } from '@vicons/ionicons5';
import { packageSearchService } from '../../services/packageSearchService';
import PackageCard from './PackageCard.vue';
import TrendingPackages from './TrendingPackages.vue';

const message = useMessage();
const show = ref(false);
const loading = ref(false);
const addLoading = ref(false);
const currentProjectPath = ref('');
const packageName = ref('');
const installedPackages = ref([]);
const installingPackage = ref('');
const installationStatus = ref('');

// SearchPackages integration state
const projectContext = ref(null);
const personalizedRecommendations = ref([]);
const packageSuggestions = ref([]);
const searchLoading = ref(false);
const searchTimeout = ref(null);

const emit = defineEmits(['packagesChanged']);

// Computed property to check if installation is in progress
const isInstalling = computed(() => installingPackage.value !== '');

// Computed property for installed package names
const installedPackageNames = computed(
  () => new Set(installedPackages.value.map((pkg) => pkg.name))
);
// Map package names to their is_direct status
const directDependencies = computed(
  () => new Map(installedPackages.value.map((pkg) => [pkg.name, pkg.is_direct]))
);

// Computed property to sort packages with direct dependencies first
const sortedPackages = computed(() => {
  return [...installedPackages.value].sort((a, b) => {
    if (a.is_direct === b.is_direct) {
      return a.name.localeCompare(b.name);
    }
    return a.is_direct ? -1 : 1;
  });
});

// Helper function to get project file name
const getProjectFileName = (path) => {
  if (!path) return 'No project';
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || 'Project';
};

// Table columns for installed packages
const tableColumns = [
  {
    title: 'Package Name',
    key: 'name',
    minWidth: 150,
    ellipsis: {
      tooltip: true,
    },
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
    ellipsis: {
      tooltip: true,
    },
  },
  {
    title: 'Direct?',
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
    minWidth: 50,
    render: (row) => {
      return h(
        NButton,
        {
          size: 'small',
          onClick: () => removePackage(row.name),
        },
        {
          icon: () => h(Trash),
        }
      );
    },
  },
];

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

    // Disable the input and other controls during installation
    const originalPackageName = packageName.value;
    packageName.value = '';

    // Set up periodic status updates for long-running installations
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
    }, 3000); // Update status every 3 seconds

    await invoke('run_julia_pkg_command', {
      command: `add ${packageToAdd}`,
    });

    clearInterval(statusInterval);
    installationStatus.value = 'Installation completed successfully!';
    message.success(`Successfully added ${packageToAdd}`);
    await refreshPackages();
    emit('packagesChanged');
  } catch (error) {
    let errorMessage = error.toString();

    // Try to parse the error as JSON first (backend returns JSON with success/message fields)
    try {
      const errorJson = JSON.parse(errorMessage);
      if (errorJson.message) {
        errorMessage = errorJson.message;
      }
    } catch (parseError) {
      // If parsing fails, use the original error message
      console.log('Error is not JSON, using as string:', errorMessage);
    }

    // Parse common Julia package addition errors and provide user-friendly messages
    if (
      errorMessage.includes('not found') ||
      errorMessage.includes('not found in registry') ||
      errorMessage.includes('not found in project, manifest or registry')
    ) {
      message.error(
        `Package "${packageToAdd}" not found in the Julia registry. Please check the package name and try again.`
      );
    } else if (
      errorMessage.includes('already exists') ||
      errorMessage.includes('already added') ||
      errorMessage.includes('already installed')
    ) {
      message.warning(`Package "${packageToAdd}" is already installed in this environment.`);
    } else if (
      errorMessage.includes('network') ||
      errorMessage.includes('connection') ||
      errorMessage.includes('timeout') ||
      errorMessage.includes('HTTP')
    ) {
      message.error(
        `Network error while adding "${packageToAdd}". Please check your internet connection and try again.`
      );
    } else if (
      errorMessage.includes('invalid') ||
      errorMessage.includes('malformed') ||
      errorMessage.includes('ParseError')
    ) {
      message.error(
        `Invalid package name "${packageToAdd}". Package names must be valid Julia identifiers.`
      );
    } else if (
      errorMessage.includes('registry') ||
      errorMessage.includes('registry not found') ||
      errorMessage.includes('Registry')
    ) {
      message.error(`Registry error while adding "${packageToAdd}". Please try again later.`);
    } else if (errorMessage.includes('resolve') || errorMessage.includes('dependency')) {
      message.error(
        `Dependency resolution failed for "${packageToAdd}". This package may have conflicting dependencies.`
      );
    } else {
      // Show the detailed error message from the captured stderr output
      message.error(`Failed to add "${packageToAdd}": ${errorMessage}`);
    }
  } finally {
    addLoading.value = false;
    installingPackage.value = '';
    installationStatus.value = '';
    // Clear any remaining intervals
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
    emit('packagesChanged');
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
    emit('packagesChanged');
  } catch (error) {
    message.error(`Failed to update packages: ${error.toString()}`);
  } finally {
    loading.value = false;
  }
};

const removeAllTransitive = async () => {
  try {
    loading.value = true;

    // First, get all packages and identify transitive ones
    const status = await invoke('get_julia_package_status');

    const transitivePackages = status.packages
      .filter((pkg) => !pkg.is_direct)
      .map((pkg) => pkg.name);

    if (transitivePackages.length === 0) {
      message.info('No transitive dependencies to remove');
      return;
    }

    // Clean transitive dependencies by removing Manifest.toml and re-instantiating
    await invoke('clean_transitive_dependencies');

    message.success(`Cleaned ${transitivePackages.length} transitive dependencies`);
    await refreshPackages();
    emit('packagesChanged');
  } catch (error) {
    message.error(`Failed to clean transitive dependencies: ${error.toString()}`);
  } finally {
    loading.value = false;
  }
};

const refreshPackages = async () => {
  try {
    loading.value = true;

    console.log('Refreshing packages for current project');
    const status = await invoke('get_julia_package_status');

    console.log('Package status result:', status);
    console.log('Status type:', typeof status);
    console.log('Status keys:', Object.keys(status));
    console.log('Status.packages type:', typeof status.packages);
    console.log('Status.packages:', status.packages);

    installedPackages.value = status.packages || [];

    // Debug: Log the packages to see their structure
    console.log('Installed packages:', installedPackages.value);
    console.log('Installed packages length:', installedPackages.value.length);
    if (installedPackages.value.length > 0) {
      console.log('First package:', installedPackages.value[0]);
      console.log(
        'Direct packages count:',
        installedPackages.value.filter((p) => p.is_direct).length
      );
      console.log(
        'Transitive packages count:',
        installedPackages.value.filter((p) => !p.is_direct).length
      );
    }

    if (installedPackages.value.length === 0) {
      message.info('No packages found in the current environment');
    } else {
      message.success(`Found ${installedPackages.value.length} package(s)`);
    }
  } catch (error) {
    console.error('Failed to refresh package status:', error);
    message.error(`Failed to refresh package status: ${error.toString()}`);
    installedPackages.value = [];
  } finally {
    loading.value = false;
  }
};

// Enhanced search using SearchPackages service
const searchPackages = async (query) => {
  if (!query || query.length < 2) {
    packageSuggestions.value = [];
    return;
  }

  try {
    searchLoading.value = true;
    console.log('[PkgOperationsDialog] Searching packages:', query);

    // Use SearchPackages service for suggestions
    const suggestions = await packageSearchService.getSuggestions(query, 10);

    if (suggestions.length > 0) {
      // Get additional package details for suggestions
      const searchResults = await packageSearchService.searchPackages({
        query,
        limit: 10,
      });

      // Create rich suggestions with metadata
      packageSuggestions.value = searchResults.packages.map((pkg) => ({
        label: pkg.name,
        value: pkg.name,
        description: pkg.description || 'No description available',
        stars: pkg.stars,
        topics: pkg.topics,
      }));
    } else {
      // Show a "no results" option
      packageSuggestions.value = [
        {
          label: `No packages found for "${query}"`,
          value: '',
          description: 'Try a different search term',
          disabled: true,
        },
      ];
    }
  } catch (error) {
    console.error('[PkgOperationsDialog] Failed to search packages:', error);
    packageSuggestions.value = [
      {
        label: 'Search failed',
        value: '',
        description: 'Unable to search packages. Please try again.',
        disabled: true,
      },
    ];
  } finally {
    searchLoading.value = false;
  }
};

const onPackageInput = (value) => {
  // Clear existing timeout
  if (searchTimeout.value) {
    clearTimeout(searchTimeout.value);
  }

  // Set new timeout for debounced search
  searchTimeout.value = setTimeout(() => {
    searchPackages(value);
  }, 300); // 300ms delay
};

const onPackageSelect = (value) => {
  // Don't select disabled options (like "no results" messages)
  if (value && value.trim() !== '') {
    packageName.value = value;
  }
  // Clear suggestions after selection
  packageSuggestions.value = [];
};

// Load project context and personalized recommendations
const loadProjectContext = async () => {
  if (!currentProjectPath.value) return;

  try {
    console.log('[PkgOperationsDialog] Loading project context...');
    projectContext.value = await packageSearchService.analyzeProjectContext(
      currentProjectPath.value
    );

    // Load personalized recommendations
    const recommendations = await packageSearchService.getPersonalizedRecommendations({
      project_path: currentProjectPath.value,
      limit: 12,
    });
    personalizedRecommendations.value = recommendations;

    console.log(
      '[PkgOperationsDialog] Loaded',
      recommendations.length,
      'personalized recommendations'
    );
  } catch (error) {
    console.error('[PkgOperationsDialog] Failed to load project context:', error);
    // Continue without context - not critical for basic functionality
  }
};

// Handle package details modal
const handleShowPackageDetails = (packageData) => {
  // TODO: Implement package details modal
  console.log('[PkgOperationsDialog] Show package details:', packageData);
  message.info(`Package details for ${packageData.package.name} - Coming soon!`);
};

// Handle add package from components
const handleAddPackage = async (packageName) => {
  if (!packageName.trim()) return;

  const originalPackageName = packageName.value;
  packageName.value = packageName;

  try {
    await addPackage();
  } finally {
    packageName.value = originalPackageName;
  }
};

// Format number helper
const formatNumber = (num) => {
  if (num >= 1000) {
    return (num / 1000).toFixed(1) + 'k';
  }
  return num.toString();
};

const open = async (projectPath) => {
  show.value = true;
  currentProjectPath.value = projectPath;

  // Load project context and recommendations
  await loadProjectContext();

  // Load installed packages
  await refreshPackages();
};

// Cleanup on unmount
onUnmounted(() => {
  if (searchTimeout.value) {
    clearTimeout(searchTimeout.value);
  }
});

defineExpose({
  show,
  open,
});
</script>

<style scoped>
.package-manager-container {
  height: 100%;
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-height: 0;
}

.header-section {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--n-border-color);
  gap: 16px;
  flex-shrink: 0;
}

.project-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
}

.project-title {
  display: flex;
  align-items: center;
  gap: 8px;
}

.project-path {
  margin-top: 2px;
}

.header-actions {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.add-section {
  padding: 16px 0;
  flex-shrink: 0;
}

.add-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.installation-progress {
  margin: 16px 0;
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

.packages-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  overflow: hidden;
}

.packages-section .n-data-table {
  flex: 1;
}

.table-container {
  overflow-y: auto;
  overflow-x: auto;
  border: 1px solid var(--n-border-color);
  border-radius: 6px;
  min-width: 0;
  max-height: calc(90vh - 350px);
  min-height: 200px;
  flex: 1;
}

/* Responsive design */
@media (max-width: 768px) {
  .header-section {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }

  .header-actions {
    flex-direction: column;
    width: 100%;
  }

  .header-actions .n-button {
    width: 100%;
  }

  .progress-content {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }
}

/* Project context styling */
.project-context-section {
  background: var(--n-color);
  border: 1px solid var(--n-border-color);
  border-radius: 8px;
  padding: 16px;
  margin-bottom: 16px;
}

.context-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 8px;
  gap: 12px;
}

.context-header h4 {
  margin: 0;
  color: var(--n-text-color);
  font-size: 1.1rem;
}

.context-tags {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.context-description {
  color: var(--n-text-color-2);
  font-size: 0.9rem;
  line-height: 1.4;
  margin: 0;
}

/* Recommendations styling */
.recommendations-section {
  margin-bottom: 20px;
}

.recommendations-section h4 {
  margin: 0 0 12px 0;
  color: var(--n-text-color);
  font-size: 1.1rem;
}

.recommendations-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 12px;
}

/* Trending section styling */
.trending-section {
  margin-bottom: 20px;
}

/* Package option styling */
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

@media (max-width: 480px) {
  .project-title {
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
  }

  .add-header {
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
  }
}
</style>
