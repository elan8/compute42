import { describe, it, expect, beforeEach, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { defineComponent } from 'vue';
import { useLspStatus } from './useLspStatus';
import { appEventBus } from '../../services/eventBus';

describe('useLspStatus', () => {
  beforeEach(() => {
    // Clear any existing listeners
    vi.clearAllMocks();
  });

  it('should initialize with null status', () => {
    const TestComponent = defineComponent({
      setup() {
        const { status } = useLspStatus();
        return { status };
      },
      template: '<div>{{ status }}</div>',
    });

    const wrapper = mount(TestComponent);
    expect(wrapper.vm.status).toBeNull();
  });

  it('should update status when event is emitted', async () => {
    const TestComponent = defineComponent({
      setup() {
        const { status } = useLspStatus();
        return { status };
      },
      template: '<div>{{ status?.status }}</div>',
    });

    const wrapper = mount(TestComponent);

    appEventBus.emit('lsp:status', {
      status: 'ready',
      message: 'LSP is ready',
      error: null,
      project_path: '/path/to/project',
    });

    await wrapper.vm.$nextTick();

    expect(wrapper.vm.status).toEqual({
      status: 'ready',
      message: 'LSP is ready',
      error: null,
      project_path: '/path/to/project',
    });
  });

  it('should cleanup listener on unmount', () => {
    const offSpy = vi.spyOn(appEventBus, 'off');
    const TestComponent = defineComponent({
      setup() {
        const { status } = useLspStatus();
        return { status };
      },
      template: '<div>{{ status }}</div>',
    });

    const wrapper = mount(TestComponent);
    wrapper.unmount();

    expect(offSpy).toHaveBeenCalled();
  });
});


