<template>
  <Transition name="notification">
    <div v-if="isVisible" class="notification-toast" :class="typeClass">
      <div class="notification-icon">
        <n-icon v-if="type === 'error'" class="error-icon">
          <CloseCircleOutline />
        </n-icon>
        <n-icon v-else-if="type === 'warning'" class="warning-icon">
          <WarningOutline />
        </n-icon>
        <n-icon v-else-if="type === 'success'" class="success-icon">
          <CheckmarkCircleOutline />
        </n-icon>
        <n-icon v-else class="info-icon">
          <InformationCircleOutline />
        </n-icon>
      </div>
      <div class="notification-content">
        <div class="notification-title">{{ title }}</div>
        <div class="notification-message">{{ message }}</div>
      </div>
      <button @click="close" class="notification-close">
        <n-icon><CloseOutline /></n-icon>
      </button>
    </div>
  </Transition>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from 'vue';
import { NIcon } from 'naive-ui';
import {
  CloseCircleOutline,
  WarningOutline,
  CheckmarkCircleOutline,
  InformationCircleOutline,
  CloseOutline,
} from '@vicons/ionicons5';

interface Props {
  title: string;
  message: string;
  type?: 'info' | 'success' | 'warning' | 'error';
  duration?: number;
  show?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  type: 'info',
  duration: 5000,
  show: true,
});

const emit = defineEmits<{
  close: [];
}>();

const isVisible = ref(props.show);

const typeClass = computed(() => {
  return `notification-${props.type}`;
});

const close = () => {
  isVisible.value = false;
  emit('close');
};

onMounted(() => {
  if (props.duration > 0) {
    setTimeout(() => {
      close();
    }, props.duration);
  }
});
</script>

<style scoped>
.notification-toast {
  position: fixed;
  top: 20px;
  right: 20px;
  background: #2d2d2d;
  border: 1px solid #444;
  border-radius: 8px;
  padding: 1rem;
  min-width: 300px;
  max-width: 400px;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
  z-index: 10000;
  display: flex;
  align-items: flex-start;
  gap: 0.75rem;
}

.notification-icon {
  flex-shrink: 0;
  margin-top: 0.125rem;
}

.notification-content {
  flex: 1;
  min-width: 0;
}

.notification-title {
  font-weight: 600;
  color: #ffffff;
  margin-bottom: 0.25rem;
  font-size: 0.9rem;
}

.notification-message {
  color: #b0b0b0;
  font-size: 0.85rem;
  line-height: 1.4;
}

.notification-close {
  background: none;
  border: none;
  color: #888;
  cursor: pointer;
  padding: 0.25rem;
  border-radius: 4px;
  flex-shrink: 0;
  margin-top: 0.125rem;
}

.notification-close:hover {
  background: rgba(255, 255, 255, 0.1);
  color: #ccc;
}

/* Type-specific styles */
.notification-info {
  border-left: 4px solid #2196f3;
}

.notification-success {
  border-left: 4px solid #4caf50;
}

.notification-warning {
  border-left: 4px solid #ff9800;
}

.notification-error {
  border-left: 4px solid #f44336;
}

.info-icon {
  color: #2196f3;
}

.success-icon {
  color: #4caf50;
}

.warning-icon {
  color: #ff9800;
}

.error-icon {
  color: #f44336;
}

/* Transition animations */
.notification-enter-active,
.notification-leave-active {
  transition: all 0.3s ease;
}

.notification-enter-from {
  opacity: 0;
  transform: translateX(100%);
}

.notification-leave-to {
  opacity: 0;
  transform: translateX(100%);
}
</style>
