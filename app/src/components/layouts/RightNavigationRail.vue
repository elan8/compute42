<template>
  <n-space
    vertical
    align="center"
    justify="start"
    style="padding-top: 10px; height: 100%; background-color: #282828"
  >
    <!-- Right navigation rail - reserved for future features -->
  </n-space>
</template>

<script setup>
import { NSpace } from 'naive-ui';
</script>

<style scoped>
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
