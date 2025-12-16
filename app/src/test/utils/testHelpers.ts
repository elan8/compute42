import { mount, type MountingOptions } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { createRouter, createWebHistory } from 'vue-router';
import type { Component } from 'vue';

/**
 * Test helper utilities for Vue components and composables
 */

/**
 * Create a Pinia store instance for testing
 */
export function createTestPinia() {
  const pinia = createPinia();
  setActivePinia(pinia);
  return pinia;
}

/**
 * Create a Vue Router instance for testing
 */
export function createTestRouter() {
  return createRouter({
    history: createWebHistory(),
    routes: [
      {
        path: '/',
        name: 'home',
        component: { template: '<div>Home</div>' },
      },
    ],
  });
}

/**
 * Mount a Vue component with Pinia and Router support
 */
export function mountWithPlugins<T extends Component>(
  component: T,
  options?: MountingOptions<any>
) {
  const pinia = createTestPinia();
  const router = createTestRouter();

  return mount(component, {
    global: {
      plugins: [pinia, router],
    },
    ...options,
  });
}

/**
 * Wait for Vue to update the DOM
 */
export function flushPromises() {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

/**
 * Wait for a specific condition to be true
 */
export async function waitFor(
  condition: () => boolean,
  timeout = 1000,
  interval = 10
): Promise<void> {
  const startTime = Date.now();
  while (!condition()) {
    if (Date.now() - startTime > timeout) {
      throw new Error('Timeout waiting for condition');
    }
    await new Promise((resolve) => setTimeout(resolve, interval));
  }
}


