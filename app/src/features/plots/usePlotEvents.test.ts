import { describe, it, expect, beforeEach, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import { defineComponent } from 'vue';
import { usePlotEvents } from './usePlotEvents';
import { appEventBus } from '../../services/eventBus';

describe('usePlotEvents', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should initialize with empty plotIds', () => {
    const TestComponent = defineComponent({
      setup() {
        const { plotIds } = usePlotEvents();
        return { plotIds };
      },
      template: '<div>{{ plotIds.length }}</div>',
    });

    const wrapper = mount(TestComponent);
    expect(wrapper.vm.plotIds).toEqual([]);
  });

  it('should update plotIds when event is emitted', async () => {
    const TestComponent = defineComponent({
      setup() {
        const { plotIds } = usePlotEvents();
        return { plotIds };
      },
      template: '<div>{{ plotIds.join(",") }}</div>',
    });

    const wrapper = mount(TestComponent);

    appEventBus.emit('plot:navigator-update', ['plot1', 'plot2', 'plot3']);

    await wrapper.vm.$nextTick();

    expect(wrapper.vm.plotIds).toEqual(['plot1', 'plot2', 'plot3']);
  });

  it('should cleanup listener on unmount', () => {
    const offSpy = vi.spyOn(appEventBus, 'off');
    const TestComponent = defineComponent({
      setup() {
        const { plotIds } = usePlotEvents();
        return { plotIds };
      },
      template: '<div>{{ plotIds.length }}</div>',
    });

    const wrapper = mount(TestComponent);
    wrapper.unmount();

    expect(offSpy).toHaveBeenCalled();
  });
});


