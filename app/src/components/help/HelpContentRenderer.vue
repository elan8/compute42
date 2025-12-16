<template>
  <div class="help-content-renderer">
    <!-- Loading state -->
    <div v-if="!content" class="loading-state">
      <n-spin size="large" />
      <p>Loading help content...</p>
    </div>

    <!-- Content rendering -->
    <div v-else class="content-wrapper">
      <!-- About page specific content -->
      <div v-if="sectionId === 'about'" class="about-content">
        <div class="about-header">
          <img src="/icon.png" alt="Compute42" class="about-logo" />
          <h1>About Compute42</h1>
        </div>

        <div class="info-section">
          <h2>Software Information</h2>
          <table class="info-table">
            <tbody>
              <tr>
                <td class="info-label">Version:</td>
                <td class="info-value">{{ appVersion }}</td>
              </tr>
              <tr>
                <td class="info-label">Build Date:</td>
                <td class="info-value">{{ buildDate }}</td>
              </tr>
            </tbody>
          </table>
        </div>

        <div class="info-section">
          <h2>Contact & Links</h2>
          <table class="info-table">
            <tbody>
              <tr>
                <td class="info-label">Website:</td>
                <td class="info-value">
                  <a
                    href="#"
                    @click.prevent="handleLinkClick('https://www.compute42.com')"
                    class="contact-link"
                    >www.compute42.com</a
                  >
                </td>
              </tr>
              <tr>
                <td class="info-label">GitHub:</td>
                <td class="info-value">
                  <a
                    href="#"
                    @click.prevent="handleLinkClick('https://github.com/elan8/compute42')"
                    class="contact-link"
                    >github.com/elan8/compute42</a
                  >
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>

      <!-- Markdown content for other pages -->
      <div v-else-if="content.content && content.content.markdown" class="markdown-content">
        <div v-html="renderedMarkdown"></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { NSpin, NButton, NIcon } from 'naive-ui';
import { CopyOutline, RefreshOutline, OpenOutline } from '@vicons/ionicons5';
import { useMessage } from 'naive-ui';
import { marked } from 'marked';

interface Props {
  content: any;
  sectionId: string;
  appVersion: string;
  buildDate: string;
  checkingUpdates: boolean;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  'copy-system-info': [];
  'check-for-updates': [];
  'open-website': [];
}>();

const message = useMessage();

// Configure marked options
marked.setOptions({
  breaks: true,
  gfm: true,
});

const renderedMarkdown = computed(() => {
  if (!props.content?.content?.markdown) {
    return '';
  }

  let markdownText = props.content.content.markdown;

  // Replace template variables
  markdownText = markdownText
    .replace(/\{\{appVersion\}\}/g, props.appVersion)
    .replace(/\{\{buildDate\}\}/g, props.buildDate);

  // Handle special links for the about page
  if (props.sectionId === 'about') {
    markdownText = markdownText.replace(
      /\[www\.compute42\.com\]\(https:\/\/www\.compute42\.com\)/g,
      '<a href="#" onclick="window.open(\'https://www.compute42.com\', \'_blank\')">www.compute42.com</a>'
    );
  }

  return marked(markdownText);
});

const handleAction = (actionName: string) => {
  switch (actionName) {
    case 'copySystemInfo':
      emit('copy-system-info');
      message.success('System information copied to clipboard');
      break;
    case 'checkForUpdates':
      emit('check-for-updates');
      message.success('Update check completed');
      break;
    case 'openWebsite':
      emit('open-website');
      break;
  }
};

const handleLinkClick = async (url: string) => {
  try {
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(url);
  } catch (error) {
    console.error('Failed to open URL:', error);
  }
};
</script>

<style scoped>
.help-content-renderer {
  height: 100%;
  display: flex;
  flex-direction: column;
  min-height: 0;
  box-sizing: border-box;
}

.loading-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 2rem;
  color: var(--n-text-color-2);
  flex: 1;
}

.loading-state p {
  margin-top: 1rem;
  font-size: 0.9rem;
}

.content-wrapper {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding: 1rem;
  box-sizing: border-box;
}

/* Custom scrollbar styling */
.content-wrapper::-webkit-scrollbar {
  width: 8px;
}

.content-wrapper::-webkit-scrollbar-track {
  background: var(--n-color);
  border-radius: 4px;
}

.content-wrapper::-webkit-scrollbar-thumb {
  background: var(--n-border-color);
  border-radius: 4px;
}

.content-wrapper::-webkit-scrollbar-thumb:hover {
  background: var(--n-text-color-3);
}

/* Firefox scrollbar styling */
.content-wrapper {
  scrollbar-width: thin;
  scrollbar-color: var(--n-border-color) var(--n-color);
}

/* Markdown content styling */
.markdown-content {
  color: var(--n-text-color-2);
  line-height: 1.6;
}

.markdown-content :deep(h1) {
  font-size: 1.8rem;
  margin-bottom: 1.5rem;
  color: #389826;
  font-weight: 600;
}

.markdown-content :deep(h2) {
  font-size: 1.5rem;
  margin: 2rem 0 1rem 0;
  color: #389826;
  border-bottom: 2px solid #389826;
  padding-bottom: 0.5rem;
}

.markdown-content :deep(h3) {
  font-size: 1.2rem;
  margin: 1.5rem 0 0.75rem 0;
  color: var(--n-text-color);
}

.markdown-content :deep(h4) {
  font-size: 1.1rem;
  margin: 1.25rem 0 0.5rem 0;
  color: var(--n-text-color);
}

.markdown-content :deep(p) {
  margin-bottom: 1rem;
  color: var(--n-text-color-2);
}

.markdown-content :deep(ul),
.markdown-content :deep(ol) {
  margin-bottom: 1rem;
  padding-left: 1.5rem;
}

.markdown-content :deep(li) {
  margin-bottom: 0.5rem;
  color: var(--n-text-color-2);
}

.markdown-content :deep(strong) {
  color: var(--n-text-color);
  font-weight: 600;
}

.markdown-content :deep(code) {
  background: var(--n-color);
  border: 1px solid var(--n-border-color);
  border-radius: 4px;
  padding: 2px 6px;
  font-family: monospace;
  font-size: 0.9em;
  color: var(--n-text-color);
}

.markdown-content :deep(pre) {
  background: var(--n-color);
  border: 1px solid var(--n-border-color);
  border-radius: 8px;
  padding: 1rem;
  margin: 1rem 0;
  overflow-x: auto;
}

.markdown-content :deep(pre code) {
  background: none;
  border: none;
  padding: 0;
  font-size: 0.9em;
}

.markdown-content :deep(blockquote) {
  border-left: 4px solid var(--n-primary-color);
  padding-left: 1rem;
  margin: 1rem 0;
  color: var(--n-text-color-2);
  font-style: italic;
}

.markdown-content :deep(a) {
  color: var(--n-primary-color);
  text-decoration: none;
  font-weight: 500;
}

.markdown-content :deep(a:hover) {
  text-decoration: underline;
}

.markdown-content :deep(table) {
  width: 100%;
  border-collapse: collapse;
  margin: 1rem 0;
}

.markdown-content :deep(th),
.markdown-content :deep(td) {
  border: 1px solid var(--n-border-color);
  padding: 0.5rem;
  text-align: left;
}

.markdown-content :deep(th) {
  background: var(--n-color);
  font-weight: 600;
  color: var(--n-text-color);
}

.markdown-content :deep(td) {
  color: var(--n-text-color-2);
}

/* About page content */
.about-content {
  color: var(--n-text-color-2);
  line-height: 1.6;
}

.about-header {
  display: flex;
  align-items: center;
  gap: 1rem;
  margin-bottom: 1rem;
}

.about-logo {
  width: 64px;
  height: 64px;
  flex-shrink: 0;
}

.about-content h1 {
  font-size: 1.8rem;
  margin: 0;
  color: #389826;
  font-weight: 600;
}

.about-content h2 {
  font-size: 1.5rem;
  margin: 1.25rem 0 0.5rem 0;
  color: var(--n-text-color);
  border-bottom: 2px solid #389826;
  padding-bottom: 0.5rem;
}

.info-section {
  margin-bottom: 1.25rem;
}

.info-table {
  width: 100%;
  border-collapse: collapse;
  margin: 0.5rem 0;
}

.info-table td {
  padding: 0.5rem 0.75rem;
  border-bottom: 1px solid var(--n-border-color);
}

.info-label {
  font-weight: 600;
  color: var(--n-text-color);
  width: 40%;
}

.info-value {
  color: var(--n-text-color-2);
}

.contact-link {
  color: #389826;
  text-decoration: none;
  font-weight: 500;
  transition: color 0.2s ease;
}

.contact-link:hover {
  color: #4aa830;
  text-decoration: underline;
}

.actions-section {
  text-align: center;
  display: flex;
  gap: 1rem;
  justify-content: center;
  flex-wrap: wrap;
}

.action-button {
  min-width: 120px;
}

/* Responsive Design */
@media (max-width: 768px) {
  .actions-section {
    flex-direction: column;
    align-items: center;
  }

  .action-button {
    width: 100%;
    max-width: 180px;
  }
}
</style>
