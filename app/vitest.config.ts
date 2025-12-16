import { defineConfig } from 'vitest/config';
import vue from '@vitejs/plugin-vue'; // Import the Vue plugin

// If vite.config.js exports a default function or object, you might need to adjust how it's imported.
// For simplicity, we'll define the Vitest config separately, ensuring Vue plugin is included.

export default defineConfig({
  plugins: [
    vue(), // Add Vue plugin here for Vitest to process .vue files
  ],
  test: {
    globals: true, // Makes describe, it, expect, etc. globally available
    environment: 'happy-dom', // or 'jsdom'
    setupFiles: ['./src/test/setup.ts'], // Test setup file
    coverage: {
      provider: 'v8', // or 'istanbul'
      reporter: ['text', 'json', 'html'],
      include: ['src/**/*.{js,ts,vue}'], // Specify files to include in coverage
      exclude: [
        // Specify files/patterns to exclude from coverage
        'src/main.ts',
        'src/router/index.ts',
        'src/store/index.ts', // if you have an index.ts for Pinia setup
        'src/plugins/**/*',
        'src/theme/**/*',
        'src/types/**/*',
        'src/shims-vue.d.ts',
        'src/**/App.vue', // Often App.vue is mostly setup
        'src/vite-env.d.ts',
        'src-tauri/**/*',
        '**/*.bak',
      ],
    },
    // To ensure that Vue components are processed correctly
    deps: {
      inline: ['@vue', '@vueuse', 'vue-router', 'naive-ui', 'pinia'], // Add other UI/state libs if needed
    },
    alias: {
      '@/': new URL('./src/', import.meta.url).pathname,
    },
  },
});
