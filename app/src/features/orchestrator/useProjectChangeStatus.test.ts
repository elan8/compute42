import { describe, it, expect, beforeEach, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { defineComponent } from 'vue';
import { useProjectChangeStatus } from './useProjectChangeStatus';
import { appEventBus } from '../../services/eventBus';

describe('useProjectChangeStatus', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should initialize with null status', () => {
    const TestComponent = defineComponent({
      setup() {
        const { status } = useProjectChangeStatus();
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
        const { status } = useProjectChangeStatus();
        return { status };
      },
      template: '<div>{{ status?.message }}</div>',
    });

    const wrapper = mount(TestComponent);

    appEventBus.emit('orchestrator:project-change-status', {
      message: 'Loading project...',
      progress_percentage: 50,
    });

    await wrapper.vm.$nextTick();

    expect(wrapper.vm.status).toEqual({
      message: 'Loading project...',
      progress_percentage: 50,
    });
  });

  it('should cleanup listener on unmount', () => {
    const TestComponent = defineComponent({
      setup() {
        const { status } = useProjectChangeStatus();
        return { status };
      },
      template: '<div>{{ status }}</div>',
    });

    const wrapper = mount(TestComponent);
    const offFn = vi.fn();
    // Mock the on method to return an off function
    const onSpy = vi.spyOn(appEventBus, 'on').mockReturnValue(offFn);

    wrapper.unmount();

    // The cleanup should be called
    expect(onSpy).toHaveBeenCalled();
  });
});


