import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount } from '@vue/test-utils';
import NotificationToast from './NotificationToast.vue';

// Mock naive-ui components
vi.mock('naive-ui', () => ({
  NIcon: {
    name: 'NIcon',
    template: '<span class="n-icon"><slot /></span>',
  },
}));

// Mock vicons
vi.mock('@vicons/ionicons5', () => ({
  CloseCircleOutline: {
    name: 'CloseCircleOutline',
    template: '<svg class="close-circle-outline"></svg>',
  },
  WarningOutline: {
    name: 'WarningOutline',
    template: '<svg class="warning-outline"></svg>',
  },
  CheckmarkCircleOutline: {
    name: 'CheckmarkCircleOutline',
    template: '<svg class="checkmark-circle-outline"></svg>',
  },
  InformationCircleOutline: {
    name: 'InformationCircleOutline',
    template: '<svg class="information-circle-outline"></svg>',
  },
  CloseOutline: {
    name: 'CloseOutline',
    template: '<svg class="close-outline"></svg>',
  },
}));

describe('NotificationToast', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('should render when show is true', () => {
    const wrapper = mount(NotificationToast, {
      props: {
        title: 'Test Title',
        message: 'Test Message',
        show: true,
      },
    });

    expect(wrapper.find('.notification-toast').exists()).toBe(true);
  });

  it('should not render when show is false', () => {
    const wrapper = mount(NotificationToast, {
      props: {
        title: 'Test Title',
        message: 'Test Message',
        show: false,
      },
    });

    expect(wrapper.find('.notification-toast').exists()).toBe(false);
  });

  it('should display title and message', () => {
    const wrapper = mount(NotificationToast, {
      props: {
        title: 'Test Title',
        message: 'Test Message',
        show: true,
      },
    });

    expect(wrapper.text()).toContain('Test Title');
    expect(wrapper.text()).toContain('Test Message');
  });

  it('should show info icon by default', () => {
    const wrapper = mount(NotificationToast, {
      props: {
        title: 'Test Title',
        message: 'Test Message',
        show: true,
      },
    });

    expect(wrapper.find('.info-icon').exists()).toBe(true);
  });

  it('should show error icon for error type', () => {
    const wrapper = mount(NotificationToast, {
      props: {
        title: 'Test Title',
        message: 'Test Message',
        type: 'error',
        show: true,
      },
    });

    expect(wrapper.find('.error-icon').exists()).toBe(true);
  });

  it('should show warning icon for warning type', () => {
    const wrapper = mount(NotificationToast, {
      props: {
        title: 'Test Title',
        message: 'Test Message',
        type: 'warning',
        show: true,
      },
    });

    expect(wrapper.find('.warning-icon').exists()).toBe(true);
  });

  it('should show success icon for success type', () => {
    const wrapper = mount(NotificationToast, {
      props: {
        title: 'Test Title',
        message: 'Test Message',
        type: 'success',
        show: true,
      },
    });

    expect(wrapper.find('.success-icon').exists()).toBe(true);
  });

  it('should auto-close after duration', async () => {
    const wrapper = mount(NotificationToast, {
      props: {
        title: 'Test Title',
        message: 'Test Message',
        show: true,
        duration: 5000,
      },
    });

    expect(wrapper.find('.notification-toast').exists()).toBe(true);

    // Fast-forward time
    vi.advanceTimersByTime(5000);

    await wrapper.vm.$nextTick();

    // The component should have closed
    expect(wrapper.find('.notification-toast').exists()).toBe(false);
  });

  it('should emit close event when closed', async () => {
    const wrapper = mount(NotificationToast, {
      props: {
        title: 'Test Title',
        message: 'Test Message',
        show: true,
      },
    });

    const closeButton = wrapper.find('.notification-close');
    closeButton.trigger('click');

    await wrapper.vm.$nextTick();

    expect(wrapper.emitted('close')).toBeTruthy();
  });

  it('should not auto-close if duration is 0', async () => {
    const wrapper = mount(NotificationToast, {
      props: {
        title: 'Test Title',
        message: 'Test Message',
        show: true,
        duration: 0,
      },
    });

    vi.advanceTimersByTime(10000);

    await wrapper.vm.$nextTick();

    // Should still be visible
    expect(wrapper.find('.notification-toast').exists()).toBe(true);
  });
});


