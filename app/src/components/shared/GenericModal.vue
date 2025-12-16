<template>
  <n-modal
    :show="show"
    @update:show="onUpdateShow"
    preset="card"
    :style="modalStyle"
    :mask-closable="maskClosable"
    :closable="closable"
    :show-close="showCloseButton"
    :auto-focus="autoFocus"
    :trap-focus="trapFocus"
    :block-scroll="blockScroll"
    :transform-origin="transformOrigin"
    :on-esc="onEsc"
    :on-close="onClose"
  >
    <!-- Header slot -->
    <template #header v-if="$slots.header">
      <slot name="header" />
    </template>

    <!-- Content slot -->
    <slot />

    <!-- Footer slot -->
    <template #footer v-if="$slots.footer">
      <slot name="footer" />
    </template>
  </n-modal>
</template>

<script setup>
import { computed } from 'vue';
import { NModal } from 'naive-ui';

const props = defineProps({
  show: {
    type: Boolean,
    default: false,
  },
  width: {
    type: [String, Number],
    default: '500px',
  },
  height: {
    type: [String, Number],
    default: 'auto',
  },
  maskClosable: {
    type: Boolean,
    default: true,
  },
  closable: {
    type: Boolean,
    default: true,
  },
  showCloseButton: {
    type: Boolean,
    default: true,
  },
  autoFocus: {
    type: Boolean,
    default: true,
  },
  trapFocus: {
    type: Boolean,
    default: true,
  },
  blockScroll: {
    type: Boolean,
    default: true,
  },
  transformOrigin: {
    type: String,
    default: 'center',
  },
});

const emit = defineEmits(['update:show', 'close']);

const modalStyle = computed(() => ({
  width: typeof props.width === 'number' ? `${props.width}px` : props.width,
  height: typeof props.height === 'number' ? `${props.height}px` : props.height,
  maxWidth: '90vw',
  maxHeight: '90vh',
}));

const onUpdateShow = (val) => {
  emit('update:show', val);
  if (!val) emit('close');
};

const onEsc = () => {
  emit('update:show', false);
  emit('close');
};

const onClose = () => {
  emit('update:show', false);
  emit('close');
};
</script>

<style scoped>
/* Custom theme overrides to match the original GenericModal styling */
:deep(.n-modal) {
  --n-color: linear-gradient(135deg, #252526 0%, #2d2d30 100%);
  --n-border: 1px solid #333;
  --n-border-radius: 16px;
  --n-box-shadow: 0 20px 40px rgba(0, 0, 0, 0.5);
  --n-text-color: #e0e0e0;
  --n-title-text-color: #e0e0e0;
  --n-action-text-color: #e0e0e0;
  --n-close-color: #999;
  --n-close-color-hover: #ccc;
  --n-close-color-pressed: #fff;
}

:deep(.n-modal-body) {
  padding: 2rem;
  text-align: center;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

:deep(.n-modal-header) {
  margin-bottom: 0;
  padding-bottom: 0;
  border-bottom: none;
}

:deep(.n-modal-content) {
  flex: 1;
  padding: 0;
}

:deep(.n-modal-footer) {
  margin-top: 0;
  padding-top: 0;
  border-top: none;
}

/* Backdrop styling to match original */
:deep(.n-modal-mask) {
  background: rgba(30, 30, 30, 0.25);
  backdrop-filter: blur(4px);
}
</style>
