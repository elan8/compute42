<template>
  <div v-if="output" class="command-output-container">
    <n-text v-if="showTitle" strong class="output-title">{{ title }}</n-text>
    <div class="scrollable-log-area" :style="{ maxHeight: maxHeight }" ref="scrollContainerRef">
      <pre class="log-content">{{ output }}</pre>
    </div>
  </div>
</template>

<script setup lang="ts">
import { NText } from 'naive-ui';
import { ref, watch, nextTick } from 'vue';

const props = defineProps({
  title: { type: String, default: 'Output' },
  output: { type: String, required: true },
  maxHeight: { type: String, default: '300px' },
  showTitle: { type: Boolean, default: true },
  autoScroll: { type: Boolean, default: false },
});

const scrollContainerRef = ref<HTMLElement | null>(null);

const scrollToBottom = () => {
  if (scrollContainerRef.value) {
    scrollContainerRef.value.scrollTop = scrollContainerRef.value.scrollHeight;
  }
};

watch(
  () => props.output,
  () => {
    if (props.autoScroll) {
      nextTick(() => {
        scrollToBottom();
      });
    }
  },
  { flush: 'post' }
);

watch(
  () => props.autoScroll,
  (newAutoScroll) => {
    if (newAutoScroll && props.output) {
      nextTick(() => {
        scrollToBottom();
      });
    }
  }
);
</script>

<style scoped>
.command-output-container {
  margin-bottom: 15px;
  background-color: #282c34;
  padding: 10px;
  border-radius: 4px;
  display: flex;
  flex-direction: column;
}

.output-title {
  color: #abb2bf;
  margin-bottom: 5px;
  flex-shrink: 0;
}

.scrollable-log-area {
  overflow-y: auto;
  flex-grow: 1;
  min-height: 0;
}

.log-content {
  color: #abb2bf;
  font-family:
    v-mono, SFMono-Regular, Menlo, Monaco, Consolas, 'Liberation Mono', 'Courier New', monospace;
  font-size: 1em;
  white-space: pre-wrap;
  word-wrap: break-word;
  margin: 0;
  padding: 5px;
}
</style>
