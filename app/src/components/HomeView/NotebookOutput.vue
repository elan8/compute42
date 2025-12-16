<template>
  <div class="notebook-output">
    <!-- Execute Result Output -->
    <div v-if="isExecuteResult(output)" class="output-execute-result">
      <div v-if="output.data['text/plain']" class="output-text">
        <!-- <pre>{{ output.data['text/plain'] }}</pre> -->
      </div>
      <div
        v-else-if="output.data['text/html']"
        class="output-html"
        v-html="sanitizeHtml(output.data['text/html'])"
      ></div>
      <div
        v-else-if="output.data['image/svg+xml']"
        class="output-image"
        v-html="formatSvgSrc(output.data['image/svg+xml'])"
      />
      <div v-else-if="output.data['image/png']" class="output-image">
        <img
          :src="formatImageSrc(output.data['image/png'], 'png')"
          alt="Output image"
          @error="handleImageError"
        />
      </div>
      <div v-else-if="output.data['image/jpeg']" class="output-image">
        <img
          :src="formatImageSrc(output.data['image/jpeg'], 'jpeg')"
          alt="Output image"
          @error="handleImageError"
        />
      </div>
    </div>

    <!-- Display Data Output -->
    <div v-else-if="isDisplayData(output)" class="output-display-data">
      <div v-if="output.data['text/plain']" class="output-text">
        <pre>{{ output.data['text/plain'] }}</pre>
      </div>
      <div
        v-else-if="output.data['text/html']"
        class="output-html"
        v-html="sanitizeHtml(output.data['text/html'])"
      ></div>
      <div
        v-else-if="output.data['image/svg+xml']"
        class="output-image"
        v-html="formatSvgSrc(output.data['image/svg+xml'])"
      />
      <div v-else-if="output.data['image/png']" class="output-image">
        <img
          :src="formatImageSrc(output.data['image/png'], 'png')"
          alt="Display image"
          @error="handleImageError"
        />
      </div>
      <div v-else-if="output.data['image/jpeg']" class="output-image">
        <img
          :src="formatImageSrc(output.data['image/jpeg'], 'jpeg')"
          alt="Display image"
          @error="handleImageError"
        />
      </div>
    </div>

    <!-- Stream Output -->
    <div v-else-if="isStream(output)" class="output-stream" :class="`stream-${output.name}`">
      <pre>{{ output.text }}</pre>
    </div>

    <!-- Error Output -->
    <div v-else-if="isError(output)" class="output-error">
      <div class="error-header">
        <n-icon color="#f44336" size="18">
          <CloseCircleOutline />
        </n-icon>
        <span class="error-name">{{ output.ename }}:</span>
        <span class="error-value">{{ output.evalue }}</span>
      </div>
      <div v-if="output.traceback && output.traceback.length > 0" class="error-traceback">
        <pre v-for="(line, index) in output.traceback" :key="index">{{ line }}</pre>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { NIcon } from 'naive-ui';
import { CloseCircleOutline } from '@vicons/ionicons5';
import type {
  CellOutput,
  ExecuteResultOutput,
  DisplayDataOutput,
  StreamOutput,
  ErrorOutput,
  isExecuteResult,
  isDisplayData,
  isStream,
  isError,
} from '../../types/notebook';

const props = defineProps<{
  output: CellOutput;
}>();

// Type guards
function isExecuteResult(output: CellOutput): output is ExecuteResultOutput {
  return output.output_type === 'execute_result';
}

function isDisplayData(output: CellOutput): output is DisplayDataOutput {
  return output.output_type === 'display_data';
}

function isStream(output: CellOutput): output is StreamOutput {
  return output.output_type === 'stream';
}

function isError(output: CellOutput): output is ErrorOutput {
  return output.output_type === 'error';
}

// Format image source - handle both base64 strings and URLs
function formatImageSrc(data: string | string[], type: 'png' | 'jpeg'): string {
  // Handle both string and array formats (Jupyter notebooks can use either)
  let dataString: string;
  if (Array.isArray(data)) {
    dataString = data.join('');
  } else if (typeof data === 'string') {
    dataString = data;
  } else {
    // Fallback for unexpected types
    console.warn('formatImageSrc: Unexpected data type', typeof data, data);
    dataString = String(data);
  }

  // If data is already a data URL or HTTP URL, return as-is
  if (
    dataString.startsWith('data:') ||
    dataString.startsWith('http://') ||
    dataString.startsWith('https://')
  ) {
    return dataString;
  }
  // Otherwise, treat as base64 and add data URL prefix
  return `data:image/${type};base64,${dataString}`;
}

// Format SVG source - handle both SVG strings and URLs
function formatSvgSrc(data: string | string[]): string {
  // Handle both string and array formats (Jupyter notebooks can use either)
  let dataString: string;
  if (Array.isArray(data)) {
    dataString = data.join('');
  } else if (typeof data === 'string') {
    dataString = data;
  } else {
    // Fallback for unexpected types
    console.warn('formatSvgSrc: Unexpected data type', typeof data, data);
    dataString = String(data);
  }

  // Clean the data first - remove any HTML markup
  let cleanedData = dataString.trim();

  // If data contains HTML img tag, extract the src attribute
  if (cleanedData.includes('<img')) {
    const srcMatch = cleanedData.match(/src=["']([^"']+)["']/);
    if (srcMatch && srcMatch[1]) {
      cleanedData = srcMatch[1];
    } else {
      // If no src found, try to extract SVG from HTML
      const svgMatch = cleanedData.match(/<svg[\s\S]*?<\/svg>/);
      if (svgMatch && svgMatch[0]) {
        cleanedData = svgMatch[0];
      }
    }
  }

  // If data is already a data URL or HTTP URL, create an img tag
  if (
    cleanedData.startsWith('data:') ||
    cleanedData.startsWith('http://') ||
    cleanedData.startsWith('https://')
  ) {
    return `<img src="${cleanedData}" alt="SVG plot" style="max-width: 100%; height: auto;" />`;
  }
  // If it's an SVG string, return as-is
  if (cleanedData.startsWith('<svg')) {
    return cleanedData;
  }
  // Otherwise, treat as base64 and create data URL
  const svgDataUrl = `data:image/svg+xml;base64,${cleanedData}`;
  return `<img src="${svgDataUrl}" alt="SVG plot" style="max-width: 100%; height: auto;" />`;
}

// Handle image loading errors
function handleImageError(event: Event) {
  const img = event.target as HTMLImageElement;
  console.error('Failed to load image:', img.src);
  // Show error message in place of image
  if (img.parentElement) {
    img.parentElement.innerHTML =
      '<div style="padding: 8px; background: rgba(244,67,54,0.1); border: 1px solid #f44336; border-radius: 4px; color: #ff6b6b;">Failed to load image</div>';
  }
}

// Sanitize HTML to prevent XSS
function sanitizeHtml(html: string): string {
  // Basic sanitization - in production, use a proper sanitization library
  const div = document.createElement('div');
  div.textContent = html;
  return div.innerHTML;
}
</script>

<style scoped>
.notebook-output {
  padding: 8px 12px;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.5;
}

.output-execute-result,
.output-display-data {
  margin: 4px 0;
}

.output-text pre {
  margin: 0;
  white-space: pre-wrap;
  word-wrap: break-word;
  color: #d4d4d4;
}

.output-html {
  margin: 4px 0;
  color: #d4d4d4;
}

.output-image {
  margin: 8px 0;
}

.output-image img {
  max-width: 100%;
  height: auto;
  border: 1px solid #444;
  border-radius: 4px;
}

.output-stream {
  margin: 4px 0;
}

.output-stream pre {
  margin: 0;
  white-space: pre-wrap;
  word-wrap: break-word;
}

.stream-stdout {
  color: #d4d4d4;
}

.stream-stderr {
  color: #f44336;
}

.output-error {
  margin: 8px 0;
  padding: 8px;
  background-color: rgba(244, 67, 54, 0.1);
  border-left: 3px solid #f44336;
  border-radius: 4px;
}

.error-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
  font-weight: 500;
}

.error-name {
  color: #f44336;
  font-weight: 600;
}

.error-value {
  color: #ff6b6b;
}

.error-traceback {
  margin-top: 8px;
  padding-left: 8px;
  border-left: 2px solid #f44336;
}

.error-traceback pre {
  margin: 2px 0;
  color: #ff6b6b;
  font-size: 12px;
}
</style>
