import { beforeEach, vi } from 'vitest';
import { createPinia, setActivePinia } from 'pinia';
import { config } from '@vue/test-utils';

// Create a Pinia instance for testing
const pinia = createPinia();

// Set up Pinia before each test
beforeEach(() => {
  setActivePinia(pinia);
});

// Configure Vue Test Utils to use Pinia
config.global.plugins = [pinia];

// Mock Tauri APIs globally
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

vi.mock('@tauri-apps/plugin-log', () => ({
  trace: vi.fn(),
  debug: vi.fn(),
  info: vi.fn(),
  warn: vi.fn(),
  error: vi.fn(),
}));

// Mock Vue Router
vi.mock('vue-router', () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    go: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  useRoute: () => ({
    path: '/',
    params: {},
    query: {},
    hash: '',
    fullPath: '/',
    matched: [],
    meta: {},
    name: null,
  }),
}));

