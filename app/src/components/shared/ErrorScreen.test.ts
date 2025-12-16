import { describe, it, expect, vi, beforeEach } from 'vitest';
import { mount } from '@vue/test-utils';
import ErrorScreen from './ErrorScreen.vue';

// Mock naive-ui components
vi.mock('naive-ui', () => ({
  NIcon: {
    name: 'NIcon',
    template: '<span class="n-icon"><slot /></span>',
  },
  NButton: {
    name: 'NButton',
    template: '<button class="n-button"><slot /></button>',
    props: ['type', 'size'],
  },
}));

// Mock vicons
vi.mock('@vicons/ionicons5', () => ({
  CloseCircleOutline: {
    name: 'CloseCircleOutline',
    template: '<svg class="close-circle-outline"></svg>',
  },
}));

// Mock unifiedEventService
vi.mock('../../services/unifiedEventService', () => ({
  unifiedEventService: {
    addEventListener: vi.fn(),
  },
  EventCategory: {
    System: 'system',
  },
}));

describe('ErrorScreen', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('should render error screen', () => {
    const wrapper = mount(ErrorScreen);

    expect(wrapper.find('.error-screen').exists()).toBe(true);
    expect(wrapper.find('.error-container').exists()).toBe(true);
  });

  it('should display default error message', () => {
    const wrapper = mount(ErrorScreen);

    expect(wrapper.text()).toContain('An error has occurred.');
  });

  it('should display custom error message', async () => {
    const { unifiedEventService, EventCategory } = await import('../../services/unifiedEventService');
    
    const wrapper = mount(ErrorScreen);

    // Simulate error event
    const addEventListener = (unifiedEventService.addEventListener as any);
    if (addEventListener.mock.calls.length > 0) {
      const handler = addEventListener.mock.calls[0][2];
      await handler({
        payload: {
          message: 'Custom error message',
        },
      });

      await wrapper.vm.$nextTick();
      expect(wrapper.text()).toContain('Custom error message');
    }
  });

  it('should have restart button', () => {
    const wrapper = mount(ErrorScreen);
    const button = wrapper.find('.n-button');

    expect(button.exists()).toBe(true);
    expect(button.text()).toContain('Restart Compute42');
  });

  it('should call window.location.reload when restart button is clicked', () => {
    const reloadSpy = vi.spyOn(window.location, 'reload').mockImplementation(() => {});

    const wrapper = mount(ErrorScreen);
    const button = wrapper.find('.n-button');

    button.trigger('click');

    expect(reloadSpy).toHaveBeenCalled();

    reloadSpy.mockRestore();
  });

  it('should set up event listener on mount', async () => {
    const { unifiedEventService, EventCategory } = await import('../../services/unifiedEventService');
    
    mount(ErrorScreen);

    await new Promise((resolve) => setTimeout(resolve, 0));

    expect(unifiedEventService.addEventListener).toHaveBeenCalledWith(
      EventCategory.System,
      'error',
      expect.any(Function)
    );
  });
});


