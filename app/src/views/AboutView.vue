<template>
  <div class="help-about-container">
    <n-tabs v-model:value="activeTab" type="line" animated class="help-tabs">
      <n-tab-pane
        v-for="section in helpSections"
        :key="section.id"
        :name="section.id"
        :tab="section.title"
      >
        <div class="help-content">
          <HelpContentRenderer
            :content="section.content"
            :section-id="section.id"
            :app-version="appVersion"
            :build-date="buildDate"
            :checking-updates="checkingUpdates"
            @copy-system-info="copySystemInfo"
            @check-for-updates="checkForUpdates"
            @open-website="openWebsite"
          />
        </div>
      </n-tab-pane>
    </n-tabs>
  </div>
</template>

<script setup>
import { ref, onMounted, computed, watch } from 'vue';
import { NCard, NTabs, NTabPane, NSpin } from 'naive-ui';
import { openUrl } from '@tauri-apps/plugin-opener';
import { invoke } from '@tauri-apps/api/core';
import { helpService } from '../services/helpService';
import HelpContentRenderer from '../components/help/HelpContentRenderer.vue';

const activeTab = ref('about');
const buildDate = ref(new Date().toLocaleDateString());
const appVersion = ref('0.4.0');
const juliaVersion = ref('Unknown');
const nodeVersion = ref('Unknown');
const platform = ref('Unknown');
const architecture = ref('Unknown');
const checkingUpdates = ref(false);

const helpSections = computed(() => helpService.getHelpSections());

const openWebsite = async () => {
  await openUrl('https://www.compute42.com');
};

const copySystemInfo = async () => {
  const systemInfo = `Compute42 v${appVersion.value}
Build Date: ${buildDate.value}
Julia Version: ${juliaVersion.value}
Platform: ${platform.value}
Architecture: ${architecture.value}`;

  try {
    await navigator.clipboard.writeText(systemInfo);
    // Message will be handled by the HelpContentRenderer component
  } catch (error) {
    console.error('Failed to copy system information:', error);
  }
};

const getSystemInfo = async () => {
  try {
    // Get system information from Tauri
    const info = await invoke('get_system_info');
    juliaVersion.value = info.julia_version || 'Unknown';
    nodeVersion.value = info.node_version || 'Unknown';
    platform.value = info.platform || 'Unknown';
    architecture.value = info.architecture || 'Unknown';
  } catch (error) {
    console.error('Failed to get system info:', error);
  }
};

const getAppVersion = async () => {
  try {
    const version = await invoke('get_app_version');
    appVersion.value = version;
  } catch (error) {
    console.error('Failed to get app version:', error);
  }
};

const checkForUpdates = async () => {
  checkingUpdates.value = true;
  try {
    await invoke('check_for_updates');
    // Message will be handled by the HelpContentRenderer component
  } catch (error) {
    console.error('Update check failed:', error);
  } finally {
    checkingUpdates.value = false;
  }
};

// Watch for tab changes and load content
watch(
  activeTab,
  async (newTab) => {
    if (newTab) {
      await helpService.loadHelpContent(newTab);
    }
  },
  { immediate: true }
);

onMounted(async () => {
  getSystemInfo();
  getAppVersion();

  // Load help content for the active tab
  await helpService.loadHelpContent(activeTab.value);
});
</script>

<style scoped>
.help-about-container {
  height: 100vh;
  max-height: 100vh;
  display: flex;
  flex-direction: column;
  padding: 1rem;
  background: var(--n-color);
  overflow: hidden;
  box-sizing: border-box;
}

.help-tabs {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  box-sizing: border-box;
}

.help-tabs :deep(.n-tabs-nav) {
  margin-bottom: 1rem;
  flex-shrink: 0;
}

.help-tabs :deep(.n-tabs-content) {
  flex: 1;
  min-height: 0;
  box-sizing: border-box;
}

.help-tabs :deep(.n-tab-pane) {
  height: 100%;
  min-height: 0;
  box-sizing: border-box;
}

/* Help Content */
.help-content {
  height: 100%;
  min-height: 0;
  padding: 0;
  box-sizing: border-box;
}

/* Responsive Design */
@media (max-width: 768px) {
  .help-about-container {
    padding: 0.5rem;
  }
}

@media (max-width: 480px) {
  .help-about-container {
    padding: 0.25rem;
  }
}
</style>
