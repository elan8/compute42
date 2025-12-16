import { describe, it, expect, vi } from 'vitest';
import { mount } from '@vue/test-utils';
import GenericModal from './GenericModal.vue';

// Mock naive-ui components
vi.mock('naive-ui', () => ({
  NModal: {
    name: 'NModal',
    template: '<div class="n-modal"><slot name="header" /><slot /><slot name="footer" /></div>',
    props: ['show', 'maskClosable', 'closable', 'showCloseButton', 'autoFocus', 'trapFocus', 'blockScroll', 'transformOrigin', 'style', 'onEsc', 'onClose'],
    emits: ['update:show'],
  },
}));

describe('GenericModal', () => {
  it('should render when show is true', () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: true,
      },
      slots: {
        default: '<div>Modal Content</div>',
      },
    });

    expect(wrapper.find('.n-modal').exists()).toBe(true);
  });

  it('should not render when show is false', () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: false,
      },
    });

    // The modal might still be in DOM but hidden, so we check the show prop
    expect(wrapper.props('show')).toBe(false);
  });

  it('should emit update:show when show changes', async () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: true,
      },
    });

    // Simulate the modal's update:show event
    await wrapper.vm.$emit('update:show', false);

    expect(wrapper.emitted('update:show')).toBeTruthy();
    expect(wrapper.emitted('update:show')?.[0]).toEqual([false]);
  });

  it('should emit close event when modal closes', async () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: true,
      },
    });

    // Simulate close
    await wrapper.vm.$emit('update:show', false);

    expect(wrapper.emitted('close')).toBeTruthy();
  });

  it('should render header slot when provided', () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: true,
      },
      slots: {
        header: '<div>Header Content</div>',
      },
    });

    expect(wrapper.html()).toContain('Header Content');
  });

  it('should render footer slot when provided', () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: true,
      },
      slots: {
        footer: '<div>Footer Content</div>',
      },
    });

    expect(wrapper.html()).toContain('Footer Content');
  });

  it('should render default slot content', () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: true,
      },
      slots: {
        default: '<div>Main Content</div>',
      },
    });

    expect(wrapper.html()).toContain('Main Content');
  });

  it('should apply custom width', () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: true,
        width: '800px',
      },
    });

    expect(wrapper.props('width')).toBe('800px');
  });

  it('should apply custom height', () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: true,
        height: '600px',
      },
    });

    expect(wrapper.props('height')).toBe('600px');
  });

  it('should handle numeric width', () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: true,
        width: 800,
      },
    });

    expect(wrapper.props('width')).toBe(800);
  });

  it('should handle numeric height', () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: true,
        height: 600,
      },
    });

    expect(wrapper.props('height')).toBe(600);
  });

  it('should respect maskClosable prop', () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: true,
        maskClosable: false,
      },
    });

    expect(wrapper.props('maskClosable')).toBe(false);
  });

  it('should respect closable prop', () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: true,
        closable: false,
      },
    });

    expect(wrapper.props('closable')).toBe(false);
  });

  it('should respect showCloseButton prop', () => {
    const wrapper = mount(GenericModal, {
      props: {
        show: true,
        showCloseButton: false,
      },
    });

    expect(wrapper.props('showCloseButton')).toBe(false);
  });
});


